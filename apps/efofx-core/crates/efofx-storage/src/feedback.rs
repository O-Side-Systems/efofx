//! Feedback collection: storage DTOs + tenant-scoped repository.
//!
//! Two shapes co-exist here for FastAPI Mongo parity:
//!
//! * The **JSON-rating** doc — written by `POST /v1/feedback`. Mirrors
//!   FastAPI's `Feedback` / `FeedbackCreate`: a free-text `feedback_type`,
//!   a 1–5 `rating`, and optional accuracy scores 0..=1.
//! * The **magic-link snapshot** doc — written by
//!   `POST /v1/feedback/forms/{token}` after a customer submits the form.
//!   Mirrors FastAPI's `FeedbackDocument`: actual cost / timeline,
//!   discrepancy reasons, plus an immutable [`EstimateSnapshotDoc`] copy
//!   of the estimate at submission time.
//!
//! Both shapes share the `feedback` collection. Calibration (Phase 2E.5)
//! reads the magic-link variant — the JSON-rating variant has no
//! `estimate_snapshot` and therefore contributes nothing to variance
//! aggregation.
//!
//! Field naming differs from the Rust domain types where FastAPI history
//! pins the wire form (e.g. `actual_timeline` here, not the Rust public
//! API's `actual_timeline_weeks`). All deviations are explicit
//! `#[serde(rename = ...)]` calls so a future schema migration is a
//! one-line change per field.

use std::collections::HashMap;
use std::time::SystemTime;

use mongodb::bson::{doc, DateTime as BsonDateTime, Document};
use mongodb::options::IndexOptions;
use mongodb::IndexModel;
use serde::{Deserialize, Serialize};

use efofx_domain::{CostCategoryEstimate, DiscrepancyReason};

use crate::{MongoAdapter, StorageError, TenantContext};

pub(crate) const FEEDBACK_COLLECTION: &str = "feedback";

/// JSON-rating insert payload — `POST /v1/feedback` body after validation.
#[derive(Debug, Clone)]
pub struct NewFeedback {
    /// FastAPI uses free-form `sess_xxxxxxxxxxxx`; we keep it as a string
    /// so this repo doesn't need to know about the estimation id format.
    pub estimation_session_id: String,
    /// Free-text category: `"accuracy"`, `"cost"`, `"timeline"`, …
    pub feedback_type: String,
    pub rating: u8,
    pub comment: Option<String>,
    pub actual_cost: Option<f64>,
    /// Weeks. Persisted under the FastAPI field name `actual_timeline`.
    pub actual_timeline_weeks: Option<u32>,
    pub actual_team_size: Option<u32>,
    /// 0.0–1.0 cost accuracy score, if the caller computed one.
    pub cost_accuracy: Option<f64>,
    pub timeline_accuracy: Option<f64>,
    pub reference_class_accuracy: Option<f64>,
}

/// Magic-link insert payload — `POST /v1/feedback/forms/{token}` body
/// after the form has been validated and the snapshot built.
#[derive(Debug, Clone)]
pub struct NewFeedbackWithSnapshot {
    pub estimation_session_id: String,
    pub reference_class_id: Option<String>,
    pub actual_cost: f64,
    pub actual_timeline_weeks: u32,
    pub rating: u8,
    pub discrepancy_reason_primary: DiscrepancyReason,
    pub discrepancy_reason_secondary: Option<DiscrepancyReason>,
    pub comment: Option<String>,
    pub estimate_snapshot: EstimateSnapshotDoc,
}

/// Mongo wire shape for the embedded snapshot. Mirrors FastAPI's
/// `EstimateSnapshot`: snake-case fields, `cost_breakdown` as raw JSON
/// objects so we don't lock the schema to a specific sub-type version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateSnapshotDoc {
    pub total_cost_p50: f64,
    pub total_cost_p80: f64,
    pub timeline_weeks_p50: u32,
    pub timeline_weeks_p80: u32,
    pub cost_breakdown: Vec<CostCategoryEstimate>,
    pub assumptions: Vec<String>,
    pub confidence_score: f64,
}

