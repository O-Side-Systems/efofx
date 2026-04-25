//! Widget collections: leads and analytics.
//!
//! Both repos are tenant-scoped — every read and write threads
//! [`TenantContext`] through to a `tenant_id` filter. The DTO shapes
//! mirror FastAPI's `widget_leads` and `widget_analytics` documents so a
//! migration would be a no-op.

use std::time::SystemTime;

use mongodb::bson::{doc, DateTime as BsonDateTime};
use mongodb::options::IndexOptions;
use mongodb::IndexModel;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use efofx_domain::{Lead, LeadStatus, SessionId, TenantId};

use crate::{MongoAdapter, StorageError, TenantContext};

pub(crate) const WIDGET_LEADS_COLLECTION: &str = "widget_leads";

pub(crate) const WIDGET_ANALYTICS_COLLECTION: &str = "widget_analytics";

/// Storage-layer lead document. Mirrors FastAPI's `widget_leads`
/// schema: tenant + session id + contact details + optional
/// consultation-only fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LeadDoc {
    /// Mongo `_id` as a 24-char hex string. Always present after read;
    /// inserts let the driver allocate one.
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none", default)]
    pub id: Option<mongodb::bson::oid::ObjectId>,
    pub tenant_id: String,
    pub session_id: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    /// `"lead"` (form) or `"consultation"` (free-text). Defaults to
    /// `"lead"` when missing — matches the historical FastAPI shape
    /// where lead-form documents had no `lead_type` field.
    #[serde(default = "default_lead_type")]
    pub lead_type: String,
    /// Free-text body, only set on consultation submissions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Pipeline status. Always `"new"` on insert; mutations come from
    /// the dashboard `PATCH /v1/leads/{id}` (future).
    #[serde(default = "default_lead_status")]
    pub status: String,
    pub captured_at: BsonDateTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<BsonDateTime>,
}

fn default_lead_type() -> String {
    "lead".into()
}

fn default_lead_status() -> String {
    "new".into()
}

/// Patch payload for a brand-new lead. The repo fills in the rest.
#[derive(Debug, Clone)]
pub struct NewLead {
    pub session_id: SessionId,
    pub name: String,
    pub email: String,
    pub phone: String,
}

/// Patch payload for a brand-new consultation (lead + free-text body).
#[derive(Debug, Clone)]
pub struct NewConsultation {
    pub session_id: SessionId,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub message: String,
}

/// Tenant-scoped repository for `widget_leads`.
#[derive(Clone)]
pub struct WidgetLeadRepo {
    mongo: MongoAdapter,
}

impl WidgetLeadRepo {
    pub fn new(mongo: MongoAdapter) -> Self {
        Self { mongo }
    }

    fn collection(&self) -> mongodb::Collection<LeadDoc> {
        self.mongo.database().collection(WIDGET_LEADS_COLLECTION)
    }

    /// Build the indexes the lead surface depends on. Idempotent.
    pub async fn ensure_indexes(&self) -> Result<(), StorageError> {
        let by_tenant_captured = IndexModel::builder()
            .keys(doc! { "tenant_id": 1, "captured_at": -1 })
            .options(
                IndexOptions::builder()
                    .name("widget_leads__tenant_captured_idx".to_string())
                    .build(),
            )
            .build();
        self.collection().create_index(by_tenant_captured).await?;
        Ok(())
    }

    /// Insert a basic lead. Returns the generated lead id (Mongo `_id`
    /// rendered as a 24-char hex string for parity with FastAPI's
    /// `str(result.inserted_id)` convention).
    pub async fn insert_lead(
        &self,
        ctx: &TenantContext,
        new: &NewLead,
    ) -> Result<String, StorageError> {
        let now = now_bson();
        let doc = LeadDoc {
            id: None,
            tenant_id: ctx.tenant_id().to_string(),
            session_id: new.session_id.to_string(),
            name: new.name.clone(),
            email: new.email.clone(),
            phone: new.phone.clone(),
            lead_type: "lead".into(),
            message: None,
            status: "new".into(),
            captured_at: now,
            updated_at: None,
        };
        let result = self.collection().insert_one(&doc).await?;
        Ok(result
            .inserted_id
            .as_object_id()
            .map(|oid| oid.to_hex())
            .unwrap_or_else(|| result.inserted_id.to_string()))
    }

