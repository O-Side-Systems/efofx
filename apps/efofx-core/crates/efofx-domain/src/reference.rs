//! Reference data types used by RCF matching and the estimation flow.
//!
//! The shape mirrors FastAPI's `app/models/reference_class.py` (the newer,
//! domain-agnostic model) rather than the older `app/models/reference.py`.
//! The two coexisted in FastAPI; Rust unifies on the newer schema because
//! it's what the RCF engine consumes and it's the more coherent of the two.
//!
//! Wire formats match FastAPI byte-for-byte so any legacy documents remain
//! readable if a migration ever lands.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;

/// Probabilistic cost distribution for a reference class (or a single
/// reference project). Currency is documented separately so a future mix of
/// regions / denominations can be supported without schema churn.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CostDistribution {
    pub p50: f64,
    pub p80: f64,
    pub p95: f64,
    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_currency() -> String {
    "USD".to_string()
}

/// Probabilistic timeline distribution expressed in calendar days.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub struct TimelineDistribution {
    pub p50_days: u32,
    pub p80_days: u32,
    pub p95_days: u32,
}

/// A reference class: pattern of past projects used to ground an estimate.
/// Platform-provided rows carry `tenant_id: None`; tenant-scoped additions
/// carry their owning tenant's UUID as a string.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReferenceClass {
    /// Mongo ObjectId stringified. Optional so newly constructed records can
    /// round-trip without inventing one; the repo fills it after insert.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// `None` = platform-provided and visible to every tenant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,

    /// Domain category, e.g. `"construction"` or `"it_dev"`. Free-form string
    /// by design — see the module docstring.
    pub category: String,
    /// Sub-classification within the domain, e.g. `"pool"`,
    /// `"api_development"`.
    pub subcategory: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Applicable regions. Stored as strings so both the display-style labels
    /// (`"SoCal - Coastal"`) and canonical tags (`"us-ca-south"`) can
    /// coexist across datasets.
    #[serde(default)]
    pub regions: Vec<String>,

    /// Arbitrary domain-specific hints (e.g. size ranges, tech stack). The
    /// estimation pipeline treats this as opaque context for the LLM.
    #[serde(default)]
    pub attributes: BTreeMap<String, serde_json::Value>,

    pub cost_distribution: CostDistribution,
    pub timeline_distribution: TimelineDistribution,

    /// Percentages on 0.0–1.0 summing (approximately) to 1.0. Validated on
    /// ingest via [`ReferenceClass::validate_cost_breakdown`].
    pub cost_breakdown_template: BTreeMap<String, f64>,

    #[serde(default)]
    pub is_synthetic: bool,
    pub validation_source: String,

    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub updated_at: Option<OffsetDateTime>,
}

impl ReferenceClass {
    /// `true` if the breakdown percentages sum to 1.0 within a 1% tolerance.
    /// Mirrors FastAPI's `validate_cost_breakdown_sum` field validator.
    pub fn validate_cost_breakdown(&self) -> Result<(), String> {
        let total: f64 = self.cost_breakdown_template.values().sum();
        if (total - 1.0).abs() > 0.01 {
            return Err(format!(
                "cost breakdown percentages must sum to 1.0 (within 0.01); got {total:.4}"
            ));
        }
        Ok(())
    }
}

