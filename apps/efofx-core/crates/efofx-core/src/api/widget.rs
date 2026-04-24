//! Widget public + authenticated endpoints (Phase 2D).

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use efofx_domain::{AnalyticsEventType, BrandingConfig, ConsultationRequest};
use efofx_openapi::{ApiError, ErrorCode};

use crate::middleware::rate_limit::rate_limit;
use crate::AppState;

use super::not_implemented;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LeadCaptureRequest {
    pub session_id: String,
    /// Customer name, 1–200 chars.
    pub name: String,
    pub email: String,
    /// Phone number, 5–30 chars.
    pub phone: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LeadCaptureResponse {
    pub message: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ConsultationCapturedResponse {
    pub lead_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AnalyticsEventRequest {
    pub event_type: AnalyticsEventType,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AnalyticsDailyBucket {
    /// ISO date `YYYY-MM-DD`.
    pub date: String,
    pub widget_view: u64,
    pub chat_start: u64,
    pub estimate_complete: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AnalyticsSummary {
    pub days: u32,
    pub buckets: Vec<AnalyticsDailyBucket>,
}

/// Public: fetch a tenant's widget branding by their API-key prefix.
/// The prefix is the 32-hex `tenant_id` (no dashes) that appears after
/// `sk_live_` in the widget's stored API key. Rate-limited per IP
/// (30 req/min). Never returns secrets or PII.
#[utoipa::path(
    get,
    path = "/v1/widget/branding/{api_key_prefix}",
    tag = "widget",
    params(
        ("api_key_prefix" = String, Path, description = "32-hex tenant-id prefix (the segment after `sk_live_` in the widget API key)")
    ),
    responses(
        (status = 200, description = "Branding config", body = BrandingConfig),
        (status = 404, description = "No tenant matches that prefix", body = ApiError),
        (status = 429, description = "Rate limit exceeded", body = ApiError),
    ),
)]
pub async fn get_branding(
    State(state): State<Arc<AppState>>,
    Path(api_key_prefix): Path<String>,
) -> Response {
    match state
        .tenants
        .fetch_branding_by_prefix(&api_key_prefix)
        .await
    {
        Ok(Some(resolved)) => Json(resolved.branding).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiError::single(
                ErrorCode::WidgetBrandingNotFound.as_str(),
                ErrorCode::WidgetBrandingNotFound.default_message(),
            )),
        )
            .into_response(),
        Err(err) => {
            tracing::error!(error = %err, "branding lookup failed");
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

/// Capture a lead from the widget.
#[utoipa::path(
    post,
    path = "/v1/widget/leads",
    tag = "widget",
    request_body = LeadCaptureRequest,
    security(("widget_api_key" = [])),
    responses(
        (status = 201, description = "Lead captured", body = LeadCaptureResponse),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 401, description = "API key invalid", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn create_lead() -> impl IntoResponse {
    not_implemented("POST /v1/widget/leads")
}

/// Submit a consultation request. Triggers the contractor notification email.
#[utoipa::path(
    post,
    path = "/v1/widget/consultations",
    tag = "widget",
    request_body = ConsultationRequest,
    security(("widget_api_key" = [])),
    responses(
        (status = 201, description = "Consultation captured", body = ConsultationCapturedResponse),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 401, description = "API key invalid", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn create_consultation() -> impl IntoResponse {
    not_implemented("POST /v1/widget/consultations")
}

/// Record a widget analytics event. Fire-and-forget from the widget.
#[utoipa::path(
    post,
    path = "/v1/widget/events",
    tag = "widget",
    request_body = AnalyticsEventRequest,
    security(("widget_api_key" = [])),
    responses(
        (status = 204, description = "Event recorded"),
        (status = 400, description = "Unknown event type", body = ApiError),
        (status = 401, description = "API key invalid", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn record_event() -> impl IntoResponse {
    not_implemented("POST /v1/widget/events")
}

/// Read daily analytics buckets for the authenticated tenant.
#[utoipa::path(
    get,
    path = "/v1/widget/events",
    tag = "widget",
    params(
        ("days" = Option<u32>, Query, description = "Days of history to return (default 30, max 365)")
    ),
    security(("supabase_jwt" = [])),
    responses(
        (status = 200, description = "Daily buckets", body = AnalyticsSummary),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 429, description = "Rate limit exceeded", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn list_events() -> impl IntoResponse {
    not_implemented("GET /v1/widget/events")
}

/// Assemble the widget router.
///
/// Layering (top-down execution order):
/// * `GET /v1/widget/branding/{prefix}` is **public**. It sits behind
///   the per-IP rate limiter only — no auth — so the widget can bootstrap
///   its theme before any user interaction.
/// * `/v1/widget/leads`, `/v1/widget/consultations`, `POST /v1/widget/events`
///   require a widget API key. Added in 2D.2–2D.4.
/// * `GET /v1/widget/events` requires a Supabase JWT (dashboard read).
///   Added in 2D.4.
pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let public = Router::new()
        .route("/v1/widget/branding/{api_key_prefix}", get(get_branding))
        .route_layer(from_fn_with_state(
            state.branding_rate_limiter.clone(),
            rate_limit,
        ));

    // The remaining widget routes keep their 501 stubs until their
    // subphase ships. They are reachable as-is (no auth yet) so the
    // contract surface stays stable for the OpenAPI lint.
    let other = Router::new()
        .route("/v1/widget/leads", post(create_lead))
        .route("/v1/widget/consultations", post(create_consultation))
        .route("/v1/widget/events", post(record_event).get(list_events));

    public.merge(other)
}
