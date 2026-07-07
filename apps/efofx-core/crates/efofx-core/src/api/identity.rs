//! Identity & tenant-profile endpoints (Phase 2A).
//!
//! Supabase owns registration, login, verify, and refresh. This module
//! owns the authenticated-principal surface (`/v1/me`), BYOK storage,
//! and widget-API-key rotation.

use std::sync::Arc;

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use efofx_auth::middleware::{either_auth, supabase_jwt};
use efofx_crypto::mask_openai_key;
use efofx_domain::Tenant;
use efofx_openapi::{ApiError, ErrorCode};
use efofx_storage::tenants::TenantProfilePatch;
use efofx_storage::TenantContext;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::services::ByokError;
use crate::AppState;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateTenantRequest {
    #[serde(default)]
    pub company_name: Option<String>,
    #[serde(default)]
    pub settings: Option<UpdateTenantSettings>,
}

/// Settings subfields settable over `PATCH /v1/me`. Each field is
/// replace-if-present — an omitted field keeps its stored value, so a
/// partial patch never clobbers sibling settings.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdateTenantSettings {
    #[serde(default)]
    pub branding: Option<efofx_domain::BrandingConfig>,
    /// Exact-match origins the tenant's widget may embed from
    /// (`scheme://host[:port]`, no path). Replaces the stored list.
    #[serde(default)]
    pub allowed_origins: Option<Vec<String>>,
    #[serde(default)]
    pub routing: Option<efofx_storage::tenants::RoutingConfig>,
}

const MAX_ALLOWED_ORIGINS: usize = 20;
const MAX_ORIGIN_LEN: usize = 255;
const MAX_URL_TEMPLATE_LEN: usize = 2048;
const MAX_TAG_OVERRIDES: usize = 100;
const MAX_TAG_LEN: usize = 100;
const MAX_COMPANY_NAME_LEN: usize = 200;

/// Reject malformed settings before they reach storage. Origins feed the
/// per-tenant CORS allow-list and the routing template is echoed to
/// partners, so both are validated for shape, scheme, and size.
fn validate_settings(s: &UpdateTenantSettings) -> Result<(), String> {
    if let Some(ref origins) = s.allowed_origins {
        if origins.len() > MAX_ALLOWED_ORIGINS {
            return Err(format!(
                "settings.allowed_origins: at most {MAX_ALLOWED_ORIGINS} origins"
            ));
        }
        for o in origins {
            validate_origin(o)?;
        }
    }
    if let Some(ref routing) = s.routing {
        if let Some(ref template) = routing.directory_url_template {
            if template.len() > MAX_URL_TEMPLATE_LEN {
                return Err(format!(
                    "settings.routing.directory_url_template: at most {MAX_URL_TEMPLATE_LEN} chars"
                ));
            }
            if !template.starts_with("https://") && !template.starts_with("http://") {
                return Err(
                    "settings.routing.directory_url_template: must start with http(s)://".into(),
                );
            }
        }
        if routing.tag_overrides.len() > MAX_TAG_OVERRIDES {
            return Err(format!(
                "settings.routing.tag_overrides: at most {MAX_TAG_OVERRIDES} entries"
            ));
        }
        for (k, v) in &routing.tag_overrides {
            if k.is_empty() || k.len() > MAX_TAG_LEN || v.is_empty() || v.len() > MAX_TAG_LEN {
                return Err(format!(
                    "settings.routing.tag_overrides: keys and values must be 1-{MAX_TAG_LEN} chars"
                ));
            }
        }
        if let Some([mid, high]) = routing.cost_tier_breakpoints {
            if mid >= high {
                return Err(
                    "settings.routing.cost_tier_breakpoints: mid bound must be below high bound"
                        .into(),
                );
            }
        }
    }
    if let Some(ref branding) = s.branding {
        if let Some(ref url) = branding.logo_url {
            if url.len() > MAX_URL_TEMPLATE_LEN
                || (!url.starts_with("https://") && !url.starts_with("http://"))
            {
                return Err("settings.branding.logo_url: must be an http(s) URL".into());
            }
        }
        if branding.welcome_message.len() > 1000
            || branding.button_text.len() > 100
            || branding.company_name.len() > MAX_COMPANY_NAME_LEN
        {
            return Err("settings.branding: text fields exceed length limits".into());
        }
    }
    Ok(())
}

