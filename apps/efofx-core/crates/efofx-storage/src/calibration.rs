//! Calibration aggregations over the `feedback` collection.
//!
//! This module owns the Mongo aggregation pipelines that back
//! `GET /v1/calibration/metrics` and `GET /v1/calibration/trend`. The
//! pipeline stages mirror FastAPI's `CalibrationService` byte-for-byte
//! so a Rust roll-out doesn't change calibration numbers.
//!
//! ## Tenant scoping invariants (CALB-04)
//!
//! Mongo aggregation does **not** propagate the outer collection's
//! tenant filter into `$lookup` inner pipelines — only the source
//! collection match is scoped. The pipelines below therefore add a
//! manual `tenant_id` clause to the lookup, allowing platform-shared
//! reference classes (`tenant_id == null`) to match while excluding
//! every other tenant. Identical to FastAPI's CALB-04 guard.
//!
//! ## Synthetic exclusion (CALB-01)
//!
//! Reference classes seeded as `data_source = "synthetic"` are
//! development fixtures, not real industry data. They are excluded
//! from the lookup so calibration variance never reflects synthetic
//! baselines.

use std::time::SystemTime;

use futures::stream::TryStreamExt;
use mongodb::bson::{doc, DateTime as BsonDateTime, Document};
use serde::{Deserialize, Serialize};

use crate::feedback::FEEDBACK_COLLECTION;
use crate::{MongoAdapter, StorageError, TenantContext};

/// Reference-class group result from the metrics pipeline.
///
/// `id` is the (free-form) `reference_class_id` string the customer
/// submitted — `None` when the feedback row had no reference class. The
/// FastAPI service treats `None` as the literal label `"Unknown"`; the
/// Rust API layer applies the same mapping.
#[derive(Debug, Clone)]
pub struct CalibrationRcGroup {
    pub id: Option<String>,
    /// Per-feedback variance percentages within this RC. Each entry is
    /// `abs(actual - estimate_p50) / actual * 100`.
    pub variances: Vec<f64>,
    /// Mongo-counted outcomes — equal to `variances.len()` in
    /// well-formed data, but kept separate to mirror the FastAPI
    /// `outcome_count` field exactly.
    pub outcome_count: u64,
}

/// Single point in the monthly trend time-series.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationTrendPoint {
    /// `YYYY-MM` calendar period.
    pub period: String,
    /// One-decimal-rounded mean variance percentage for the period.
    pub mean_variance_pct: f64,
    pub outcome_count: u64,
}

/// Date filter for the metrics pipeline. Builders convert wire strings
/// into one of these variants; pipelines pattern-match.
#[derive(Debug, Clone, Copy)]
pub enum DateFilter {
    /// No date filter — count and aggregate every outcome.
    All,
    /// `submitted_at >= now() - duration`. Days lifted from FastAPI:
    /// `6months -> 182`, `1year -> 365`.
    SinceDays(i64),
}

impl DateFilter {
    fn to_match_doc(self) -> Option<Document> {
        let days = match self {
            DateFilter::All => return None,
            DateFilter::SinceDays(d) => d,
        };
        Some(doc! { "submitted_at": { "$gte": days_ago_bson(days) } })
    }
}

fn days_ago_bson(days: i64) -> BsonDateTime {
    let secs = (days as u64).saturating_mul(86_400);
    let target = SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(secs))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    BsonDateTime::from_system_time(target)
}

/// Repository wrapping the calibration aggregation surface.
#[derive(Clone)]
pub struct CalibrationRepo {
    mongo: MongoAdapter,
}

impl CalibrationRepo {
    pub fn new(mongo: MongoAdapter) -> Self {
        Self { mongo }
    }

    fn collection(&self) -> mongodb::Collection<Document> {
        self.mongo.database().collection(FEEDBACK_COLLECTION)
    }

    /// Count real outcomes for the threshold check (CALB-03). The
    /// optional `date_filter` matches the metrics window; for the
    /// trend-endpoint threshold use [`DateFilter::All`] — FastAPI
    /// counts every outcome regardless of the trend window.
    pub async fn count_outcomes(
        &self,
        ctx: &TenantContext,
        date_filter: DateFilter,
    ) -> Result<u64, StorageError> {
        let mut filter = doc! { "tenant_id": ctx.tenant_id().to_string() };
        if let Some(d) = date_filter.to_match_doc() {
            for (k, v) in d {
                filter.insert(k, v);
            }
        }
        let n = self.collection().count_documents(filter).await?;
        Ok(n)
    }

