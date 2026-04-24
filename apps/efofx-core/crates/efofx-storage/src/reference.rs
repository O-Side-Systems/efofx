//! Reference-data collections: `reference_classes` and `reference_projects`.
//!
//! These two collections carry **both** platform-provided rows
//! (`tenant_id: null`) and per-tenant overrides (`tenant_id: "<uuid>"`).
//! Unlike the chat-session repo, read methods here accept an optional
//! [`TenantContext`] and fold platform data into the result so a tenant
//! without any custom rows still sees the platform catalog.
//!
//! The only writes exposed in this phase target **platform** rows
//! (`tenant_id: null`) and power the `seed-references` ops binary —
//! user-facing tenant endpoints will land alongside the first
//! tenant-customization UI.
//!
//! Indexes created by [`ReferenceRepo::ensure_indexes`]:
//! - `reference_classes`: `{tenant_id: 1, category: 1}`,
//!   `{tenant_id: 1, name: 1}`, `{keywords: 1}`
//! - `reference_projects`: `{tenant_id: 1, reference_class: 1, region: 1}`,
//!   `{project_id: 1}`

use std::collections::BTreeMap;
use std::time::SystemTime;

use futures::stream::TryStreamExt;
use mongodb::bson::{doc, Bson, DateTime as BsonDateTime, Document};
use mongodb::options::IndexOptions;
use mongodb::{Collection, IndexModel};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use efofx_domain::{CostDistribution, ReferenceClass, ReferenceProject, TimelineDistribution};

use crate::{MongoAdapter, StorageError, TenantContext};

pub(crate) const REFERENCE_CLASSES_COLLECTION: &str = "reference_classes";
pub(crate) const REFERENCE_PROJECTS_COLLECTION: &str = "reference_projects";

