use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;

use crate::geo::Region;

/// Top-level reference-class taxonomy. Mirrors
/// `efofx_shared.core.constants.ReferenceClassCategory`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceClassCategory {
    Residential,
    Commercial,
    Industrial,
    Infrastructure,
    Landscaping,
    Renovation,
    NewConstruction,
}

/// A reference class: pattern of past projects used to ground an estimate.
///
/// v1 keeps this lean — seeded distributions and breakdown templates live in
/// the storage layer as loosely typed maps (mirrors the FastAPI model). The
/// domain type exposes the fields the estimation engine and dashboard need.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReferenceClass {
    /// Opaque string id (Mongo ObjectId stringified, or UUID on seed).
    pub id: String,
    pub name: String,
    pub category: ReferenceClassCategory,
    pub description: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub regions: Vec<Region>,
    /// Timeline multiplier applied to the base estimate (1.0 = unchanged).
    #[serde(default = "default_timeline_multiplier")]
    pub timeline_multiplier: f64,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

fn default_timeline_multiplier() -> f64 {
    1.0
}
fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_new_construction_snake_case() {
        assert_eq!(
            serde_json::to_string(&ReferenceClassCategory::NewConstruction).unwrap(),
            "\"new_construction\""
        );
    }
}