/// A single real / synthetic reference project. Fed to the LLM as grounding
/// data during structured estimation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReferenceProject {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// `None` = platform-provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,

    /// Stable application-level identifier (distinct from `_id`).
    pub project_id: String,
    /// Name of the associated [`ReferenceClass`] (`ReferenceClass::name`).
    pub reference_class: String,
    pub region: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_sqft: Option<f64>,
    pub total_cost: f64,
    pub timeline_weeks: u32,
    pub team_size: u32,
    pub cost_breakdown: BTreeMap<String, f64>,
    #[serde(with = "time::serde::rfc3339")]
    pub completion_date: OffsetDateTime,
    /// 0.0–1.0.
    pub quality_score: f64,
    pub source: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub updated_at: Option<OffsetDateTime>,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_class() -> ReferenceClass {
        let mut breakdown = BTreeMap::new();
        breakdown.insert("materials".into(), 0.40);
        breakdown.insert("labor".into(), 0.30);
        breakdown.insert("excavation".into(), 0.15);
        breakdown.insert("permits".into(), 0.05);
        breakdown.insert("overhead".into(), 0.10);
        ReferenceClass {
            id: Some("507f1f77bcf86cd799439011".into()),
            tenant_id: None,
            category: "construction".into(),
            subcategory: "pool".into(),
            name: "residential_pool_socal".into(),
            description: "Standard residential swimming pool, SoCal.".into(),
            keywords: vec!["pool".into(), "swimming".into(), "residential".into()],
            regions: vec!["SoCal - Coastal".into(), "SoCal - Inland".into()],
            attributes: {
                let mut m = BTreeMap::new();
                m.insert("size_range".into(), json!("300-500 sq ft"));
                m
            },
            cost_distribution: CostDistribution {
                p50: 55_000.0,
                p80: 72_000.0,
                p95: 95_000.0,
                currency: "USD".into(),
            },
            timeline_distribution: TimelineDistribution {
                p50_days: 45,
                p80_days: 60,
                p95_days: 90,
            },
            cost_breakdown_template: breakdown,
            is_synthetic: true,
            validation_source: "synthetic_generator_v1".into(),
            created_at: OffsetDateTime::now_utc(),
            updated_at: None,
        }
    }

    #[test]
    fn cost_breakdown_validates_within_tolerance() {
        let rc = sample_class();
        rc.validate_cost_breakdown().expect("sums to 1.0");
    }

    #[test]
    fn cost_breakdown_rejects_off_by_more_than_one_percent() {
        let mut rc = sample_class();
        rc.cost_breakdown_template.insert("labor".into(), 0.25); // totals to 0.95
        let err = rc.validate_cost_breakdown().unwrap_err();
        assert!(err.contains("sum to 1.0"), "got {err}");
    }

    #[test]
    fn reference_class_round_trips_through_json() {
        let rc = sample_class();
        let json = serde_json::to_string(&rc).unwrap();
        let back: ReferenceClass = serde_json::from_str(&json).unwrap();
        assert_eq!(back.category, rc.category);
        assert_eq!(back.subcategory, rc.subcategory);
        assert_eq!(back.cost_distribution.p50, rc.cost_distribution.p50);
        assert_eq!(
            back.timeline_distribution.p80_days,
            rc.timeline_distribution.p80_days
        );
        assert_eq!(back.cost_breakdown_template.len(), 5);
    }

    #[test]
    fn cost_distribution_defaults_currency() {
        let raw = r#"{"p50":1.0,"p80":2.0,"p95":3.0}"#;
        let cd: CostDistribution = serde_json::from_str(raw).unwrap();
        assert_eq!(cd.currency, "USD");
    }

    #[test]
    fn reference_project_round_trips() {
        let mut breakdown = BTreeMap::new();
        breakdown.insert("materials".into(), 25_000.0);
        breakdown.insert("labor".into(), 15_000.0);
        let p = ReferenceProject {
            id: None,
            tenant_id: None,
            project_id: "pool_001".into(),
            reference_class: "residential_pool_socal".into(),
            region: "SoCal - Coastal".into(),
            description: "15x30 pool with spa".into(),
            size_sqft: Some(450.0),
            total_cost: 63_000.0,
            timeline_weeks: 8,
            team_size: 4,
            cost_breakdown: breakdown,
            completion_date: OffsetDateTime::now_utc(),
            quality_score: 0.9,
            source: "internal_database".into(),
            metadata: BTreeMap::new(),
            is_active: true,
            created_at: OffsetDateTime::now_utc(),
            updated_at: None,
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: ReferenceProject = serde_json::from_str(&json).unwrap();
        assert_eq!(back.project_id, p.project_id);
        assert_eq!(back.cost_breakdown.get("labor").copied(), Some(15_000.0));
    }
}
