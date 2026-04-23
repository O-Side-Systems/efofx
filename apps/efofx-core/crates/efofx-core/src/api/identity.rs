//! Identity & tenant-profile endpoints (Phase 2A).
//!
//! Supabase owns registration, login, verify, and refresh. This module
//! owns the authenticated-principal surface (`/v1/me`), BYOK storage,
//! and widget-API-key rotation.

use std::sync::Arc;

use axum::{
    response::IntoResponse,
    routing::{get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use efofx_domain::Tenant;
use efofx_openapi::ApiError;

use crate::AppState;

use super::not_implemented;

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

/// Fetch the authenticated principal's merged profile.
#[utoipa::path(
    get,
    path = "/v1/me",
    tag = "identity",
    security(("supabase_jwt" = []), ("widget_api_key" = [])),
    responses(
        (status = 200, description = "Tenant profile", body = Tenant),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn get_me() -> impl IntoResponse {
    not_implemented("GET /v1/me")
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
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn patch_me() -> impl IntoResponse {
    not_implemented("PATCH /v1/me")
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
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn put_openai_key() -> impl IntoResponse {
    not_implemented("PUT /v1/me/openai-key")
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
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn get_openai_key_status() -> impl IntoResponse {
    not_implemented("GET /v1/me/openai-key/status")
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
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn delete_openai_key() -> impl IntoResponse {
    not_implemented("DELETE /v1/me/openai-key")
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
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn rotate_api_key() -> impl IntoResponse {
    not_implemented("POST /v1/me/api-keys:rotate")
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/me", get(get_me).patch(patch_me))
        .route(
            "/v1/me/openai-key",
            put(put_openai_key).delete(delete_openai_key),
        )
        .route("/v1/me/openai-key/status", get(get_openai_key_status))
        .route("/v1/me/api-keys:rotate", post(rotate_api_key))
}