    /// Insert a consultation request (lead with `lead_type=consultation`
    /// and a non-empty `message`).
    pub async fn insert_consultation(
        &self,
        ctx: &TenantContext,
        new: &NewConsultation,
    ) -> Result<String, StorageError> {
        let now = now_bson();
        let doc = LeadDoc {
            id: None,
            tenant_id: ctx.tenant_id().to_string(),
            session_id: new.session_id.to_string(),
            name: new.name.clone(),
            email: new.email.clone(),
            phone: new.phone.clone(),
            lead_type: "consultation".into(),
            message: Some(new.message.clone()),
            status: "new".into(),
            captured_at: now,
            updated_at: None,
        };
        let result = self.collection().insert_one(&doc).await?;
        Ok(result
            .inserted_id
            .as_object_id()
            .map(|oid| oid.to_hex())
            .unwrap_or_else(|| result.inserted_id.to_string()))
    }
}

impl LeadDoc {
    /// Hydrate the persisted form into the public `Lead` domain type.
    /// Only used by the dashboard read surface (later phase); kept here
    /// so the conversion lives next to the schema.
    #[allow(dead_code)]
    pub(crate) fn into_domain(self) -> Result<Lead, StorageError> {
        let tenant_id: TenantId = self
            .tenant_id
            .parse::<uuid::Uuid>()
            .map(TenantId)
            .map_err(|e| StorageError::Bson(format!("tenant_id not a uuid: {e}")))?;
        let session_id: SessionId = self
            .session_id
            .parse::<uuid::Uuid>()
            .map(SessionId)
            .map_err(|e| StorageError::Bson(format!("session_id not a uuid: {e}")))?;
        let status = match self.status.as_str() {
            "new" => LeadStatus::New,
            "contacted" => LeadStatus::Contacted,
            "converted" => LeadStatus::Converted,
            "closed" => LeadStatus::Closed,
            other => return Err(StorageError::Bson(format!("unknown lead status: {other}"))),
        };
        let captured_at = bson_to_offset(self.captured_at)?;
        let updated_at = self.updated_at.map(bson_to_offset).transpose()?;

        Ok(Lead {
            id: self
                .id
                .map(|oid| oid.to_hex())
                .unwrap_or_else(|| String::from("")),
            tenant_id,
            session_id,
            name: self.name,
            email: self.email,
            phone: self.phone,
            status,
            consultation_message: self.message,
            captured_at,
            updated_at,
        })
    }
}

fn bson_to_offset(dt: BsonDateTime) -> Result<OffsetDateTime, StorageError> {
    OffsetDateTime::from_unix_timestamp_nanos(dt.timestamp_millis() as i128 * 1_000_000)
        .map_err(|e| StorageError::Bson(format!("timestamp out of range: {e}")))
}

fn now_bson() -> BsonDateTime {
    BsonDateTime::from_system_time(SystemTime::now())
}

/// Persisted widget-analytics daily bucket. One document per
/// `(tenant_id, date)`, with per-event-type counters incremented in
/// place. Mirrors the FastAPI `widget_analytics` shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetDailyBucket {
    /// ISO date `YYYY-MM-DD`. UTC.
    pub date: String,
    #[serde(default)]
    pub widget_view: u64,
    #[serde(default)]
    pub chat_start: u64,
    #[serde(default)]
    pub estimate_complete: u64,
}