/// Persisted feedback document — supports both the JSON-rating and
/// magic-link shapes via `#[serde(default)]` on every variant-only field.
/// Reads are tolerant; writes always populate the variant-correct subset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FeedbackDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none", default)]
    pub id: Option<mongodb::bson::oid::ObjectId>,
    pub tenant_id: String,
    pub estimation_session_id: String,

    // -------- JSON-rating variant fields --------
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback_type: Option<String>,
    pub rating: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_cost: Option<f64>,
    /// FastAPI field name. Rust public DTOs render this as
    /// `actual_timeline_weeks`; only the storage layer touches the bare
    /// `actual_timeline` form.
    #[serde(
        rename = "actual_timeline",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub actual_timeline_weeks: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_team_size: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_accuracy: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_accuracy: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_class_accuracy: Option<f64>,

    // -------- Magic-link variant fields --------
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_class_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discrepancy_reason_primary: Option<DiscrepancyReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discrepancy_reason_secondary: Option<DiscrepancyReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimate_snapshot: Option<EstimateSnapshotDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submitted_at: Option<BsonDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<u32>,

    // -------- Shared timestamps --------
    pub created_at: BsonDateTime,
    pub updated_at: BsonDateTime,
}

/// Aggregate stats returned by [`FeedbackRepo::summary`]. Only the bits
/// the dashboard renders today; new fields are additive.
#[derive(Debug, Clone, Default)]
pub struct FeedbackSummaryStats {
    pub total_feedback: u64,
    /// `0.0` when there are no rows (FastAPI parity — not `None`).
    pub average_rating: f64,
    pub cost_accuracy_avg: Option<f64>,
    pub timeline_accuracy_avg: Option<f64>,
    pub reference_class_accuracy_avg: Option<f64>,
    /// `feedback_type` → count. Includes only the JSON-rating shape;
    /// magic-link snapshot rows have no `feedback_type` and are skipped.
    pub feedback_by_type: HashMap<String, u64>,
}

/// Tenant-scoped repository for the `feedback` collection.
#[derive(Clone)]
pub struct FeedbackRepo {
    mongo: MongoAdapter,
}

impl FeedbackRepo {
    pub fn new(mongo: MongoAdapter) -> Self {
        Self { mongo }
    }

    fn collection(&self) -> mongodb::Collection<FeedbackDoc> {
        self.mongo.database().collection(FEEDBACK_COLLECTION)
    }

    fn docs_collection(&self) -> mongodb::Collection<Document> {
        self.mongo.database().collection(FEEDBACK_COLLECTION)
    }

