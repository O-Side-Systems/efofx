//! Calibration orchestrator (Phase 2E.5).
//!
//! Owns the policy that turns raw aggregation rows from
//! [`efofx_storage::CalibrationRepo`] into the wire shapes the
//! dashboard consumes:
//!
//! * **Threshold gate** (CALB-03): swap to `MetricsBelowThreshold` /
//!   `TrendBelowThreshold` when fewer than `minimum_outcomes` real
//!   outcomes exist.
//! * **Bucketing** (CALB-02): exclusive variance buckets — `[0, 10]`,
//!   `(10, 20]`, `(20, 30]`, `> 30` — as proportions.
//! * **Per-RC summary**: `Unknown` label for `None` reference classes,
//!   `limited_data` flag at fewer than 5 outcomes per RC.
//! * **Trend window**: `now() - months * 30 days` (FastAPI parity —
//!   not a calendar-aware calculation).

use thiserror::Error;

use efofx_config::CalibrationConfig;
use efofx_openapi::ApiError;
use efofx_storage::{CalibrationRcGroup, CalibrationRepo, DateFilter, StorageError, TenantContext};

use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};

use crate::api::calibration::{
    AccuracyBuckets, CalibrationMetrics, CalibrationTrend, CalibrationTrendPoint, DateRange,
    MetricsBelowThreshold, MetricsResponse, ReferenceClassAccuracy, TrendBelowThreshold,
    TrendResponse,
};

#[derive(Debug, Error)]
pub enum CalibrationServiceError {
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl IntoResponse for CalibrationServiceError {
    fn into_response(self) -> Response {
        let CalibrationServiceError::Storage(_) = &self;
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::single(
                "common.internal",
                "Calibration aggregation failed",
            )),
        )
            .into_response()
    }
}

#[derive(Clone)]
pub struct CalibrationService {
    repo: CalibrationRepo,
    config: CalibrationConfig,
}

impl CalibrationService {
    pub fn new(repo: CalibrationRepo, config: CalibrationConfig) -> Self {
        Self { repo, config }
    }

    /// Compute calibration metrics for the tenant. Returns the
    /// below-threshold envelope when the tenant has fewer than
    /// `minimum_outcomes` real feedback rows in the requested window.
    pub async fn metrics(
        &self,
        ctx: &TenantContext,
        date_range: DateRange,
    ) -> Result<MetricsResponse, CalibrationServiceError> {
        let date_filter = date_filter_for(date_range);
        let count = self.repo.count_outcomes(ctx, date_filter).await?;
        let threshold = self.config.minimum_outcomes;

        if count < threshold {
            return Ok(MetricsResponse::BelowThreshold(MetricsBelowThreshold {
                below_threshold: true,
                outcome_count: count,
                threshold,
            }));
        }

        let groups = self.repo.metrics(ctx, date_filter).await?;
        let (overall_mean, overall_buckets, by_reference_class) = build_metrics_summary(&groups);

        Ok(MetricsResponse::Metrics(CalibrationMetrics {
            below_threshold: false,
            outcome_count: count,
            threshold,
            mean_variance_pct: overall_mean,
            accuracy_buckets: overall_buckets,
            by_reference_class,
            date_range: date_range.as_wire().to_string(),
        }))
    }

    /// Compute monthly accuracy trend for the tenant. Threshold check
    /// counts every outcome for the tenant — a tenant with 8 lifetime
    /// outcomes always sees the below-threshold envelope regardless
    /// of the requested window.
    pub async fn trend(
        &self,
        ctx: &TenantContext,
        months: u32,
    ) -> Result<TrendResponse, CalibrationServiceError> {
        let count = self.repo.count_outcomes(ctx, DateFilter::All).await?;
        let threshold = self.config.minimum_outcomes;

        if count < threshold {
            return Ok(TrendResponse::BelowThreshold(TrendBelowThreshold {
                below_threshold: true,
                outcome_count: count,
                threshold,
                trend: Vec::new(),
            }));
        }

        let since_days = (months as i64).saturating_mul(30);
        let points = self.repo.trend(ctx, since_days).await?;
        let trend = points
            .into_iter()
            .map(|p| CalibrationTrendPoint {
                period: p.period,
                mean_variance_pct: p.mean_variance_pct,
                outcome_count: p.outcome_count,
            })
            .collect();

        Ok(TrendResponse::Trend(CalibrationTrend {
            below_threshold: false,
            outcome_count: count,
            threshold,
            trend,
            months,
        }))
    }
}

