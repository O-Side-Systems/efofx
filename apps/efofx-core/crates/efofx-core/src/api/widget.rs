//! Widget public + authenticated endpoints (Phase 2D).

use std::sync::Arc;

use axum::{
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use efofx_domain::{AnalyticsEventType, BrandingConfig, ConsultationRequest};
use efofx_openapi::ApiError;

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

/// Public: fetch a tenant's widget branding by the last-6 of their API key.
/// Rate-limited per IP. Never includes secrets or PII.
#[utoipa::path(
    get,
    path = "/v1/widget/branding/{api_key_prefix}",
    tag = "widget",
    params(
        ("api_key_prefix" = String, Path, description = "Last-6 hex chars of tenant API key")
    ),
    responses(
        (status = 200, description = "Branding config", body = BrandingConfig),
        (status = 404, description = "No tenant matches that prefix", body = ApiError),
        (status = 429, description = "Rate limit exceeded", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn get_branding() -> impl IntoResponse {
    not_implemented("GET /v1/widget/branding/{prefix}")
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

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/widget/branding/{api_key_prefix}", get(get_branding))
        .route("/v1/widget/leads", post(create_lead))
        .route("/v1/widget/consultations", post(create_consultation))
        .route("/v1/widget/events", post(record_event).get(list_events))
}
