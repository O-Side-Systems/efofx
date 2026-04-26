use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;

use crate::estimation::CostCategoryEstimate;
use crate::ids::{SessionId, TenantId};

/// Scope-focused categorical reason for why an estimate diverged from reality.
/// Drives feedback-based calibration (Phase 2E).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiscrepancyReason {
    ScopeChanged,
    UnforeseenIssues,
    TimelinePressure,
    VendorMaterialCosts,
    ClientChanges,
    EstimateWasAccurate,
}

/// Immutable copy of the estimate at the moment feedback was submitted.
/// Persisted with the feedback so later estimate edits don't mutate history.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EstimateSnapshot {
    pub total_cost_p50: f64,
    pub total_cost_p80: f64,
    pub timeline_weeks_p50: u32,
    pub timeline_weeks_p80: u32,
    pub cost_breakdown: Vec<CostCategoryEstimate>,
    pub assumptions: Vec<String>,
    pub confidence_score: f64,
}

/// Request body for `POST /v1/feedback/forms/{token}`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeedbackSubmission {
    pub actual_cost: f64,
    pub actual_timeline_weeks: u32,
    /// Overall experience rating, 1–5.
    pub rating: u8,
    pub discrepancy_reason_primary: DiscrepancyReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discrepancy_reason_secondary: Option<DiscrepancyReason>,
    /// Free-text comment, up to 2000 chars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// Persisted feedback record. Stored in the `feedback` collection.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeedbackDocument {
    pub tenant_id: TenantId,
    pub estimation_session_id: SessionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_class_id: Option<String>,
    pub actual_cost: f64,
    pub actual_timeline_weeks: u32,
    pub rating: u8,
    pub discrepancy_reason_primary: DiscrepancyReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discrepancy_reason_secondary: Option<DiscrepancyReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub estimate_snapshot: EstimateSnapshot,
    #[serde(with = "time::serde::rfc3339")]
    pub submitted_at: OffsetDateTime,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
}

fn default_schema_version() -> u32 {
    1
}

/// Aggregate feedback stats returned by `GET /v1/feedback/summary`.
///
/// `feedback_by_type` and `reference_class_accuracy_avg` keep parity with
/// the FastAPI dashboard JSON. Both are skipped when empty/absent so the
/// wire shape stays compatible with older clients reading just the
/// totals.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct FeedbackSummary {
    pub total_feedback: u64,
    pub average_rating: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_accuracy_avg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_accuracy_avg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_class_accuracy_avg: Option<f64>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub feedback_by_type: HashMap<String, u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discrepancy_reason_wire_format() {
        assert_eq!(
            serde_json::to_string(&DiscrepancyReason::ScopeChanged).unwrap(),
            "\"scope_changed\""
        );
    }
}