// ----------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------

fn date_filter_for(date_range: DateRange) -> DateFilter {
    match date_range {
        DateRange::All => DateFilter::All,
        DateRange::SixMonths => DateFilter::SinceDays(182),
        DateRange::OneYear => DateFilter::SinceDays(365),
    }
}

/// Roll up per-RC variance lists into the response shape: overall
/// mean, overall buckets (across all RCs), and a per-RC list with
/// own-mean / own-buckets / `limited_data`.
fn build_metrics_summary(
    groups: &[CalibrationRcGroup],
) -> (f64, AccuracyBuckets, Vec<ReferenceClassAccuracy>) {
    let mut all_variances: Vec<f64> = Vec::new();
    let mut by_rc: Vec<ReferenceClassAccuracy> = Vec::with_capacity(groups.len());

    for g in groups {
        all_variances.extend_from_slice(&g.variances);
        let mean_var = mean_round_1(&g.variances);
        by_rc.push(ReferenceClassAccuracy {
            reference_class: g.id.clone().unwrap_or_else(|| "Unknown".to_string()),
            outcome_count: g.outcome_count,
            mean_variance_pct: mean_var,
            accuracy_buckets: compute_accuracy_buckets(&g.variances),
            limited_data: g.outcome_count < 5,
        });
    }

    let overall_mean = mean_round_1(&all_variances);
    let overall_buckets = compute_accuracy_buckets(&all_variances);
    (overall_mean, overall_buckets, by_rc)
}

fn mean_round_1(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    let mean = xs.iter().sum::<f64>() / xs.len() as f64;
    (mean * 10.0).round() / 10.0
}

/// Exclusive variance buckets matching FastAPI's
/// `_compute_accuracy_buckets`. Returns proportions in `[0.0, 1.0]`.
/// Empty input yields all zeros.
fn compute_accuracy_buckets(variances: &[f64]) -> AccuracyBuckets {
    if variances.is_empty() {
        return AccuracyBuckets::default();
    }
    let total = variances.len() as f64;
    let mut w10 = 0u64;
    let mut w20 = 0u64;
    let mut w30 = 0u64;
    let mut b30 = 0u64;
    for v in variances {
        if *v <= 10.0 {
            w10 += 1;
        } else if *v <= 20.0 {
            w20 += 1;
        } else if *v <= 30.0 {
            w30 += 1;
        } else {
            b30 += 1;
        }
    }
    AccuracyBuckets {
        within_10_pct: w10 as f64 / total,
        within_20_pct: w20 as f64 / total,
        within_30_pct: w30 as f64 / total,
        beyond_30_pct: b30 as f64 / total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn buckets_empty_input_is_all_zero() {
        let b = compute_accuracy_buckets(&[]);
        assert_eq!(b.within_10_pct, 0.0);
        assert_eq!(b.within_20_pct, 0.0);
        assert_eq!(b.within_30_pct, 0.0);
        assert_eq!(b.beyond_30_pct, 0.0);
    }

    #[test]
    fn buckets_partition_exclusively() {
        // 5 values → 1 in each bucket plus one boundary that lands in
        // within_10 (≤ 10.0).
        let xs = [0.0, 10.0, 15.0, 25.0, 50.0];
        let b = compute_accuracy_buckets(&xs);
        assert!(approx(b.within_10_pct, 0.4));
        assert!(approx(b.within_20_pct, 0.2));
        assert!(approx(b.within_30_pct, 0.2));
        assert!(approx(b.beyond_30_pct, 0.2));
        assert!(approx(
            b.within_10_pct + b.within_20_pct + b.within_30_pct + b.beyond_30_pct,
            1.0
        ));
    }

    #[test]
    fn mean_rounds_to_one_decimal() {
        assert!(approx(mean_round_1(&[10.0, 20.0, 30.0]), 20.0));
        assert!(approx(mean_round_1(&[1.0, 2.0, 3.0]), 2.0));
        // 0.333... rounds to 0.3
        assert!(approx(mean_round_1(&[0.0, 1.0]), 0.5));
        assert!(approx(mean_round_1(&[0.0, 0.0, 1.0]), 0.3));
        // empty
        assert!(approx(mean_round_1(&[]), 0.0));
    }
}
