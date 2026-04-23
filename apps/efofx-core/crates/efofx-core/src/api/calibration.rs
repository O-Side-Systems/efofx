//! Calibration endpoints (Phase 2E).
//!
//! Both endpoints return a `below_threshold` response instead of metrics
//! when the tenant has fewer than `calibration.minimum_outcomes`
//! (config default 10). Preserve this behavior exactly — it's a product
//! decision (CALB-03) not an implementation detail.

use std::sync::Arc;

use axum::{response::IntoResponse, routing::get, Router};
use serde::Serialize;
use utoipa::ToSchema;

use efofx_openapi::ApiError;

use crate::AppState;

use super::not_implemented;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BelowThresholdResponse {
    pub below_threshold: bool,
    pub count: u64,
    pub threshold: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AccuracyBucket {
    /// Variance bucket label, e.g. `"<10%"`, `"10-20%"`.
    pub label: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CalibrationMetricsResponse {
    pub mean_variance_pct: f64,
    pub accuracy_buckets: Vec<AccuracyBucket>,
    pub by_reference_class: Vec<ReferenceClassAccuracy>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReferenceClassAccuracy {
    pub reference_class_id: String,
    pub reference_class_name: String,
    pub mean_variance_pct: f64,
    pub outcome_count: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CalibrationTrendPoint {
    /// Period label, `"YYYY-MM"`.
    pub period: String,
    pub mean_variance_pct: f64,
    pub outcome_count: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CalibrationTrendResponse {
    pub trend: Vec<CalibrationTrendPoint>,
}

/// Accuracy metrics and histogram. Below-threshold response when count
/// < `calibration.minimum_outcomes`.
#[utoipa::path(
    get,
    path = "/v1/calibration/metrics",
    tag = "calibration",
    params(
        ("date_range" = Option<String>, Query, description = "`6months`, `1year`, or `all` (default `all`)")
    ),
    security(("supabase_jwt" = [])),
    responses(
        (status = 200, description = "Metrics or below-threshold", body = CalibrationMetricsResponse),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn metrics() -> impl IntoResponse {
    not_implemented("GET /v1/calibration/metrics")
}

/// Time-series accuracy trend. Below-threshold response applies per-period.
#[utoipa::path(
    get,
    path = "/v1/calibration/trend",
    tag = "calibration",
    params(
        ("months" = Option<u32>, Query, description = "Lookback in months (1–36, default 12)")
    ),
    security(("supabase_jwt" = [])),
    responses(
        (status = 200, description = "Monthly trend points", body = CalibrationTrendResponse),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn trend() -> impl IntoResponse {
    not_implemented("GET /v1/calibration/trend")
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/calibration/metrics", get(metrics))
        .route("/v1/calibration/trend", get(trend))
}