/// An allowed origin must look like `scheme://host[:port]` — exact-match
/// CORS means anything with a path, query, wildcard, or trailing slash
/// would never match a real `Origin` header and only mask config errors.
fn validate_origin(origin: &str) -> Result<(), String> {
    let err = || format!("settings.allowed_origins: '{origin}' is not a valid http(s) origin");
    if origin.len() > MAX_ORIGIN_LEN {
        return Err(err());
    }
    let rest = origin
        .strip_prefix("https://")
        .or_else(|| origin.strip_prefix("http://"))
        .ok_or_else(err)?;
    if rest.is_empty() || rest.contains(['/', '?', '#', '*', ' ', '@']) {
        return Err(err());
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct StoreOpenAiKeyRequest {
    /// Raw OpenAI API key. Validated against OpenAI `models.list()` and
    /// encrypted with a per-tenant HKDF-derived Fernet key before storage.
    pub openai_key: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct StoreOpenAiKeyResponse {
    /// Last 6 chars for masked display, e.g. `"sk-...abc123"`.
    pub masked_key: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OpenAiKeyStatusResponse {
    pub has_key: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub masked_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RotateApiKeyResponse {
    /// Plaintext key returned once and never again. Store immediately.
    pub api_key: String,
    /// Last 6 chars for masked display.
    pub masked_key: String,
}

/// Convert a storage error into a user-safe HTTP response. NotFound on
/// `/v1/me` means the tenant record went missing between auth and the
/// read — surface as `404` so the caller can distinguish from unauth.
fn storage_into_response(err: efofx_storage::StorageError) -> Response {
    use efofx_storage::StorageError::*;
    match err {
        NotFound => (
            StatusCode::NOT_FOUND,
            Json(ApiError::single(
                "tenant.not_found",
                "Tenant record missing for authenticated principal",
            )),
        )
            .into_response(),
        ApiKeyInvalid => (
            StatusCode::UNAUTHORIZED,
            Json(ApiError::single(
                ErrorCode::AuthApiKeyInvalid.as_str(),
                ErrorCode::AuthApiKeyInvalid.default_message(),
            )),
        )
            .into_response(),
        _ => {
            tracing::error!(error = %err, "storage failure in /v1/me surface");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::single(
                    "common.internal",
                    "An internal error occurred",
                )),
            )
                .into_response()
        }
    }
}

/// Fetch the authenticated principal's merged profile.
#[utoipa::path(
    get,
    path = "/v1/me",
    tag = "identity",
    security(("supabase_jwt" = []), ("widget_api_key" = [])),
    responses(
        (status = 200, description = "Tenant profile", body = Tenant),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 404, description = "Tenant record missing", body = ApiError),
    ),
)]
pub async fn get_me(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
) -> Response {
    match state.tenants.get(&ctx).await {
        Ok(tenant) => Json(tenant).into_response(),
        Err(e) => storage_into_response(e),
    }
}

