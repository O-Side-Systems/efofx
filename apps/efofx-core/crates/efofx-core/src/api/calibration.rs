//! Calibration endpoints (Phase 2E).
//!
//! Both endpoints return a `below_threshold` envelope instead of metrics
//! when the tenant has fewer than `calibration.minimum_outcomes`
//! (config default 10). Preserve this behavior exactly — it's a product
//! decision (CALB-03) not an implementation detail.
//!
//! Wire shapes mirror FastAPI's `CalibrationService` byte-for-byte:
//! `accuracy_buckets` is a struct with four named proportion fields,
//! `by_reference_class` is a flat list, and `metrics` / `trend`
//! responses are returned as `#[serde(untagged)]` enums so a single
//! endpoint URL serves the below-threshold and full-data shapes
//! without an envelope discriminator on the wire.

use std::sync::Arc;

use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::{routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use efofx_auth::middleware::supabase_jwt;
use efofx_openapi::{ApiError, ErrorCode};
use efofx_storage::TenantContext;

use crate::AppState;

// ----------------------------------------------------------------------
// Below-threshold variants
// ----------------------------------------------------------------------

/// Returned by `GET /v1/calibration/metrics` when the tenant has fewer
/// than `minimum_outcomes` real feedback rows. Field order matches
/// FastAPI for snapshot-test stability.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MetricsBelowThreshold {
    pub below_threshold: bool,
    pub outcome_count: u64,
    pub threshold: u64,
}

/// Trend variant of [`MetricsBelowThreshold`] — same fields plus an
/// empty `trend: []` so the frontend can read `.trend` unconditionally.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TrendBelowThreshold {
    pub below_threshold: bool,
    pub outcome_count: u64,
    pub threshold: u64,
    pub trend: Vec<CalibrationTrendPoint>,
}

// ----------------------------------------------------------------------
// Above-threshold variants
// ----------------------------------------------------------------------

/// Exclusive variance buckets, returned as proportions (0.0–1.0).
/// Names mirror FastAPI's `_compute_accuracy_buckets`.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct AccuracyBuckets {
    pub within_10_pct: f64,
    pub within_20_pct: f64,
    pub within_30_pct: f64,
    pub beyond_30_pct: f64,
}

/// Per-reference-class accuracy summary. `reference_class` is the
/// free-form label submitted with the feedback row (FastAPI persists
/// `reference_class_id` as a string, not a UUID — `Unknown` when the
/// row had no RC).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReferenceClassAccuracy {
    pub reference_class: String,
    pub outcome_count: u64,
    pub mean_variance_pct: f64,
    pub accuracy_buckets: AccuracyBuckets,
    /// `true` when this RC has fewer than 5 outcomes — the dashboard
    /// dims the row to flag low-confidence numbers.
    pub limited_data: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CalibrationMetrics {
    pub below_threshold: bool,
    pub outcome_count: u64,
    pub threshold: u64,
    pub mean_variance_pct: f64,
    pub accuracy_buckets: AccuracyBuckets,
    pub by_reference_class: Vec<ReferenceClassAccuracy>,
    /// Echoes the validated `?date_range` filter, or `"all"` when
    /// none was supplied.
    pub date_range: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CalibrationTrendPoint {
    /// Period label, `"YYYY-MM"`.
    pub period: String,
    pub mean_variance_pct: f64,
    pub outcome_count: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CalibrationTrend {
    pub below_threshold: bool,
    pub outcome_count: u64,
    pub threshold: u64,
    pub trend: Vec<CalibrationTrendPoint>,
    pub months: u32,
}

// ----------------------------------------------------------------------
// Untagged response unions
// ----------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(untagged)]
pub enum MetricsResponse {
    BelowThreshold(MetricsBelowThreshold),
    Metrics(CalibrationMetrics),
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(untagged)]
pub enum TrendResponse {
    BelowThreshold(TrendBelowThreshold),
    Trend(CalibrationTrend),
}

// ----------------------------------------------------------------------
// Query parameters
// ----------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct MetricsParams {
    #[serde(default)]
    pub date_range: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrendParams {
    #[serde(default)]
    pub months: Option<u32>,
}

// ----------------------------------------------------------------------
// Handlers
// ----------------------------------------------------------------------

/// Accuracy metrics for the authenticated tenant. Returns a
/// below-threshold envelope when fewer than
/// `calibration.minimum_outcomes` real outcomes exist.
#[utoipa::path(
    get,
    path = "/v1/calibration/metrics",
    tag = "calibration",
    params(
        ("date_range" = Option<String>, Query, description = "`6months`, `1year`, or `all` (default `all`)")
    ),
    security(("supabase_jwt" = [])),
    responses(
        (status = 200, description = "Metrics or below-threshold", body = CalibrationMetrics),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
    ),
)]
pub async fn metrics(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
    Query(params): Query<MetricsParams>,
) -> Response {
    let date_range = match parse_date_range(params.date_range.as_deref()) {
        Ok(s) => s,
        Err(resp) => return resp,
    };
    match state.calibration.metrics(&ctx, date_range).await {
        Ok(resp) => Json(resp).into_response(),
        Err(err) => {
            tracing::error!(error = %err, "calibration metrics failed");
            err.into_response()
        }
    }
}

