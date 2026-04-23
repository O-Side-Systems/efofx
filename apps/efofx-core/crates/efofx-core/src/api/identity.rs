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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<serde_json::Value>,
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

/// Update tenant-owned settings. Does not touch email or password (those
/// live in Supabase).
#[utoipa::path(
    patch,
    path = "/v1/me",
    tag = "identity",
    request_body = UpdateTenantRequest,
    security(("supabase_jwt" = [])),
    responses(
        (status = 200, description = "Updated profile", body = Tenant),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 404, description = "Tenant record missing", body = ApiError),
    ),
)]
pub async fn patch_me(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
    Json(body): Json<UpdateTenantRequest>,
) -> Response {
    let patch = TenantProfilePatch {
        company_name: body.company_name,
    };
    match state.tenants.update_profile(&ctx, &patch).await {
        Ok(tenant) => Json(tenant).into_response(),
        Err(e) => storage_into_response(e),
    }
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
