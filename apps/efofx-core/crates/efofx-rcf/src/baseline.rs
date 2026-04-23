//! Baseline-estimate extraction and complexity/risk adjustments.
//!
//! `calculate_baseline_estimate` and `apply_adjustments` are pure and
//! mirror the FastAPI engine byte-for-byte. They operate on the unified
//! `ReferenceClass` domain type (not raw JSON) so callers get compile-time
//! checking on the required fields.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tracing::warn;
use utoipa::ToSchema;

use efofx_domain::ReferenceClass;

// --- baseline ------------------------------------------------------------

/// Baseline estimate produced from a reference class. Costs are rounded to
/// the cent; the breakdown is reconciled so its sum equals `p50_cost`
/// exactly (within 1¢).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BaselineEstimate {
    pub p50_cost: f64,
    pub p80_cost: f64,
    /// `p80_cost - p50_cost`.
    pub variance: f64,
    pub p50_timeline_days: u32,
    pub p80_timeline_days: u32,
    pub cost_breakdown: BTreeMap<String, f64>,
    pub reference_class_name: String,
}

/// Compute the baseline estimate from a reference class. Fails only if the
/// breakdown template is empty (nothing to allocate to).
pub fn calculate_baseline_estimate(rc: &ReferenceClass) -> BaselineEstimate {
    let p50 = rc.cost_distribution.p50;
    let p80 = rc.cost_distribution.p80;
    let variance = p80 - p50;

    let mut breakdown: BTreeMap<String, f64> = rc
        .cost_breakdown_template
        .iter()
        .map(|(cat, pct)| (cat.clone(), round2(p50 * pct)))
        .collect();

    // Reconcile rounding drift against P50. Adjust the category with the
    // largest template share (stable tiebreak by name for determinism).
    let total: f64 = breakdown.values().sum();
    let diff = round2(p50 - total);
    if diff.abs() > 0.0 {
        let largest = rc
            .cost_breakdown_template
            .iter()
            .max_by(|a, b| {
                a.1.partial_cmp(b.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| b.0.cmp(a.0))
            })
            .map(|(k, _)| k.clone());
        if let Some(k) = largest {
            if let Some(v) = breakdown.get_mut(&k) {
                *v = round2(*v + diff);
            }
        }
    }

    let final_total: f64 = breakdown.values().sum();
    if (final_total - p50).abs() > 0.01 {
        warn!(total = final_total, p50, "baseline breakdown drift > 1¢");
    }

    BaselineEstimate {
        p50_cost: p50,
        p80_cost: p80,
        variance,
        p50_timeline_days: rc.timeline_distribution.p50_days,
        p80_timeline_days: rc.timeline_distribution.p80_days,
        cost_breakdown: breakdown,
        reference_class_name: rc.name.clone(),
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

// --- adjustments ---------------------------------------------------------

/// Complexity rating, case-insensitive on input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Complexity {
    Simple,
    Standard,
    Complex,
}

impl Complexity {
    /// Parse `"simple"` / `"standard"` / `"complex"` (case-insensitive).
    /// Anything else → [`Complexity::Standard`], matching FastAPI's
    /// `.get(complexity, 1.0)` fallback.
    pub fn parse(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "simple" => Complexity::Simple,
            "complex" => Complexity::Complex,
            _ => Complexity::Standard,
        }
    }
    pub fn multiplier(self) -> f64 {
        match self {
            Complexity::Simple => 0.8,
            Complexity::Standard => 1.0,
            Complexity::Complex => 1.5,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Complexity::Simple => "simple",
            Complexity::Standard => "standard",
            Complexity::Complex => "complex",
        }
    }
}

/// Risk level, case-insensitive on input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

impl RiskLevel {
    /// Parse `"low"` / `"medium"` / `"high"` (case-insensitive). Anything
    /// else → [`RiskLevel::Low`].
    pub fn parse(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "medium" => RiskLevel::Medium,
            "high" => RiskLevel::High,
            _ => RiskLevel::Low,
        }
    }
    pub fn multiplier(self) -> f64 {
        match self {
            RiskLevel::Low => 1.0,
            RiskLevel::Medium => 1.15,
            RiskLevel::High => 1.3,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            RiskLevel::Low => "low",
            RiskLevel::Medium => "medium",
            RiskLevel::High => "high",
        }
    }
}