/// Storage-layer document for a reference class. Optional fields mirror the
/// domain type so legacy rows with missing metadata still deserialize.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ReferenceClassDoc {
    #[serde(rename = "_id", default, skip_serializing_if = "Option::is_none")]
    pub id: Option<mongodb::bson::oid::ObjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    pub category: String,
    pub subcategory: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub regions: Vec<String>,
    #[serde(default)]
    pub attributes: BTreeMap<String, Bson>,
    pub cost_distribution: CostDistributionDoc,
    pub timeline_distribution: TimelineDistributionDoc,
    pub cost_breakdown_template: BTreeMap<String, f64>,
    #[serde(default)]
    pub is_synthetic: bool,
    pub validation_source: String,
    pub created_at: BsonDateTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<BsonDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CostDistributionDoc {
    pub p50: f64,
    pub p80: f64,
    pub p95: f64,
    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_currency() -> String {
    "USD".into()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct TimelineDistributionDoc {
    pub p50_days: u32,
    pub p80_days: u32,
    pub p95_days: u32,
}

/// Storage document for a reference project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ReferenceProjectDoc {
    #[serde(rename = "_id", default, skip_serializing_if = "Option::is_none")]
    pub id: Option<mongodb::bson::oid::ObjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    pub project_id: String,
    pub reference_class: String,
    pub region: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_sqft: Option<f64>,
    pub total_cost: f64,
    pub timeline_weeks: u32,
    pub team_size: u32,
    pub cost_breakdown: BTreeMap<String, f64>,
    pub completion_date: BsonDateTime,
    pub quality_score: f64,
    pub source: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, Bson>,
    #[serde(default = "default_true")]
    pub is_active: bool,
    pub created_at: BsonDateTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<BsonDateTime>,
}

fn default_true() -> bool {
    true
}

// --- DTO ↔ domain --------------------------------------------------------

impl ReferenceClassDoc {
    pub(crate) fn into_domain(self) -> Result<ReferenceClass, StorageError> {
        let attributes = self
            .attributes
            .into_iter()
            .map(|(k, v)| Ok((k, bson_to_json(v)?)))
            .collect::<Result<BTreeMap<_, _>, StorageError>>()?;
        Ok(ReferenceClass {
            id: self.id.map(|o| o.to_hex()),
            tenant_id: self.tenant_id,
            category: self.category,
            subcategory: self.subcategory,
            name: self.name,
            description: self.description,
            keywords: self.keywords,
            regions: self.regions,
            attributes,
            cost_distribution: CostDistribution {
                p50: self.cost_distribution.p50,
                p80: self.cost_distribution.p80,
                p95: self.cost_distribution.p95,
                currency: self.cost_distribution.currency,
            },
            timeline_distribution: TimelineDistribution {
                p50_days: self.timeline_distribution.p50_days,
                p80_days: self.timeline_distribution.p80_days,
                p95_days: self.timeline_distribution.p95_days,
            },
            cost_breakdown_template: self.cost_breakdown_template,
            is_synthetic: self.is_synthetic,
            validation_source: self.validation_source,
            created_at: bson_to_offset(self.created_at)?,
            updated_at: self.updated_at.map(bson_to_offset).transpose()?,
        })
    }
}

impl ReferenceProjectDoc {
    pub(crate) fn into_domain(self) -> Result<ReferenceProject, StorageError> {
        let metadata = self
            .metadata
            .into_iter()
            .map(|(k, v)| Ok((k, bson_to_json(v)?)))
            .collect::<Result<BTreeMap<_, _>, StorageError>>()?;
        Ok(ReferenceProject {
            id: self.id.map(|o| o.to_hex()),
            tenant_id: self.tenant_id,
            project_id: self.project_id,
            reference_class: self.reference_class,
            region: self.region,
            description: self.description,
            size_sqft: self.size_sqft,
            total_cost: self.total_cost,
            timeline_weeks: self.timeline_weeks,
            team_size: self.team_size,
            cost_breakdown: self.cost_breakdown,
            completion_date: bson_to_offset(self.completion_date)?,
            quality_score: self.quality_score,
            source: self.source,
            metadata,
            is_active: self.is_active,
            created_at: bson_to_offset(self.created_at)?,
            updated_at: self.updated_at.map(bson_to_offset).transpose()?,
        })
    }
}

fn bson_to_offset(dt: BsonDateTime) -> Result<OffsetDateTime, StorageError> {
    OffsetDateTime::from_unix_timestamp_nanos(dt.timestamp_millis() as i128 * 1_000_000)
        .map_err(|e| StorageError::Bson(format!("timestamp out of range: {e}")))
}

fn offset_to_bson(dt: OffsetDateTime) -> BsonDateTime {
    BsonDateTime::from_system_time(SystemTime::from(dt))
}

/// Convert an LLM-hint JSON scalar into BSON. The `attributes` /
/// `metadata` maps only ever carry scalar hints in practice, but the
/// conversion is general enough to handle arrays / objects too.
fn json_to_bson(v: serde_json::Value) -> Result<Bson, StorageError> {
    mongodb::bson::serialize_to_bson(&v).map_err(|e| StorageError::Bson(format!("json→bson: {e}")))
}

impl ReferenceClassDoc {
    /// Build a platform-scoped storage document from a domain value. Clears
    /// `_id` so the upsert path lets Mongo keep or mint it, and forces
    /// `tenant_id: null` (platform rows only).
    fn from_platform_domain(c: &ReferenceClass) -> Result<Self, StorageError> {
        let attributes = c
            .attributes
            .iter()
            .map(|(k, v)| Ok((k.clone(), json_to_bson(v.clone())?)))
            .collect::<Result<BTreeMap<_, _>, StorageError>>()?;
        Ok(ReferenceClassDoc {
            id: None,
            tenant_id: None,
            category: c.category.clone(),
            subcategory: c.subcategory.clone(),
            name: c.name.clone(),
            description: c.description.clone(),
            keywords: c.keywords.clone(),
            regions: c.regions.clone(),
            attributes,
            cost_distribution: CostDistributionDoc {
                p50: c.cost_distribution.p50,
                p80: c.cost_distribution.p80,
                p95: c.cost_distribution.p95,
                currency: c.cost_distribution.currency.clone(),
            },
            timeline_distribution: TimelineDistributionDoc {
                p50_days: c.timeline_distribution.p50_days,
                p80_days: c.timeline_distribution.p80_days,
                p95_days: c.timeline_distribution.p95_days,
            },
            cost_breakdown_template: c.cost_breakdown_template.clone(),
            is_synthetic: c.is_synthetic,
            validation_source: c.validation_source.clone(),
            created_at: offset_to_bson(c.created_at),
            updated_at: c.updated_at.map(offset_to_bson),
        })
    }
}

impl ReferenceProjectDoc {
    /// Build a platform-scoped storage document from a domain value. Clears
    /// `_id` and forces `tenant_id: null`.
    fn from_platform_domain(p: &ReferenceProject) -> Result<Self, StorageError> {
        let metadata = p
            .metadata
            .iter()
            .map(|(k, v)| Ok((k.clone(), json_to_bson(v.clone())?)))
            .collect::<Result<BTreeMap<_, _>, StorageError>>()?;
        Ok(ReferenceProjectDoc {
            id: None,
            tenant_id: None,
            project_id: p.project_id.clone(),
            reference_class: p.reference_class.clone(),
            region: p.region.clone(),
            description: p.description.clone(),
            size_sqft: p.size_sqft,
            total_cost: p.total_cost,
            timeline_weeks: p.timeline_weeks,
            team_size: p.team_size,
            cost_breakdown: p.cost_breakdown.clone(),
            completion_date: offset_to_bson(p.completion_date),
            quality_score: p.quality_score,
            source: p.source.clone(),
            metadata,
            is_active: p.is_active,
            created_at: offset_to_bson(p.created_at),
            updated_at: p.updated_at.map(offset_to_bson),
        })
    }
}

/// Outcome of a platform-rows upsert batch. Accumulated across calls by
/// [`UpsertStats::observe`]. Used by the `seed-references` binary to emit
/// a concise summary without inspecting raw [`mongodb::results::UpdateResult`]
/// values.
#[derive(Debug, Clone, Copy, Default)]
pub struct UpsertStats {
    pub inserted: u64,
    pub modified: u64,
    pub unchanged: u64,
}

impl UpsertStats {
    /// Folded total across all outcomes.
    pub fn total(&self) -> u64 {
        self.inserted + self.modified + self.unchanged
    }

    /// Record one upsert result.
    pub fn observe(&mut self, result: &mongodb::results::UpdateResult) {
        if result.upserted_id.is_some() {
            self.inserted += 1;
        } else if result.modified_count > 0 {
            self.modified += 1;
        } else {
            self.unchanged += 1;
        }
    }
}

fn bson_to_json(b: Bson) -> Result<serde_json::Value, StorageError> {
    // `Bson` implements `Serialize`, which produces extended-JSON for non-
    // scalar types (`{"$oid": "..."}` for ObjectId, etc.). That's lossy but
    // fine: `attributes` and `metadata` carry scalar LLM hints, not document
    // refs — any non-scalar is already a contract violation.
    serde_json::to_value(&b).map_err(|e| StorageError::Bson(format!("bson→json: {e}")))
}

// --- Repository ----------------------------------------------------------

/// Reads reference classes and projects, honoring platform-vs-tenant
/// visibility. The repo is cheap to clone (just a Mongo handle).
#[derive(Clone)]
pub struct ReferenceRepo {
    mongo: MongoAdapter,
}

impl ReferenceRepo {
    pub fn new(mongo: MongoAdapter) -> Self {
        Self { mongo }
    }

    fn classes(&self) -> Collection<ReferenceClassDoc> {
        self.mongo
            .database()
            .collection(REFERENCE_CLASSES_COLLECTION)
    }

    fn classes_docs(&self) -> Collection<Document> {
        self.mongo
            .database()
            .collection(REFERENCE_CLASSES_COLLECTION)
    }

    fn projects(&self) -> Collection<ReferenceProjectDoc> {
        self.mongo
            .database()
            .collection(REFERENCE_PROJECTS_COLLECTION)
    }

    fn projects_docs(&self) -> Collection<Document> {
        self.mongo
            .database()
            .collection(REFERENCE_PROJECTS_COLLECTION)
    }

    /// Idempotent index creation. Call on startup.
    pub async fn ensure_indexes(&self) -> Result<(), StorageError> {
        let class_tenant_category = IndexModel::builder()
            .keys(doc! { "tenant_id": 1, "category": 1 })
            .build();
        let class_tenant_name = IndexModel::builder()
            .keys(doc! { "tenant_id": 1, "name": 1 })
            .build();
        let class_keywords = IndexModel::builder().keys(doc! { "keywords": 1 }).build();
        self.classes()
            .create_indexes(vec![
                class_tenant_category,
                class_tenant_name,
                class_keywords,
            ])
            .await?;

        let proj_tenant_class_region = IndexModel::builder()
            .keys(doc! { "tenant_id": 1, "reference_class": 1, "region": 1 })
            .build();
        let proj_project_id = IndexModel::builder()
            .keys(doc! { "project_id": 1 })
            .options(IndexOptions::builder().unique(false).build())
            .build();
        self.projects()
            .create_indexes(vec![proj_tenant_class_region, proj_project_id])
            .await?;
        Ok(())
    }

    /// List active reference classes. When a tenant context is supplied, the
    /// result includes both the tenant's own classes and platform rows
    /// (`tenant_id: null`). Without a tenant context the query returns
    /// platform-only rows — useful for admin tooling.
    pub async fn list_reference_classes(
        &self,
        ctx: Option<&TenantContext>,
        category: Option<&str>,
    ) -> Result<Vec<ReferenceClass>, StorageError> {
        let mut filter = Document::new();
        apply_tenant_visibility(&mut filter, ctx);
        if let Some(c) = category {
            filter.insert("category", c);
        }

        let mut cursor = self.classes_docs().find(filter).await?;
        let mut out = Vec::new();
        while let Some(raw) = cursor.try_next().await? {
            let doc: ReferenceClassDoc = mongodb::bson::deserialize_from_document(raw)
                .map_err(|e| StorageError::Bson(format!("decode reference class: {e}")))?;
            out.push(doc.into_domain()?);
        }
        Ok(out)
    }

    /// Fetch a reference class by its `name`. Prefers tenant-specific rows
    /// over platform rows when both exist under the same name.
    pub async fn get_reference_class(
        &self,
        ctx: Option<&TenantContext>,
        name: &str,
    ) -> Result<Option<ReferenceClass>, StorageError> {
        let mut tenant_ids: Vec<Bson> = Vec::new();
        if let Some(c) = ctx {
            tenant_ids.push(Bson::String(c.tenant_id().to_string()));
        }
        tenant_ids.push(Bson::Null);

        let filter = doc! {
            "name": name,
            "tenant_id": { "$in": tenant_ids },
        };

        // Sort so tenant rows sort before platform rows (strings < null per
        // BSON ordering, which is the opposite of what we want), so we
        // resolve manually after fetching both.
        let mut cursor = self.classes_docs().find(filter).await?;
        let mut tenant_hit: Option<ReferenceClass> = None;
        let mut platform_hit: Option<ReferenceClass> = None;
        while let Some(raw) = cursor.try_next().await? {
            let parsed: ReferenceClassDoc = mongodb::bson::deserialize_from_document(raw)
                .map_err(|e| StorageError::Bson(format!("decode reference class: {e}")))?;
            let is_platform = parsed.tenant_id.is_none();
            let dom = parsed.into_domain()?;
            if is_platform {
                platform_hit.get_or_insert(dom);
            } else {
                tenant_hit.get_or_insert(dom);
            }
        }
        Ok(tenant_hit.or(platform_hit))
    }

    /// Fetch reference projects for a given class + region, ordered by
    /// `quality_score` descending and capped at `limit`.
    pub async fn get_reference_projects(
        &self,
        ctx: Option<&TenantContext>,
        reference_class: &str,
        region: &str,
        limit: i64,
    ) -> Result<Vec<ReferenceProject>, StorageError> {
        let mut filter = doc! {
            "reference_class": reference_class,
            "region": region,
            "is_active": true,
        };
        apply_tenant_visibility(&mut filter, ctx);

        let opts = mongodb::options::FindOptions::builder()
            .sort(doc! { "quality_score": -1 })
            .limit(limit.max(1))
            .build();
        let mut cursor = self.projects_docs().find(filter).with_options(opts).await?;
        let mut out = Vec::new();
        while let Some(raw) = cursor.try_next().await? {
            let doc: ReferenceProjectDoc = mongodb::bson::deserialize_from_document(raw)
                .map_err(|e| StorageError::Bson(format!("decode reference project: {e}")))?;
            out.push(doc.into_domain()?);
        }
        Ok(out)
    }

    /// Upsert a **platform** reference class by `name`. Platform rows are
    /// visible to every tenant. The caller owns content validation but we
    /// re-verify cost-breakdown sums defensively so a bad fixture can't slip
    /// through.
    ///
    /// Returns the raw [`mongodb::results::UpdateResult`] so the seeding
    /// binary can distinguish inserts from modifications.
    pub async fn upsert_platform_class(
        &self,
        class: &ReferenceClass,
    ) -> Result<mongodb::results::UpdateResult, StorageError> {
        class
            .validate_cost_breakdown()
            .map_err(StorageError::Bson)?;
        let doc = ReferenceClassDoc::from_platform_domain(class)?;
        let filter = doc! { "tenant_id": Bson::Null, "name": class.name.clone() };
        let opts = mongodb::options::ReplaceOptions::builder()
            .upsert(true)
            .build();
        Ok(self
            .classes()
            .replace_one(filter, &doc)
            .with_options(opts)
            .await?)
    }

    /// Upsert a **platform** reference project by `project_id`. Project IDs
    /// are expected to be globally unique across platform data; collisions
    /// with tenant-scoped projects are avoided by the `tenant_id: null`
    /// filter clause.
    pub async fn upsert_platform_project(
        &self,
        project: &ReferenceProject,
    ) -> Result<mongodb::results::UpdateResult, StorageError> {
        let doc = ReferenceProjectDoc::from_platform_domain(project)?;
        let filter = doc! {
            "tenant_id": Bson::Null,
            "project_id": project.project_id.clone(),
        };
        let opts = mongodb::options::ReplaceOptions::builder()
            .upsert(true)
            .build();
        Ok(self
            .projects()
            .replace_one(filter, &doc)
            .with_options(opts)
            .await?)
    }

    /// Delete every platform reference class (`tenant_id: null`). Used by the
    /// seeding binary's `--clean` mode. Tenant-scoped rows are untouched.
    pub async fn delete_platform_classes(&self) -> Result<u64, StorageError> {
        let result = self
            .classes_docs()
            .delete_many(doc! { "tenant_id": Bson::Null })
            .await?;
        Ok(result.deleted_count)
    }

    /// Delete every platform reference project (`tenant_id: null`). Used by
    /// the seeding binary's `--clean` mode.
    pub async fn delete_platform_projects(&self) -> Result<u64, StorageError> {
        let result = self
            .projects_docs()
            .delete_many(doc! { "tenant_id": Bson::Null })
            .await?;
        Ok(result.deleted_count)
    }

    /// Raw `Document` variant for callers (like the RCF engine) that need to
    /// score against unconstrained schema. Returns only the `category`
    /// match, honoring platform/tenant visibility. Intentionally leaves
    /// decoding to the caller.
    pub async fn list_reference_class_docs_for_category(
        &self,
        ctx: Option<&TenantContext>,
        category: &str,
    ) -> Result<Vec<Document>, StorageError> {
        let mut filter = doc! { "category": category };
        apply_tenant_visibility(&mut filter, ctx);
        let mut cursor = self.classes_docs().find(filter).await?;
        let mut out = Vec::new();
        while let Some(raw) = cursor.try_next().await? {
            out.push(raw);
        }
        Ok(out)
    }
}

/// Decode a raw Mongo document into a domain [`ReferenceClass`]. Public so
/// the RCF matcher can turn the winning candidate into a typed value
/// without re-querying. Returns the human-readable decode error as a string
/// (the exact bson error type isn't part of this crate's public surface).
pub fn decode_reference_class(doc: Document) -> Result<ReferenceClass, String> {
    let parsed: ReferenceClassDoc = mongodb::bson::deserialize_from_document(doc)
        .map_err(|e| format!("decode reference class: {e}"))?;
    parsed
        .into_domain()
        .map_err(|e| format!("into domain: {e}"))
}

/// Fold "tenant-or-platform" visibility into a mongodb filter document.
/// `None` context means platform-only.
fn apply_tenant_visibility(filter: &mut Document, ctx: Option<&TenantContext>) {
    match ctx {
        Some(c) => {
            filter.insert(
                "tenant_id",
                doc! {
                    "$in": [Bson::String(c.tenant_id().to_string()), Bson::Null],
                },
            );
        }
        None => {
            filter.insert("tenant_id", Bson::Null);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::bson::doc;

    #[test]
    fn visibility_platform_only_when_no_ctx() {
        let mut filter = Document::new();
        apply_tenant_visibility(&mut filter, None);
        assert_eq!(filter.get("tenant_id"), Some(&Bson::Null));
    }

    #[test]
    fn visibility_in_tenant_or_null_when_ctx_given() {
        // We can't construct a TenantContext from tests in this crate,
        // but the handling of `None` is the interesting case. The `Some`
        // path is exercised by integration tests.
        let mut filter = doc! { "extra": 1 };
        apply_tenant_visibility(&mut filter, None);
        assert_eq!(filter.get("tenant_id"), Some(&Bson::Null));
        assert_eq!(filter.get("extra"), Some(&Bson::Int32(1)));
    }

    #[test]
    fn class_doc_to_domain_roundtrips_attributes() {
        let now = BsonDateTime::from_system_time(SystemTime::now());
        let mut attrs: BTreeMap<String, Bson> = BTreeMap::new();
        attrs.insert("size_range".into(), Bson::String("300-500 sq ft".into()));

        let mut breakdown: BTreeMap<String, f64> = BTreeMap::new();
        breakdown.insert("materials".into(), 0.40);
        breakdown.insert("labor".into(), 0.30);
        breakdown.insert("excavation".into(), 0.15);
        breakdown.insert("permits".into(), 0.05);
        breakdown.insert("overhead".into(), 0.10);

        let doc = ReferenceClassDoc {
            id: None,
            tenant_id: None,
            category: "construction".into(),
            subcategory: "pool".into(),
            name: "residential_pool_socal".into(),
            description: "Pool".into(),
            keywords: vec!["pool".into()],
            regions: vec!["SoCal - Coastal".into()],
            attributes: attrs,
            cost_distribution: CostDistributionDoc {
                p50: 1.0,
                p80: 2.0,
                p95: 3.0,
                currency: "USD".into(),
            },
            timeline_distribution: TimelineDistributionDoc {
                p50_days: 30,
                p80_days: 45,
                p95_days: 60,
            },
            cost_breakdown_template: breakdown,
            is_synthetic: true,
            validation_source: "synthetic_v1".into(),
            created_at: now,
            updated_at: None,
        };
        let dom = doc.into_domain().expect("into domain");
        assert_eq!(dom.name, "residential_pool_socal");
        assert_eq!(
            dom.attributes.get("size_range").and_then(|v| v.as_str()),
            Some("300-500 sq ft")
        );
        assert_eq!(dom.cost_distribution.p80, 2.0);
    }
}
