//! Feedback endpoints (Phase 2E).
//!
//! Mix of authenticated JSON endpoints (submit, summary, email-requests)
//! and token-authenticated HTML form endpoints for customer-facing flows.

use std::sync::Arc;

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use efofx_auth::middleware::{either_auth, supabase_jwt};
use efofx_domain::{FeedbackDocument, FeedbackSubmission, FeedbackSummary};
use efofx_openapi::{ApiError, ErrorCode};
use efofx_storage::{NewFeedback, TenantContext};

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

/// Submit feedback on an existing estimation session. Accepts either a
/// dashboard-issued Supabase JWT or a widget API key — both flows produce
/// a [`TenantContext`] before the handler runs.
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
    ),
)]
pub async fn create_feedback(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
    Json(body): Json<CreateFeedbackRequest>,
) -> Response {
    if let Err(resp) = validate_create_feedback(&body) {
        return resp;
    }

    let new = NewFeedback {
        estimation_session_id: body.estimation_session_id,
        feedback_type: body.feedback_type,
        rating: body.rating,
        comment: body.comment,
        actual_cost: body.actual_cost,
        actual_timeline_weeks: body.actual_timeline_weeks,
        // The 2E.2 wire DTO doesn't carry these; magic-link / future
        // widget flows fill them in.
        actual_team_size: None,
        cost_accuracy: None,
        timeline_accuracy: None,
        reference_class_accuracy: None,
    };

    match state.feedback.insert_basic(&ctx, &new).await {
        Ok(feedback_id) => (
            StatusCode::CREATED,
            Json(CreateFeedbackResponse {
                feedback_id,
                message: "Feedback recorded successfully".into(),
            }),
        )
            .into_response(),
        Err(err) => {
            tracing::error!(error = %err, "feedback insert failed");
            internal_error()
        }
    }
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
    ),
)]
pub async fn feedback_summary(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
) -> Response {
    match state.feedback.summary(&ctx).await {
        Ok(stats) => Json(FeedbackSummary {
            total_feedback: stats.total_feedback,
            average_rating: stats.average_rating,
            cost_accuracy_avg: stats.cost_accuracy_avg,
            timeline_accuracy_avg: stats.timeline_accuracy_avg,
            reference_class_accuracy_avg: stats.reference_class_accuracy_avg,
            feedback_by_type: stats.feedback_by_type,
        })
        .into_response(),
        Err(err) => {
            tracing::error!(error = %err, "feedback summary failed");
            internal_error()
        }
    }
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

#[allow(clippy::result_large_err)]
fn validate_create_feedback(body: &CreateFeedbackRequest) -> Result<(), Response> {
    if body.estimation_session_id.trim().is_empty() {
        return Err(validation_error("estimation_session_id must not be empty"));
    }
    if body.feedback_type.trim().is_empty() {
        return Err(validation_error("feedback_type must not be empty"));
    }
    if !(1..=5).contains(&body.rating) {
        return Err(validation_error("rating must be between 1 and 5"));
    }
    Ok(())
}

fn validation_error(msg: &'static str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiError::single(ErrorCode::ValidationFailed.as_str(), msg)),
    )
        .into_response()
}

fn internal_error() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiError::single(
            "common.internal",
            "An internal error occurred",
        )),
    )
        .into_response()
}

/// Assemble the feedback router.
///
/// Layering, outer to inner:
/// * `POST /v1/feedback` accepts either Supabase JWT or widget API key —
///   contractor dashboards and embedded widgets both call this.
/// * `GET /v1/feedback/summary` is dashboard-only; supabase_jwt enforced.
/// * `POST /v1/feedback/email-requests` is dashboard-only (2E.3 stub).
/// * `GET|POST /v1/feedback/forms/{token}` is public, token-gated (2E.4
///   stub) — no auth layer applied here.
pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let either_authed = Router::new()
        .route("/v1/feedback", post(create_feedback))
        .route_layer(from_fn_with_state(state.auth.clone(), either_auth));

    let dashboard = Router::new()
        .route("/v1/feedback/summary", get(feedback_summary))
        .route("/v1/feedback/email-requests", post(request_email))
        .route_layer(from_fn_with_state(state.auth.clone(), supabase_jwt));

    let public = Router::new().route(
        "/v1/feedback/forms/:token",
        get(render_form).post(submit_form),
    );

    either_authed.merge(dashboard).merge(public)
}
