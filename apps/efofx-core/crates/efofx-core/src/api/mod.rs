//! HTTP API modules for the Rust platform core.
//!
//! Each module registers one bounded area: handlers + utoipa path
//! annotations + local request/response DTOs. Domain types live in
//! `efofx-domain` and are referenced here, not redefined.
//!
//! Phase 1.2: every planned endpoint exists as a stub returning 501
//! with a `common.not_implemented` error envelope. The `utoipa::path`
//! attributes are fully populated so `/openapi.json` describes the
//! complete Phase 2 surface today.

use std::sync::Arc;

use axum::{http::StatusCode, response::IntoResponse, Json, Router};
use efofx_openapi::ApiError;

use crate::AppState;

pub mod calibration;
pub mod chat;
pub mod estimation;
pub mod feedback;
pub mod identity;
pub mod integration;
pub mod leads;
pub mod widget;

/// Common stub response for endpoints that are scheduled for Phase 2.
/// Returns `501 Not Implemented` with the standard error envelope.
pub(crate) fn not_implemented(description: &'static str) -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiError::single(
            "common.not_implemented",
            format!("{description} is not yet implemented"),
        )),
    )
}

/// Assemble the `/v1/*` router. Identity routes are fully wired in Phase
/// 2A; the rest remain 501 stubs until their phase ships.
pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .merge(identity::routes(state.clone()))
        .merge(chat::routes(state.clone()))
        .merge(estimation::routes(state.clone()))
        .merge(widget::routes(state))
        .merge(leads::routes())
        .merge(feedback::routes())
        .merge(calibration::routes())
        .merge(integration::routes())
}
