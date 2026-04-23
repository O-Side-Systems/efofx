//! Partner / contractor-directory integrations (Phase 2F).

use std::sync::Arc;

use axum::{response::IntoResponse, routing::post, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use efofx_openapi::ApiError;

use crate::AppState;

use super::not_implemented;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ContractorMatchRequest {
    pub estimation_session_id: String,
    /// Optional override of the inferred project type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_type: Option<String>,
    /// Optional location hint (city, ZIP, or region label).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ContractorMatchResponse {
    /// Structured tags the partner directory can filter on.
    pub tags: Vec<String>,
    /// Region label, if resolved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// Estimated cost tier for the target project, e.g. `"mid"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_cost_tier: Option<String>,
    /// URL with partner filters applied, ready for redirect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory_url: Option<String>,
}

/// Produce structured routing data from a completed estimation session.
/// Partners call this to filter their directory by the session's project.
#[utoipa::path(
    post,
    path = "/v1/integration/contractor-match",
    tag = "integration",
    request_body = ContractorMatchRequest,
    security(("widget_api_key" = [])),
    responses(
        (status = 200, description = "Routing data", body = ContractorMatchResponse),
        (status = 401, description = "API key invalid", body = ApiError),
        (status = 404, description = "Session not found", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn contractor_match() -> impl IntoResponse {
    not_implemented("POST /v1/integration/contractor-match")
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/v1/integration/contractor-match", post(contractor_match))
}