    /// Idempotent index creation. Call on startup.
    pub async fn ensure_indexes(&self) -> Result<(), StorageError> {
        let by_tenant_created = IndexModel::builder()
            .keys(doc! { "tenant_id": 1, "created_at": -1 })
            .options(
                IndexOptions::builder()
                    .name("feedback__tenant_created_idx".to_string())
                    .build(),
            )
            .build();
        let by_tenant_session = IndexModel::builder()
            .keys(doc! { "tenant_id": 1, "estimation_session_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("feedback__tenant_session_idx".to_string())
                    .build(),
            )
            .build();
        self.collection()
            .create_indexes(vec![by_tenant_created, by_tenant_session])
            .await?;
        Ok(())
    }

    /// Insert a JSON-rating feedback row. Returns the generated id as a
    /// 24-char hex string for parity with FastAPI's `str(inserted_id)`.
    pub async fn insert_basic(
        &self,
        ctx: &TenantContext,
        new: &NewFeedback,
    ) -> Result<String, StorageError> {
        let now = now_bson();
        let doc = FeedbackDoc {
            id: None,
            tenant_id: ctx.tenant_id().to_string(),
            estimation_session_id: new.estimation_session_id.clone(),
            feedback_type: Some(new.feedback_type.clone()),
            rating: new.rating,
            comment: new.comment.clone(),
            actual_cost: new.actual_cost,
            actual_timeline_weeks: new.actual_timeline_weeks,
            actual_team_size: new.actual_team_size,
            cost_accuracy: new.cost_accuracy,
            timeline_accuracy: new.timeline_accuracy,
            reference_class_accuracy: new.reference_class_accuracy,
            reference_class_id: None,
            discrepancy_reason_primary: None,
            discrepancy_reason_secondary: None,
            estimate_snapshot: None,
            submitted_at: None,
            schema_version: None,
            created_at: now,
            updated_at: now,
        };
        let result = self.collection().insert_one(&doc).await?;
        Ok(inserted_id_hex(result.inserted_id))
    }

    /// Insert a magic-link feedback row with an immutable estimate
    /// snapshot. `submitted_at` and `schema_version` are populated to
    /// match the FastAPI `FeedbackDocument` shape exactly.
    pub async fn insert_with_snapshot(
        &self,
        tenant_id: &str,
        new: NewFeedbackWithSnapshot,
    ) -> Result<String, StorageError> {
        let now = now_bson();
        let doc = FeedbackDoc {
            id: None,
            tenant_id: tenant_id.to_string(),
            estimation_session_id: new.estimation_session_id,
            feedback_type: None,
            rating: new.rating,
            comment: new.comment,
            actual_cost: Some(new.actual_cost),
            actual_timeline_weeks: Some(new.actual_timeline_weeks),
            actual_team_size: None,
            cost_accuracy: None,
            timeline_accuracy: None,
            reference_class_accuracy: None,
            reference_class_id: new.reference_class_id,
            discrepancy_reason_primary: Some(new.discrepancy_reason_primary),
            discrepancy_reason_secondary: new.discrepancy_reason_secondary,
            estimate_snapshot: Some(new.estimate_snapshot),
            submitted_at: Some(now),
            schema_version: Some(1),
            created_at: now,
            updated_at: now,
        };
        let result = self.collection().insert_one(&doc).await?;
        Ok(inserted_id_hex(result.inserted_id))
    }

    /// Compute aggregate summary stats for the authenticated tenant.
    /// Mirrors FastAPI's `FeedbackService.get_feedback_summary` minus
    /// the `recent_feedback` list — that field is a candidate for
    /// re-introduction when the dashboard asks for it.
    pub async fn summary(
        &self,
        ctx: &TenantContext,
    ) -> Result<FeedbackSummaryStats, StorageError> {
        use futures::stream::TryStreamExt;

        let cursor = self
            .docs_collection()
            .find(doc! { "tenant_id": ctx.tenant_id().to_string() })
            .await?;
        let rows: Vec<Document> = cursor.try_collect().await?;

        if rows.is_empty() {
            return Ok(FeedbackSummaryStats::default());
        }

        let total = rows.len() as u64;
        let mut rating_sum: f64 = 0.0;
        let mut cost_acc: Vec<f64> = Vec::new();
        let mut time_acc: Vec<f64> = Vec::new();
        let mut rc_acc: Vec<f64> = Vec::new();
        let mut by_type: HashMap<String, u64> = HashMap::new();

        for row in &rows {
            if let Ok(r) = row.get_i32("rating") {
                rating_sum += r as f64;
            } else if let Ok(r) = row.get_i64("rating") {
                rating_sum += r as f64;
            }
            if let Ok(v) = row.get_f64("cost_accuracy") {
                cost_acc.push(v);
            }
            if let Ok(v) = row.get_f64("timeline_accuracy") {
                time_acc.push(v);
            }
            if let Ok(v) = row.get_f64("reference_class_accuracy") {
                rc_acc.push(v);
            }
            if let Ok(t) = row.get_str("feedback_type") {
                *by_type.entry(t.to_string()).or_insert(0) += 1;
            }
        }

        Ok(FeedbackSummaryStats {
            total_feedback: total,
            average_rating: rating_sum / (total as f64),
            cost_accuracy_avg: avg(&cost_acc),
            timeline_accuracy_avg: avg(&time_acc),
            reference_class_accuracy_avg: avg(&rc_acc),
            feedback_by_type: by_type,
        })
    }
}

fn avg(xs: &[f64]) -> Option<f64> {
    if xs.is_empty() {
        None
    } else {
        Some(xs.iter().sum::<f64>() / xs.len() as f64)
    }
}

fn inserted_id_hex(id: mongodb::bson::Bson) -> String {
    id.as_object_id()
        .map(|oid| oid.to_hex())
        .unwrap_or_else(|| id.to_string())
}

fn now_bson() -> BsonDateTime {
    BsonDateTime::from_system_time(SystemTime::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_doc_round_trip_renames_actual_timeline() {
        let now = now_bson();
        let doc = FeedbackDoc {
            id: None,
            tenant_id: "tid".into(),
            estimation_session_id: "sess_abc".into(),
            feedback_type: Some("accuracy".into()),
            rating: 4,
            comment: None,
            actual_cost: Some(50_000.0),
            actual_timeline_weeks: Some(8),
            actual_team_size: None,
            cost_accuracy: None,
            timeline_accuracy: None,
            reference_class_accuracy: None,
            reference_class_id: None,
            discrepancy_reason_primary: None,
            discrepancy_reason_secondary: None,
            estimate_snapshot: None,
            submitted_at: None,
            schema_version: None,
            created_at: now,
            updated_at: now,
        };

        let bson = mongodb::bson::serialize_to_document(&doc).unwrap();
        // Wire field name is FastAPI's `actual_timeline`, not the Rust
        // public-API `actual_timeline_weeks`.
        assert!(
            bson.contains_key("actual_timeline"),
            "missing actual_timeline: {bson:?}"
        );
        assert!(!bson.contains_key("actual_timeline_weeks"));

        let back: FeedbackDoc = mongodb::bson::deserialize_from_document(bson).unwrap();
        assert_eq!(back.actual_timeline_weeks, Some(8));
        assert_eq!(back.feedback_type.as_deref(), Some("accuracy"));
    }

    #[test]
    fn snapshot_doc_round_trip_preserves_discrepancy_reasons() {
        let now = now_bson();
        let snap = EstimateSnapshotDoc {
            total_cost_p50: 50_000.0,
            total_cost_p80: 65_000.0,
            timeline_weeks_p50: 6,
            timeline_weeks_p80: 9,
            cost_breakdown: vec![],
            assumptions: vec!["dry weather".into()],
            confidence_score: 0.78,
        };
        let doc = FeedbackDoc {
            id: None,
            tenant_id: "tid".into(),
            estimation_session_id: "sess_xyz".into(),
            feedback_type: None,
            rating: 5,
            comment: Some("nailed it".into()),
            actual_cost: Some(52_000.0),
            actual_timeline_weeks: Some(7),
            actual_team_size: None,
            cost_accuracy: None,
            timeline_accuracy: None,
            reference_class_accuracy: None,
            reference_class_id: Some("residential_pool_socal".into()),
            discrepancy_reason_primary: Some(DiscrepancyReason::ScopeChanged),
            discrepancy_reason_secondary: Some(DiscrepancyReason::TimelinePressure),
            estimate_snapshot: Some(snap),
            submitted_at: Some(now),
            schema_version: Some(1),
            created_at: now,
            updated_at: now,
        };

        let bson = mongodb::bson::serialize_to_document(&doc).unwrap();
        let back: FeedbackDoc = mongodb::bson::deserialize_from_document(bson).unwrap();
        assert_eq!(
            back.discrepancy_reason_primary,
            Some(DiscrepancyReason::ScopeChanged)
        );
        assert_eq!(
            back.discrepancy_reason_secondary,
            Some(DiscrepancyReason::TimelinePressure)
        );
        assert!(back.estimate_snapshot.is_some());
    }

    #[test]
    fn avg_handles_empty_and_populated() {
        assert_eq!(avg(&[]), None);
        assert_eq!(avg(&[1.0, 2.0, 3.0]), Some(2.0));
    }
}
