//! Estimation session read endpoint (Phase 2C).
//!
//! The generate-estimate *action* lives under `chat` (AIP-136 custom verb on
//! the chat session resource). This module only exposes the read view.

use std::sync::Arc;

use axum::{response::IntoResponse, routing::get, Router};
use serde::Serialize;
use utoipa::ToSchema;

use efofx_domain::{EstimationOutput, EstimationStatus};
use efofx_openapi::ApiError;

use crate::AppState;

use super::not_implemented;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EstimationSessionResponse {
    pub session_id: String,
    pub status: EstimationStatus,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<EstimationOutput>,
}

/// Read a persisted estimation session by id. Returns the structured
/// output once `status = completed`.
#[utoipa::path(
    get,
    path = "/v1/estimates/{session_id}",
    tag = "estimation",
    params(
        ("session_id" = String, Path, description = "Estimation session identifier")
    ),
    security(("supabase_jwt" = []), ("widget_api_key" = [])),
    responses(
        (status = 200, description = "Estimation session", body = EstimationSessionResponse),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 404, description = "Session not found", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn get_estimation() -> impl IntoResponse {
    not_implemented("GET /v1/estimates/{id}")
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/v1/estimates/{session_id}", get(get_estimation))
}
