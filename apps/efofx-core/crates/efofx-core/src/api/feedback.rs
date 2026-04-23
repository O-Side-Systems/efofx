//! Feedback endpoints (Phase 2E).
//!
//! Mix of authenticated JSON endpoints (submit, summary, email-requests)
//! and token-authenticated HTML form endpoints for customer-facing flows.

use std::sync::Arc;

use axum::{
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use efofx_domain::{FeedbackDocument, FeedbackSubmission, FeedbackSummary};
use efofx_openapi::ApiError;

use crate::AppState;

use super::not_implemented;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateFeedbackRequest {
    pub estimation_session_id: String,
    /// Free-text category, e.g. `"accuracy"` or `"timeline"`.
    pub feedback_type: String,
    pub rating: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_cost: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_timeline_weeks: Option<u32>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreateFeedbackResponse {
    pub feedback_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct FeedbackEmailRequest {
    pub estimation_session_id: String,
    pub customer_email: String,
    #[serde(default = "default_project_name")]
    pub project_name: String,
}

fn default_project_name() -> String {
    "Your Project".into()
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FeedbackEmailResponse {
    pub message: String,
    /// SHA-256 hash of the minted magic-link token. The raw token is sent
    /// only in the email; never returned to the requester.
    pub token_hash: String,
}

/// Submit feedback on an existing estimation session.
#[utoipa::path(
    post,
    path = "/v1/feedback",
    tag = "feedback",
    request_body = CreateFeedbackRequest,
    security(("supabase_jwt" = []), ("widget_api_key" = [])),
    responses(
        (status = 201, description = "Feedback captured", body = CreateFeedbackResponse),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 404, description = "Estimation session not found", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn create_feedback() -> impl IntoResponse {
    not_implemented("POST /v1/feedback")
}

/// Aggregate feedback summary for the authenticated tenant.
#[utoipa::path(
    get,
    path = "/v1/feedback/summary",
    tag = "feedback",
    security(("supabase_jwt" = [])),
    responses(
        (status = 200, description = "Aggregate stats", body = FeedbackSummary),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn feedback_summary() -> impl IntoResponse {
    not_implemented("GET /v1/feedback/summary")
}

/// Mint a feedback magic-link and email it to the customer. The raw token
/// is sent only in the email. Token TTL: 72 hours (config override).
#[utoipa::path(
    post,
    path = "/v1/feedback/email-requests",
    tag = "feedback",
    request_body = FeedbackEmailRequest,
    security(("supabase_jwt" = [])),
    responses(
        (status = 202, description = "Email queued", body = FeedbackEmailResponse),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn request_email() -> impl IntoResponse {
    not_implemented("POST /v1/feedback/email-requests")
}

/// Render the feedback form. Idempotent — scanner-safe. Sets `opened_at`
/// on first visit; does not consume the token.
#[utoipa::path(
    get,
    path = "/v1/feedback/forms/{token}",
    tag = "feedback",
    params(("token" = String, Path, description = "Raw magic-link token from email")),
    responses(
        (status = 200, description = "HTML form", content_type = "text/html"),
        (status = 404, description = "Token not found", body = ApiError),
        (status = 410, description = "Token expired", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn render_form() -> impl IntoResponse {
    not_implemented("GET /v1/feedback/forms/{token}")
}

/// Submit the feedback form. Consumes the token atomically — resubmits
/// return the thank-you page, never a double-write.
#[utoipa::path(
    post,
    path = "/v1/feedback/forms/{token}",
    tag = "feedback",
    params(("token" = String, Path)),
    request_body(content = FeedbackSubmission, content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = 200, description = "HTML thank-you page", content_type = "text/html"),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 404, description = "Token not found", body = ApiError),
        (status = 410, description = "Token already used or expired", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn submit_form() -> impl IntoResponse {
    not_implemented("POST /v1/feedback/forms/{token}")
}

// FeedbackDocument referenced so the OpenAPI component graph keeps it.
#[allow(dead_code)]
fn _keep_feedback_doc_in_openapi(_: &FeedbackDocument) {}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/feedback", post(create_feedback))
        .route("/v1/feedback/summary", get(feedback_summary))
        .route("/v1/feedback/email-requests", post(request_email))
        .route(
            "/v1/feedback/forms/{token}",
            get(render_form).post(submit_form),
        )
}