/// Output of [`apply_adjustments`]: baseline, factors, and the final
/// numbers. Timeline is affected by complexity only (not risk), matching
/// FastAPI's behavior.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdjustedEstimate {
    pub baseline_p50: f64,
    pub baseline_p80: f64,
    pub baseline_variance: f64,
    pub baseline_p50_timeline: u32,
    pub baseline_p80_timeline: u32,

    pub complexity: String,
    pub complexity_factor: f64,
    pub risk_level: String,
    pub risk_factor: f64,

    pub adjusted_p50: f64,
    pub adjusted_p80: f64,
    pub adjusted_variance: f64,
    pub adjusted_p50_timeline: u32,
    pub adjusted_p80_timeline: u32,

    pub cost_breakdown: BTreeMap<String, f64>,
    pub adjustment_summary: String,
    pub reference_class_name: String,
}

/// Apply complexity × risk multipliers to a baseline. Missing inputs
/// resolve to `Standard` / `Low` (no-op).
pub fn apply_adjustments(
    baseline: &BaselineEstimate,
    complexity: Option<&str>,
    risk_level: Option<&str>,
) -> AdjustedEstimate {
    let c = Complexity::parse(complexity.unwrap_or("standard"));
    let r = RiskLevel::parse(risk_level.unwrap_or("low"));
    let cf = c.multiplier();
    let rf = r.multiplier();

    let adjusted_p50 = round2(baseline.p50_cost * cf * rf);
    let adjusted_p80 = round2(baseline.p80_cost * cf * rf);
    let adjusted_variance = round2(adjusted_p80 - adjusted_p50);

    let adjusted_p50_timeline = (baseline.p50_timeline_days as f64 * cf) as u32;
    let adjusted_p80_timeline = (baseline.p80_timeline_days as f64 * cf) as u32;

    let mut cost_breakdown: BTreeMap<String, f64> = baseline
        .cost_breakdown
        .iter()
        .map(|(k, v)| (k.clone(), round2(v * cf * rf)))
        .collect();

    // Reconcile against adjusted_p50, same drift-fix as baseline.
    let total: f64 = cost_breakdown.values().sum();
    let diff = round2(adjusted_p50 - total);
    if diff.abs() > 0.0 {
        let largest = baseline
            .cost_breakdown
            .iter()
            .max_by(|a, b| {
                a.1.partial_cmp(b.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| b.0.cmp(a.0))
            })
            .map(|(k, _)| k.clone());
        if let Some(k) = largest {
            if let Some(v) = cost_breakdown.get_mut(&k) {
                *v = round2(*v + diff);
            }
        }
    }

    let mut summary_parts = Vec::new();
    if cf != 1.0 {
        summary_parts.push(format!("complexity={} ({}x)", c.as_str(), cf));
    }
    if rf != 1.0 {
        summary_parts.push(format!("risk={} ({}x)", r.as_str(), rf));
    }
    let summary = if summary_parts.is_empty() {
        "No adjustments applied (standard complexity, low risk)".to_string()
    } else {
        format!("Applied adjustments: {}", summary_parts.join(", "))
    };

    AdjustedEstimate {
        baseline_p50: baseline.p50_cost,
        baseline_p80: baseline.p80_cost,
        baseline_variance: baseline.variance,
        baseline_p50_timeline: baseline.p50_timeline_days,
        baseline_p80_timeline: baseline.p80_timeline_days,
        complexity: c.as_str().to_string(),
        complexity_factor: cf,
        risk_level: r.as_str().to_string(),
        risk_factor: rf,
        adjusted_p50,
        adjusted_p80,
        adjusted_variance,
        adjusted_p50_timeline,
        adjusted_p80_timeline,
        cost_breakdown,
        adjustment_summary: summary,
        reference_class_name: baseline.reference_class_name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use efofx_domain::{CostDistribution, ReferenceClass, TimelineDistribution};
    use std::collections::BTreeMap;
    use time::OffsetDateTime;

    fn rc_with(
        breakdown: &[(&str, f64)],
        p50: f64,
        p80: f64,
        t50: u32,
        t80: u32,
    ) -> ReferenceClass {
        let mut map = BTreeMap::new();
        for (k, v) in breakdown {
            map.insert((*k).to_string(), *v);
        }
        ReferenceClass {
            id: None,
            tenant_id: None,
            category: "construction".into(),
            subcategory: "pool".into(),
            name: "Test Pool - Medium".into(),
            description: "test".into(),
            keywords: vec![],
            regions: vec![],
            attributes: BTreeMap::new(),
            cost_distribution: CostDistribution {
                p50,
                p80,
                p95: p80 + 10_000.0,
                currency: "USD".into(),
            },
            timeline_distribution: TimelineDistribution {
                p50_days: t50,
                p80_days: t80,
                p95_days: t80 + 15,
            },
            cost_breakdown_template: map,
            is_synthetic: true,
            validation_source: "test".into(),
            created_at: OffsetDateTime::now_utc(),
            updated_at: None,
        }
    }

    // --- baseline --------------------------------------------------------

    #[test]
    fn baseline_basic() {
        let rc = rc_with(
            &[
                ("materials", 0.40),
                ("labor", 0.30),
                ("equipment", 0.10),
                ("permits", 0.05),
                ("finishing", 0.15),
            ],
            55_000.0,
            72_000.0,
            50,
            65,
        );
        let b = calculate_baseline_estimate(&rc);
        assert_eq!(b.p50_cost, 55_000.0);
        assert_eq!(b.p80_cost, 72_000.0);
        assert_eq!(b.variance, 17_000.0);
        assert_eq!(b.p50_timeline_days, 50);
        assert_eq!(b.p80_timeline_days, 65);
        assert!(b.cost_breakdown.contains_key("materials"));
        let total: f64 = b.cost_breakdown.values().sum();
        assert!((total - 55_000.0).abs() <= 0.01);
        assert_eq!(b.reference_class_name, "Test Pool - Medium");
    }

    #[test]
    fn baseline_breakdown_percentages() {
        let rc = rc_with(
            &[("materials", 0.50), ("labor", 0.30), ("overhead", 0.20)],
            100_000.0,
            120_000.0,
            30,
            40,
        );
        let b = calculate_baseline_estimate(&rc);
        assert!((b.cost_breakdown["materials"] - 50_000.0).abs() <= 1.0);
        assert!((b.cost_breakdown["labor"] - 30_000.0).abs() <= 1.0);
        assert!((b.cost_breakdown["overhead"] - 20_000.0).abs() <= 1.0);
        let total: f64 = b.cost_breakdown.values().sum();
        assert!((total - 100_000.0).abs() <= 0.01);
    }

    #[test]
    fn baseline_rounding_adjusts_largest_category() {
        // 33333.33 with five categories introduces rounding drift. Largest
        // template share ("materials", 0.40) should absorb the difference.
        let rc = rc_with(
            &[
                ("materials", 0.40),
                ("labor", 0.30),
                ("equipment", 0.10),
                ("permits", 0.05),
                ("overhead", 0.15),
            ],
            33_333.33,
            40_000.0,
            30,
            40,
        );
        let b = calculate_baseline_estimate(&rc);
        let total: f64 = b.cost_breakdown.values().sum();
        assert!((total - 33_333.33).abs() <= 0.01);
    }

    #[test]
    fn baseline_variance_is_p80_minus_p50() {
        let rc = rc_with(
            &[("materials", 0.5), ("labor", 0.5)],
            50_000.0,
            70_000.0,
            30,
            40,
        );
        let b = calculate_baseline_estimate(&rc);
        assert_eq!(b.variance, 20_000.0);
    }

    // --- adjustments -----------------------------------------------------

    fn sample_baseline() -> BaselineEstimate {
        let mut bk = BTreeMap::new();
        bk.insert("materials".into(), 50_000.0);
        bk.insert("labor".into(), 30_000.0);
        bk.insert("overhead".into(), 20_000.0);
        BaselineEstimate {
            p50_cost: 100_000.0,
            p80_cost: 120_000.0,
            variance: 20_000.0,
            p50_timeline_days: 60,
            p80_timeline_days: 75,
            cost_breakdown: bk,
            reference_class_name: "Test Project".into(),
        }
    }

    #[test]
    fn adjust_none_is_identity() {
        let a = apply_adjustments(&sample_baseline(), None, None);
        assert_eq!(a.complexity, "standard");
        assert_eq!(a.complexity_factor, 1.0);
        assert_eq!(a.risk_level, "low");
        assert_eq!(a.risk_factor, 1.0);
        assert_eq!(a.adjusted_p50, 100_000.0);
        assert_eq!(a.adjusted_p80, 120_000.0);
        assert_eq!(a.adjusted_p50_timeline, 60);
        assert_eq!(a.adjusted_p80_timeline, 75);
        assert!(a.adjustment_summary.contains("No adjustments"));
    }

    #[test]
    fn adjust_simple_reduces_by_08() {
        let a = apply_adjustments(&sample_baseline(), Some("simple"), None);
        assert_eq!(a.complexity_factor, 0.8);
        assert_eq!(a.adjusted_p50, 80_000.0);
        assert_eq!(a.adjusted_p80, 96_000.0);
        assert_eq!(a.adjusted_p50_timeline, 48);
        assert_eq!(a.adjusted_p80_timeline, 60);
    }

    #[test]
    fn adjust_complex_bumps_by_15() {
        let a = apply_adjustments(&sample_baseline(), Some("complex"), None);
        assert_eq!(a.complexity_factor, 1.5);
        assert_eq!(a.adjusted_p50, 150_000.0);
        assert_eq!(a.adjusted_p80, 180_000.0);
        // 75 * 1.5 = 112.5 truncated to 112 (matches FastAPI `int()`).
        assert_eq!(a.adjusted_p50_timeline, 90);
        assert_eq!(a.adjusted_p80_timeline, 112);
    }

    #[test]
    fn adjust_medium_risk() {
        let a = apply_adjustments(&sample_baseline(), None, Some("medium"));
        assert_eq!(a.risk_factor, 1.15);
        assert_eq!(a.adjusted_p50, 115_000.0);
        assert_eq!(a.adjusted_p80, 138_000.0);
        // Timeline unaffected by risk.
        assert_eq!(a.adjusted_p50_timeline, 60);
        assert_eq!(a.adjusted_p80_timeline, 75);
    }

    #[test]
    fn adjust_high_risk() {
        let a = apply_adjustments(&sample_baseline(), None, Some("high"));
        assert_eq!(a.risk_factor, 1.3);
        assert_eq!(a.adjusted_p50, 130_000.0);
        assert_eq!(a.adjusted_p80, 156_000.0);
    }

    #[test]
    fn adjust_combined() {
        let a = apply_adjustments(&sample_baseline(), Some("complex"), Some("high"));
        assert_eq!(a.complexity_factor, 1.5);
        assert_eq!(a.risk_factor, 1.3);
        assert_eq!(a.adjusted_p50, 195_000.0);
        assert_eq!(a.adjusted_p80, 234_000.0);
        assert_eq!(a.adjusted_p50_timeline, 90);
    }

    #[test]
    fn adjust_breakdown_scales_proportionally() {
        let a = apply_adjustments(&sample_baseline(), Some("complex"), Some("high"));
        let bk = &a.cost_breakdown;
        assert!((bk["materials"] - 50_000.0 * 1.95).abs() <= 1.0);
        assert!((bk["labor"] - 30_000.0 * 1.95).abs() <= 1.0);
        assert!((bk["overhead"] - 20_000.0 * 1.95).abs() <= 1.0);
        let total: f64 = bk.values().sum();
        assert!((total - 195_000.0).abs() <= 0.01);
    }

    #[test]
    fn adjust_variance_recalculated() {
        let a = apply_adjustments(&sample_baseline(), Some("complex"), None);
        // 180_000 - 150_000 = 30_000
        assert_eq!(a.adjusted_variance, 30_000.0);
    }

    #[test]
    fn adjust_case_insensitive() {
        let a = apply_adjustments(&sample_baseline(), Some("COMPLEX"), Some("HIGH"));
        let b = apply_adjustments(&sample_baseline(), Some("Complex"), Some("High"));
        let c = apply_adjustments(&sample_baseline(), Some("complex"), Some("high"));
        assert_eq!(a.adjusted_p50, b.adjusted_p50);
        assert_eq!(b.adjusted_p50, c.adjusted_p50);
    }

    #[test]
    fn adjust_invalid_complexity_defaults_standard() {
        let a = apply_adjustments(&sample_baseline(), Some("nonsense"), None);
        assert_eq!(a.complexity_factor, 1.0);
    }

    #[test]
    fn adjust_invalid_risk_defaults_low() {
        let a = apply_adjustments(&sample_baseline(), None, Some("nonsense"));
        assert_eq!(a.risk_factor, 1.0);
    }

    #[test]
    fn adjust_summary_mentions_applied_factors() {
        let plain = apply_adjustments(&sample_baseline(), None, None);
        assert!(plain.adjustment_summary.contains("No adjustments"));

        let c_only = apply_adjustments(&sample_baseline(), Some("complex"), None);
        assert!(c_only.adjustment_summary.contains("complexity=complex"));

        let r_only = apply_adjustments(&sample_baseline(), None, Some("high"));
        assert!(r_only.adjustment_summary.contains("risk=high"));

        let both = apply_adjustments(&sample_baseline(), Some("complex"), Some("high"));
        assert!(both.adjustment_summary.contains("complexity=complex"));
        assert!(both.adjustment_summary.contains("risk=high"));
    }

    #[test]
    fn adjust_preserves_baseline_values() {
        let a = apply_adjustments(&sample_baseline(), Some("complex"), Some("high"));
        assert_eq!(a.baseline_p50, 100_000.0);
        assert_eq!(a.baseline_p80, 120_000.0);
        assert_eq!(a.baseline_p50_timeline, 60);
        assert_eq!(a.baseline_p80_timeline, 75);
    }
}
