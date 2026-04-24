//! Estimation session read endpoint (Phase 2C).
//!
//! The generate-estimate *action* lives under `chat` (AIP-136 custom verb on
//! the chat session resource). This module only exposes the read view.

use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::{routing::get, Json, Router};

use efofx_auth::middleware::either_auth;
use efofx_domain::{EstimationOutput, EstimationSessionId, EstimationStatus};
use efofx_openapi::ApiError;
use efofx_storage::TenantContext;

use serde::Serialize;
use utoipa::ToSchema;

use crate::AppState;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EstimationSessionResponse {
    pub session_id: String,
    pub status: EstimationStatus,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<EstimationOutput>,
}

/// Read a persisted estimation session by id. Returns the structured
/// output once `status = completed`. FastAPI parity: `EstimationService.get_estimation`.
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
    ),
)]
pub async fn get_estimation(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
    Path(session_id): Path<String>,
) -> Response {
    let id = EstimationSessionId(session_id);
    match state.estimation.get_session(&ctx, &id).await {
        Ok(session) => {
            // The Rust write path only persists a session once `generate_from_chat`
            // succeeded, so the session always carries an output in practice. The
            // response wraps it in the FastAPI-compatible envelope so legacy
            // consumers see `{session_id, status, message, result?}`.
            let message = match session.status {
                EstimationStatus::Completed => "Estimation completed.".to_string(),
                EstimationStatus::Expired => "Estimation session has expired.".to_string(),
                EstimationStatus::Cancelled => "Estimation was cancelled.".to_string(),
                EstimationStatus::Initiated => "Estimation is initiated.".to_string(),
                EstimationStatus::InProgress => "Estimation is in progress.".to_string(),
            };
            // `EstimationOutput` is not persisted alongside the session in the
            // current schema — FastAPI historically returned `result: null` and
            // relied on narrative replay for the rendered estimate. Rust keeps
            // that contract until a dedicated results collection lands.
            Json(EstimationSessionResponse {
                session_id: session.id.0,
                status: session.status,
                message,
                result: None,
            })
            .into_response()
        }
        Err(e) => e.into_response(),
    }
}

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/estimates/{session_id}", get(get_estimation))
        .route_layer(from_fn_with_state(state.auth.clone(), either_auth))
}