/// Monthly accuracy trend for the authenticated tenant. Threshold
/// check applies across *all* outcomes — a tenant with 8 lifetime
/// outcomes always sees the below-threshold envelope, regardless of
/// the requested window.
#[utoipa::path(
    get,
    path = "/v1/calibration/trend",
    tag = "calibration",
    params(
        ("months" = Option<u32>, Query, description = "Lookback in months (1–36, default 12)")
    ),
    security(("supabase_jwt" = [])),
    responses(
        (status = 200, description = "Monthly trend points", body = CalibrationTrend),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
    ),
)]
pub async fn trend(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
    Query(params): Query<TrendParams>,
) -> Response {
    let months = match parse_months(params.months) {
        Ok(m) => m,
        Err(resp) => return resp,
    };
    match state.calibration.trend(&ctx, months).await {
        Ok(resp) => Json(resp).into_response(),
        Err(err) => {
            tracing::error!(error = %err, "calibration trend failed");
            err.into_response()
        }
    }
}

// ----------------------------------------------------------------------
// Validation helpers
// ----------------------------------------------------------------------

/// Validated date-range filter. The handler converts the raw query
/// string into one of three accepted variants — anything else gets a
/// 400 envelope. Mirrors FastAPI's regex `^(6months|1year|all)$`.
#[derive(Debug, Clone, Copy)]
pub enum DateRange {
    SixMonths,
    OneYear,
    All,
}

impl DateRange {
    pub fn as_wire(self) -> &'static str {
        match self {
            DateRange::SixMonths => "6months",
            DateRange::OneYear => "1year",
            DateRange::All => "all",
        }
    }
}

#[allow(clippy::result_large_err)]
fn parse_date_range(raw: Option<&str>) -> Result<DateRange, Response> {
    match raw {
        None | Some("all") => Ok(DateRange::All),
        Some("6months") => Ok(DateRange::SixMonths),
        Some("1year") => Ok(DateRange::OneYear),
        Some(_) => Err(validation_error(
            "date_range must be one of: 6months, 1year, all",
        )),
    }
}

#[allow(clippy::result_large_err)]
fn parse_months(raw: Option<u32>) -> Result<u32, Response> {
    let m = raw.unwrap_or(12);
    if !(1..=36).contains(&m) {
        return Err(validation_error("months must be between 1 and 36"));
    }
    Ok(m)
}

fn validation_error(msg: &'static str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiError::single(ErrorCode::ValidationFailed.as_str(), msg)),
    )
        .into_response()
}

/// Assemble the calibration router. Both endpoints sit behind
/// supabase_jwt — they never serve widget keys.
pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/calibration/metrics", get(metrics))
        .route("/v1/calibration/trend", get(trend))
        .route_layer(from_fn_with_state(state.auth.clone(), supabase_jwt))
}