/// Update tenant-owned profile fields and settings (branding,
/// allowed_origins, routing). Does not touch email or password (those
/// live in Supabase).
#[utoipa::path(
    patch,
    path = "/v1/me",
    tag = "identity",
    request_body = UpdateTenantRequest,
    security(("supabase_jwt" = [])),
    responses(
        (status = 200, description = "Updated profile", body = Tenant),
        (status = 400, description = "Invalid settings payload", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 404, description = "Tenant record missing", body = ApiError),
    ),
)]
pub async fn patch_me(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
    Json(body): Json<UpdateTenantRequest>,
) -> Response {
    if let Some(ref cn) = body.company_name {
        if cn.is_empty() || cn.len() > MAX_COMPANY_NAME_LEN {
            return validation_failed(&format!(
                "company_name: must be 1-{MAX_COMPANY_NAME_LEN} chars"
            ));
        }
    }
    let settings = body.settings.unwrap_or_default();
    if let Err(msg) = validate_settings(&settings) {
        return validation_failed(&msg);
    }
    let patch = TenantProfilePatch {
        company_name: body.company_name,
        branding: settings.branding,
        allowed_origins: settings.allowed_origins.clone(),
        routing: settings.routing,
    };
    match state.tenants.update_profile(&ctx, &patch).await {
        Ok(tenant) => {
            // Newly-allowed origins take effect immediately. Removed
            // origins stay in the process-wide cache until restart — the
            // documented eviction gap in `tenant_cors.rs`.
            if let Some(origins) = settings.allowed_origins {
                state.origin_cache.insert_many(origins);
            }
            Json(tenant).into_response()
        }
        Err(e) => storage_into_response(e),
    }
}

fn validation_failed(message: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiError::single(
            ErrorCode::ValidationFailed.as_str(),
            message,
        )),
    )
        .into_response()
}

/// Store or rotate the tenant's BYOK OpenAI API key.
#[utoipa::path(
    put,
    path = "/v1/me/openai-key",
    tag = "identity",
    request_body = StoreOpenAiKeyRequest,
    security(("supabase_jwt" = [])),
    responses(
        (status = 200, description = "Key stored", body = StoreOpenAiKeyResponse),
        (status = 400, description = "Invalid OpenAI key", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 503, description = "OpenAI unavailable during validation", body = ApiError),
    ),
)]
pub async fn put_openai_key(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
    Json(body): Json<StoreOpenAiKeyRequest>,
) -> Response {
    match state.byok.store(&ctx, &body.openai_key).await {
        Ok(masked) => Json(StoreOpenAiKeyResponse {
            masked_key: masked,
            message: "OpenAI API key stored".to_string(),
        })
        .into_response(),
        Err(e) => e.into_response(),
    }
}

/// Return the BYOK key presence and masked display value.
#[utoipa::path(
    get,
    path = "/v1/me/openai-key/status",
    tag = "identity",
    security(("supabase_jwt" = []), ("widget_api_key" = [])),
    responses(
        (status = 200, description = "Key status", body = OpenAiKeyStatusResponse),
        (status = 401, description = "Authentication required", body = ApiError),
    ),
)]
pub async fn get_openai_key_status(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
) -> Response {
    match state.tenants.openai_key_status(&ctx).await {
        Ok(status) => {
            let masked_key = status.last6.map(|s| format!("sk-...{s}"));
            Json(OpenAiKeyStatusResponse {
                has_key: status.has_key,
                masked_key,
            })
            .into_response()
        }
        Err(e) => storage_into_response(e),
    }
}

/// Remove the stored BYOK OpenAI key.
#[utoipa::path(
    delete,
    path = "/v1/me/openai-key",
    tag = "identity",
    security(("supabase_jwt" = [])),
    responses(
        (status = 204, description = "Key removed"),
        (status = 401, description = "Authentication required", body = ApiError),
    ),
)]
pub async fn delete_openai_key(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
) -> Response {
    match state.byok.clear(&ctx).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(ByokError::Storage(e)) => storage_into_response(e),
        Err(e) => e.into_response(),
    }
}