/// Tenant-scoped repository for `widget_analytics`. All writes are
/// fire-and-forget upserts; reads return ordered daily buckets.
#[derive(Clone)]
pub struct WidgetAnalyticsRepo {
    mongo: MongoAdapter,
}

impl WidgetAnalyticsRepo {
    pub fn new(mongo: MongoAdapter) -> Self {
        Self { mongo }
    }

    fn collection(&self) -> mongodb::Collection<mongodb::bson::Document> {
        self.mongo
            .database()
            .collection(WIDGET_ANALYTICS_COLLECTION)
    }

    /// Build the indexes the analytics surface depends on. The
    /// `(tenant_id, date)` index is unique so the upsert path can rely
    /// on at most one document per day per tenant. Idempotent.
    pub async fn ensure_indexes(&self) -> Result<(), StorageError> {
        let unique_tenant_date = IndexModel::builder()
            .keys(doc! { "tenant_id": 1, "date": 1 })
            .options(
                IndexOptions::builder()
                    .name("widget_analytics__tenant_date_unique_idx".to_string())
                    .unique(true)
                    .build(),
            )
            .build();
        self.collection().create_index(unique_tenant_date).await?;
        Ok(())
    }

    /// Increment the counter for `event_field` in today's bucket. Caller
    /// passes the field name (`widget_view`, `chat_start`,
    /// `estimate_complete`). Allowlisting happens at the API layer; this
    /// method does *not* re-validate, but it never substitutes the field
    /// into a free-form key path.
    pub async fn increment_today(
        &self,
        ctx: &TenantContext,
        event_field: &str,
        date_iso: &str,
    ) -> Result<(), StorageError> {
        let filter = doc! {
            "tenant_id": ctx.tenant_id().to_string(),
            "date": date_iso,
        };
        let update = doc! {
            "$inc": { event_field: 1i64 },
            "$setOnInsert": {
                "tenant_id": ctx.tenant_id().to_string(),
                "date": date_iso,
            },
        };
        let opts = mongodb::options::UpdateOptions::builder()
            .upsert(true)
            .build();
        self.collection()
            .update_one(filter, update)
            .with_options(opts)
            .await?;
        Ok(())
    }

    /// Return the daily buckets for the tenant whose `date` is `>=
    /// start_date_iso`, sorted descending. The widget read endpoint
    /// caps `days` to 365 before calling.
    pub async fn find_range(
        &self,
        ctx: &TenantContext,
        start_date_iso: &str,
        limit: i64,
    ) -> Result<Vec<WidgetDailyBucket>, StorageError> {
        use futures::stream::TryStreamExt;

        let filter = doc! {
            "tenant_id": ctx.tenant_id().to_string(),
            "date": { "$gte": start_date_iso },
        };
        let opts = mongodb::options::FindOptions::builder()
            .sort(doc! { "date": -1 })
            .limit(limit)
            .projection(doc! {
                "_id": 0,
                "date": 1,
                "widget_view": 1,
                "chat_start": 1,
                "estimate_complete": 1,
            })
            .build();

        let mut cursor = self.collection().find(filter).with_options(opts).await?;
        let mut out = Vec::new();
        while let Some(d) = cursor.try_next().await? {
            // Tolerate missing counter fields on legacy docs.
            let bucket = WidgetDailyBucket {
                date: d.get_str("date").unwrap_or_default().to_string(),
                widget_view: d
                    .get_i64("widget_view")
                    .unwrap_or_else(|_| d.get_i32("widget_view").unwrap_or(0) as i64)
                    as u64,
                chat_start: d
                    .get_i64("chat_start")
                    .unwrap_or_else(|_| d.get_i32("chat_start").unwrap_or(0) as i64)
                    as u64,
                estimate_complete: d
                    .get_i64("estimate_complete")
                    .unwrap_or_else(|_| d.get_i32("estimate_complete").unwrap_or(0) as i64)
                    as u64,
            };
            out.push(bucket);
        }
        Ok(out)
    }
}
