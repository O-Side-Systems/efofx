use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{Region, TenantId};

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

/// Short prefixed identifier for an estimation session: `sess_{hex[:12]}`.
/// Distinct from [`crate::SessionId`] (UUID) because estimates and chat
/// sessions live in separate identifier spaces — mirroring FastAPI, where
/// `EstimationSession.session_id` is a freshly-minted short ID, not a
/// carry-over of the chat-session UUID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct EstimationSessionId(pub String);

impl EstimationSessionId {
    /// Mint a fresh ID in the FastAPI shape: `sess_` + first 12 hex chars of
    /// a UUID v4 (lowercase, no dashes). 17 chars total.
    pub fn new() -> Self {
        let hex = Uuid::new_v4().simple().to_string();
        Self(format!("sess_{}", &hex[..12]))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for EstimationSessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EstimationSessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<String> for EstimationSessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Persisted estimation session.
///
/// Shape mirrors `apps/efofx-estimate/app/models/estimation.py::EstimationSession`
/// for the fields the new (Rust) flow actually writes. Three FastAPI fields
/// are intentionally omitted because the new flow never populates them:
///
/// - `result` (legacy `Any`, always `None` post-structured-output rollout)
/// - `chat_messages` (was a redundant message-id mirror)
/// - `images` (image upload path is deferred)
///
/// Dropping them keeps the Rust write path honest: Pydantic-on-read would
/// default these to `None`/`[]` anyway, so mixed-reader environments are
/// safe during a cutover.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EstimationSession {
    pub id: EstimationSessionId,
    pub tenant_id: TenantId,
    pub status: EstimationStatus,
    pub description: String,
    pub region: Region,
    pub reference_class: Option<String>,
    pub confidence_threshold: f64,
    pub prompt_version: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub completed_at: Option<OffsetDateTime>,
    pub expires_at: Option<OffsetDateTime>,
}

impl EstimationSession {
    /// Flip status to [`EstimationStatus::Expired`] if `expires_at` is in the
    /// past relative to `now`. Returns `true` when the status actually
    /// changed — callers persist only on a real transition.
    pub fn expire_if_past(&mut self, now: OffsetDateTime) -> bool {
        if self.status == EstimationStatus::Expired {
            return false;
        }
        match self.expires_at {
            Some(expires) if now > expires => {
                self.status = EstimationStatus::Expired;
                self.updated_at = now;
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration as TimeDuration;

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

    #[test]
    fn estimation_session_id_shape() {
        let id = EstimationSessionId::new();
        assert_eq!(id.0.len(), 17, "sess_ + 12 hex = 17");
        assert!(id.0.starts_with("sess_"));
        assert!(
            id.0[5..].chars().all(|c| c.is_ascii_hexdigit()),
            "suffix must be lowercase hex: {}",
            id.0
        );
    }

    #[test]
    fn estimation_session_id_transparent_serde() {
        let id = EstimationSessionId("sess_abcdef012345".into());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"sess_abcdef012345\"");
        let back: EstimationSessionId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    fn sample_session(now: OffsetDateTime) -> EstimationSession {
        EstimationSession {
            id: EstimationSessionId::new(),
            tenant_id: TenantId::new(),
            status: EstimationStatus::Completed,
            description: "A pool in the SoCal coast.".into(),
            region: Region::SoCalCoastal,
            reference_class: Some("residential_pool_socal".into()),
            confidence_threshold: 0.7,
            prompt_version: Some("1.0.0".into()),
            created_at: now,
            updated_at: now,
            completed_at: Some(now),
            expires_at: Some(now + TimeDuration::minutes(30)),
        }
    }

    #[test]
    fn expire_if_past_noops_when_future() {
        let now = OffsetDateTime::now_utc();
        let mut s = sample_session(now);
        assert!(!s.expire_if_past(now));
        assert_eq!(s.status, EstimationStatus::Completed);
    }

    #[test]
    fn expire_if_past_flips_when_past() {
        let base = OffsetDateTime::now_utc();
        let mut s = sample_session(base);
        let future = base + TimeDuration::hours(1);
        assert!(s.expire_if_past(future));
        assert_eq!(s.status, EstimationStatus::Expired);
        assert_eq!(s.updated_at, future);
    }

    #[test]
    fn expire_if_past_idempotent_once_expired() {
        let base = OffsetDateTime::now_utc();
        let mut s = sample_session(base);
        let future = base + TimeDuration::hours(1);
        assert!(s.expire_if_past(future));
        assert!(!s.expire_if_past(future + TimeDuration::hours(1)));
    }

    #[test]
    fn expire_if_past_noops_without_expires_at() {
        let now = OffsetDateTime::now_utc();
        let mut s = sample_session(now);
        s.expires_at = None;
        assert!(!s.expire_if_past(now + TimeDuration::hours(24)));
        assert_eq!(s.status, EstimationStatus::Completed);
    }
}