    /// Run the per-RC variance pipeline. Mirrors FastAPI's
    /// `CalibrationService._build_pipeline`. Returns one row per
    /// `reference_class_id` (including `None` for missing-RC feedback
    /// — the API layer renders these as `"Unknown"`).
    pub async fn metrics(
        &self,
        ctx: &TenantContext,
        date_filter: DateFilter,
    ) -> Result<Vec<CalibrationRcGroup>, StorageError> {
        let tenant_id = ctx.tenant_id().to_string();

        let mut pipeline: Vec<Document> = Vec::new();

        // Source-collection tenant filter. FastAPI relies on
        // `TenantAwareCollection` for this; the Rust storage layer is
        // explicit so tenant scoping is greppable.
        pipeline.push(doc! { "$match": { "tenant_id": tenant_id.clone() } });

        if let Some(d) = date_filter.to_match_doc() {
            pipeline.push(doc! { "$match": d });
        }

        // CALB-04 lookup: explicit `tenant_id` filter inside the inner
        // pipeline so platform-shared (`null`) and own-tenant rows
        // both join, and other tenants stay isolated.
        // CALB-01: synthetic reference classes never contribute.
        pipeline.push(doc! {
            "$lookup": {
                "from": "reference_classes",
                "let": {
                    "rc_id": "$reference_class_id",
                    "tenant": tenant_id.clone(),
                },
                "pipeline": [
                    {
                        "$match": {
                            "$expr": {
                                "$and": [
                                    { "$eq": ["$name", "$$rc_id"] },
                                    {
                                        "$or": [
                                            { "$eq": ["$tenant_id", "$$tenant"] },
                                            { "$eq": ["$tenant_id", null] },
                                        ]
                                    },
                                    { "$ne": ["$data_source", "synthetic"] },
                                ]
                            }
                        }
                    }
                ],
                "as": "reference_class_doc",
            }
        });

        pipeline.push(doc! {
            "$unwind": {
                "path": "$reference_class_doc",
                "preserveNullAndEmptyArrays": true,
            }
        });

        pipeline.push(doc! {
            "$group": {
                "_id": "$reference_class_id",
                "variances": {
                    "$push": {
                        "$multiply": [
                            {
                                "$divide": [
                                    {
                                        "$abs": {
                                            "$subtract": [
                                                "$actual_cost",
                                                "$estimate_snapshot.total_cost_p50",
                                            ]
                                        }
                                    },
                                    "$actual_cost",
                                ]
                            },
                            100,
                        ]
                    }
                },
                "outcome_count": { "$sum": 1 },
            }
        });

        let cursor = self.collection().aggregate(pipeline).await?;
        let rows: Vec<Document> = cursor.try_collect().await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let id = match row.get("_id") {
                Some(mongodb::bson::Bson::String(s)) => Some(s.clone()),
                _ => None,
            };
            let variances: Vec<f64> = match row.get_array("variances") {
                Ok(arr) => arr
                    .iter()
                    .filter_map(|b| match b {
                        mongodb::bson::Bson::Double(v) => Some(*v),
                        mongodb::bson::Bson::Int32(v) => Some(*v as f64),
                        mongodb::bson::Bson::Int64(v) => Some(*v as f64),
                        _ => None,
                    })
                    .collect(),
                Err(_) => Vec::new(),
            };
            let outcome_count = row
                .get_i64("outcome_count")
                .or_else(|_| row.get_i32("outcome_count").map(|v| v as i64))
                .unwrap_or(variances.len() as i64)
                .max(0) as u64;

            out.push(CalibrationRcGroup {
                id,
                variances,
                outcome_count,
            });
        }

        Ok(out)
    }

    /// Run the monthly trend pipeline. Mirrors
    /// `CalibrationService._build_trend_pipeline`. Returns sorted
    /// `(period, mean_variance_pct, outcome_count)` rows for the
    /// `since_days`-bounded lookback window. Days, not calendar
    /// months, matches FastAPI's `timedelta(days=months * 30)` math.
    pub async fn trend(
        &self,
        ctx: &TenantContext,
        since_days: i64,
    ) -> Result<Vec<CalibrationTrendPoint>, StorageError> {
        let tenant_id = ctx.tenant_id().to_string();
        let since = days_ago_bson(since_days);

        let pipeline = vec![
            doc! {
                "$match": {
                    "tenant_id": tenant_id,
                    "submitted_at": { "$gte": since },
                }
            },
            doc! {
                "$project": {
                    "variance_pct": {
                        "$multiply": [
                            {
                                "$divide": [
                                    {
                                        "$abs": {
                                            "$subtract": [
                                                "$actual_cost",
                                                "$estimate_snapshot.total_cost_p50",
                                            ]
                                        }
                                    },
                                    "$actual_cost",
                                ]
                            },
                            100,
                        ]
                    },
                    "period": {
                        "$dateToString": {
                            "format": "%Y-%m",
                            "date": "$submitted_at",
                        }
                    },
                }
            },
            doc! {
                "$group": {
                    "_id": "$period",
                    "mean_variance_pct": { "$avg": "$variance_pct" },
                    "outcome_count": { "$sum": 1 },
                }
            },
            doc! { "$sort": { "_id": 1 } },
            doc! {
                "$project": {
                    "_id": 0,
                    "period": "$_id",
                    "mean_variance_pct": { "$round": ["$mean_variance_pct", 1] },
                    "outcome_count": 1,
                }
            },
        ];

        let cursor = self.collection().aggregate(pipeline).await?;
        let rows: Vec<Document> = cursor.try_collect().await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let period = row.get_str("period").unwrap_or("").to_string();
            let mean = row
                .get_f64("mean_variance_pct")
                .or_else(|_| row.get_i32("mean_variance_pct").map(|v| v as f64))
                .or_else(|_| row.get_i64("mean_variance_pct").map(|v| v as f64))
                .unwrap_or(0.0);
            let outcome_count = row
                .get_i64("outcome_count")
                .or_else(|_| row.get_i32("outcome_count").map(|v| v as i64))
                .unwrap_or(0)
                .max(0) as u64;
            out.push(CalibrationTrendPoint {
                period,
                mean_variance_pct: mean,
                outcome_count,
            });
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_filter_all_emits_no_match() {
        assert!(DateFilter::All.to_match_doc().is_none());
    }

    #[test]
    fn date_filter_since_days_emits_gte() {
        let d = DateFilter::SinceDays(30).to_match_doc().expect("doc");
        assert!(d.contains_key("submitted_at"));
    }
}
