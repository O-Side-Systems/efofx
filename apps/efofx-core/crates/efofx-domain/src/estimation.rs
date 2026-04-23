use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Status of an estimation session. Mirrors `efofx_shared.core.constants.EstimationStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EstimationStatus {
    Initiated,
    InProgress,
    Completed,
    Cancelled,
    Expired,
}

/// Top-level cost categories for output decomposition.
/// Mirrors `efofx_shared.core.constants.CostBreakdownCategory`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CostBreakdownCategory {
    Materials,
    Labor,
    Equipment,
    Permits,
    Design,
    Contingency,
    ProfitMargin,
}

/// One line of the cost breakdown in a structured estimate.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CostCategoryEstimate {
    /// Category name. Accepts freeform text because legacy data carries
    /// string values; new code should prefer [`CostBreakdownCategory`].
    pub category: String,
    pub p50_cost: f64,
    pub p80_cost: f64,
    /// Share of total cost, 0.0–1.0.
    pub percentage_of_total: f64,
}

/// A named multiplier applied to the estimate with a human-readable reason.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdjustmentFactor {
    pub name: String,
    pub multiplier: f64,
    pub reason: String,
}

/// Authoritative output of the estimation LLM call. This is the shape the
/// OpenAI structured-output parse produces, and the shape clients render.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EstimationOutput {
    pub total_cost_p50: f64,
    pub total_cost_p80: f64,
    pub timeline_weeks_p50: u32,
    pub timeline_weeks_p80: u32,
    pub cost_breakdown: Vec<CostCategoryEstimate>,
    pub adjustment_factors: Vec<AdjustmentFactor>,
    /// Confidence 0–100 reflecting information completeness.
    pub confidence_score: f64,
    pub assumptions: Vec<String>,
    /// One-paragraph plain-language summary rendered beside the numbers.
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimation_status_wire_format() {
        assert_eq!(
            serde_json::to_string(&EstimationStatus::InProgress).unwrap(),
            "\"in_progress\""
        );
        let parsed: EstimationStatus = serde_json::from_str("\"completed\"").unwrap();
        assert_eq!(parsed, EstimationStatus::Completed);
    }

    #[test]
    fn cost_breakdown_category_profit_margin_snake() {
        assert_eq!(
            serde_json::to_string(&CostBreakdownCategory::ProfitMargin).unwrap(),
            "\"profit_margin\""
        );
    }

    #[test]
    fn estimation_output_roundtrip() {
        let out = EstimationOutput {
            total_cost_p50: 60000.0,
            total_cost_p80: 72000.0,
            timeline_weeks_p50: 8,
            timeline_weeks_p80: 12,
            cost_breakdown: vec![CostCategoryEstimate {
                category: "materials".into(),
                p50_cost: 24000.0,
                p80_cost: 28000.0,
                percentage_of_total: 0.4,
            }],
            adjustment_factors: vec![AdjustmentFactor {
                name: "Urban premium".into(),
                multiplier: 1.15,
                reason: "dense metro area".into(),
            }],
            confidence_score: 72.0,
            assumptions: vec!["flat lot".into()],
            summary: "A standard residential pool in coastal SoCal.".into(),
        };
        let json = serde_json::to_string(&out).unwrap();
        let back: EstimationOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(back.total_cost_p50, 60000.0);
    }
}