/// Rotate the tenant's widget API key. The previous key is invalidated and
/// the new plaintext is returned exactly once — store it immediately.
#[utoipa::path(
    post,
    path = "/v1/me/api-keys:rotate",
    tag = "identity",
    security(("supabase_jwt" = [])),
    responses(
        (status = 200, description = "New API key issued", body = RotateApiKeyResponse),
        (status = 401, description = "Authentication required", body = ApiError),
    ),
)]
pub async fn rotate_api_key(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
) -> Response {
    let key = state.api_key_auth.generate(ctx.tenant_id());
    if let Err(e) = state
        .tenants
        .set_api_key(&ctx, &key.hmac_hex, &key.last6)
        .await
    {
        return storage_into_response(e);
    }
    // The masked view the dashboard shows — keep using the `sk-...` prefix
    // convention from the BYOK mask helper so UI code can reuse one
    // renderer.
    let masked_key = mask_openai_key(&key.raw);
    Json(RotateApiKeyResponse {
        api_key: key.raw,
        masked_key,
    })
    .into_response()
}

/// Assemble the `/v1/me*` router with its middleware stack.
///
/// - `GET /v1/me` and `GET /v1/me/openai-key/status` accept either
///   Supabase JWT or widget API key (same principal, different surface).
/// - All mutating endpoints require a Supabase JWT — widget keys can't
///   rotate themselves or manage BYOK.
pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let jwt_only = Router::new()
        .route("/v1/me", axum::routing::patch(patch_me))
        .route(
            "/v1/me/openai-key",
            put(put_openai_key).delete(delete_openai_key),
        )
        .route("/v1/me/api-keys:rotate", post(rotate_api_key))
        .route_layer(from_fn_with_state(state.auth.clone(), supabase_jwt));

    let either = Router::new()
        .route("/v1/me", get(get_me))
        .route("/v1/me/openai-key/status", get(get_openai_key_status))
        .route_layer(from_fn_with_state(state.auth.clone(), either_auth));

    jwt_only.merge(either)
}

#[cfg(test)]
mod tests {
    use super::*;
    use efofx_storage::tenants::RoutingConfig;

    #[test]
    fn valid_origins_pass() {
        for o in [
            "https://example.com",
            "http://localhost:5173",
            "https://sub.partner-site.co.uk:8443",
        ] {
            assert!(validate_origin(o).is_ok(), "{o} should be valid");
        }
    }

    #[test]
    fn malformed_origins_rejected() {
        for o in [
            "example.com",                // no scheme
            "ftp://example.com",          // wrong scheme
            "https://example.com/",       // trailing slash never matches Origin
            "https://example.com/widget", // path
            "https://*.example.com",      // wildcard
            "https://",                   // empty host
            "https://a b.com",            // space
            "https://user@example.com",   // userinfo
        ] {
            assert!(validate_origin(o).is_err(), "{o} should be rejected");
        }
    }

    #[test]
    fn settings_with_bad_template_scheme_rejected() {
        let s = UpdateTenantSettings {
            routing: Some(RoutingConfig {
                enabled: true,
                directory_url_template: Some("javascript:alert(1)".into()),
                ..RoutingConfig::default()
            }),
            ..UpdateTenantSettings::default()
        };
        assert!(validate_settings(&s).is_err());
    }

    #[test]
    fn settings_with_inverted_breakpoints_rejected() {
        let s = UpdateTenantSettings {
            routing: Some(RoutingConfig {
                enabled: true,
                cost_tier_breakpoints: Some([250_000, 25_000]),
                ..RoutingConfig::default()
            }),
            ..UpdateTenantSettings::default()
        };
        assert!(validate_settings(&s).is_err());
    }

    #[test]
    fn well_formed_settings_pass() {
        let s = UpdateTenantSettings {
            branding: None,
            allowed_origins: Some(vec!["https://contractor.example".into()]),
            routing: Some(RoutingConfig {
                enabled: true,
                directory_url_template: Some("https://partner.example/find?tags={tags}".into()),
                cost_tier_breakpoints: Some([25_000, 250_000]),
                ..RoutingConfig::default()
            }),
        };
        assert!(validate_settings(&s).is_ok());
    }

    #[test]
    fn empty_settings_pass() {
        assert!(validate_settings(&UpdateTenantSettings::default()).is_ok());
    }
}
