//! Estimation sessions collection: storage DTO and tenant-scoped repository.
//!
//! Mirrors `apps/efofx-estimate/app/models/estimation.py::EstimationSession`
//! on the wire. Notable shape notes:
//!
//! - `session_id` is stored as a string (FastAPI uses `sess_{hex[:12]}`,
//!   not a UUID). See [`efofx_domain::EstimationSessionId`].
//! - `tenant_id` is a string UUID to parity-match the chat repo.
//! - `status` is a snake_case enum string.
//! - `region` is the FastAPI display form (`"SoCal - Coastal"`, etc.),
//!   serialized via the domain [`efofx_domain::Region`] serde rename.
//! - `expires_at` is nullable but drives the TTL index when present.
//!
//! Indexes created by [`EstimationRepo::ensure_indexes`]:
//! - `{tenant_id: 1, session_id: 1}` (unique)
//! - `{expires_at: 1}` with `expireAfterSeconds: 0` (TTL; partial index on
//!   non-null `expires_at` so rows without expiry survive indefinitely).

use std::time::SystemTime;

use mongodb::bson::{doc, DateTime as BsonDateTime, Document};
use mongodb::options::{IndexOptions, ReturnDocument};
use mongodb::IndexModel;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use efofx_domain::{EstimationSession, EstimationSessionId, EstimationStatus, Region, TenantId};

use crate::{MongoAdapter, StorageError, TenantContext};

pub(crate) const ESTIMATES_COLLECTION: &str = "estimates";

/// Storage-layer estimation-session document. Field order mirrors
/// [`EstimationSession`] so a diff between the two is a bidirectional audit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EstimationSessionDoc {
    pub session_id: String,
    pub tenant_id: String,
    pub status: String,
    pub description: String,
    /// Stored as the FastAPI display string (e.g. `"SoCal - Coastal"`).
    pub region: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_class: Option<String>,
    pub confidence_threshold: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_version: Option<String>,
    pub created_at: BsonDateTime,
    pub updated_at: BsonDateTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<BsonDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<BsonDateTime>,
}

impl EstimationSessionDoc {
    pub(crate) fn from_domain(session: &EstimationSession) -> Self {
        Self {
            session_id: session.id.to_string(),
            tenant_id: session.tenant_id.to_string(),
            status: status_to_str(session.status).to_string(),
            description: session.description.clone(),
            region: region_to_str(session.region).to_string(),
            reference_class: session.reference_class.clone(),
            confidence_threshold: session.confidence_threshold,
            prompt_version: session.prompt_version.clone(),
            created_at: offset_to_bson(session.created_at),
            updated_at: offset_to_bson(session.updated_at),
            completed_at: session.completed_at.map(offset_to_bson),
            expires_at: session.expires_at.map(offset_to_bson),
        }
    }

    pub(crate) fn into_domain(self) -> Result<EstimationSession, StorageError> {
        Ok(EstimationSession {
            id: EstimationSessionId(self.session_id),
            tenant_id: TenantId(
                self.tenant_id
                    .parse()
                    .map_err(|e| StorageError::Bson(format!("tenant_id not a uuid: {e}")))?,
            ),
            status: status_from_str(&self.status)?,
            description: self.description,
            region: region_from_str(&self.region)?,
            reference_class: self.reference_class,
            confidence_threshold: self.confidence_threshold,
            prompt_version: self.prompt_version,
            created_at: bson_to_offset(self.created_at)?,
            updated_at: bson_to_offset(self.updated_at)?,
            completed_at: self.completed_at.map(bson_to_offset).transpose()?,
            expires_at: self.expires_at.map(bson_to_offset).transpose()?,
        })
    }
}

fn status_to_str(s: EstimationStatus) -> &'static str {
    match s {
        EstimationStatus::Initiated => "initiated",
        EstimationStatus::InProgress => "in_progress",
        EstimationStatus::Completed => "completed",
        EstimationStatus::Cancelled => "cancelled",
        EstimationStatus::Expired => "expired",
    }
}

fn status_from_str(s: &str) -> Result<EstimationStatus, StorageError> {
    match s {
        "initiated" => Ok(EstimationStatus::Initiated),
        "in_progress" => Ok(EstimationStatus::InProgress),
        "completed" => Ok(EstimationStatus::Completed),
        "cancelled" => Ok(EstimationStatus::Cancelled),
        "expired" => Ok(EstimationStatus::Expired),
        other => Err(StorageError::Bson(format!(
            "unknown estimation status: {other}"
        ))),
    }
}

fn region_to_str(r: Region) -> &'static str {
    match r {
        Region::SoCalCoastal => "SoCal - Coastal",
        Region::SoCalInland => "SoCal - Inland",
        Region::NorCalBayArea => "NorCal - Bay Area",
        Region::NorCalCentral => "NorCal - Central",
        Region::ArizonaPhoenix => "Arizona - Phoenix",
        Region::ArizonaTucson => "Arizona - Tucson",
        Region::NevadaLasVegas => "Nevada - Las Vegas",
        Region::NevadaReno => "Nevada - Reno",
    }
}

fn region_from_str(s: &str) -> Result<Region, StorageError> {
    match s {
        "SoCal - Coastal" => Ok(Region::SoCalCoastal),
        "SoCal - Inland" => Ok(Region::SoCalInland),
        "NorCal - Bay Area" => Ok(Region::NorCalBayArea),
        "NorCal - Central" => Ok(Region::NorCalCentral),
        "Arizona - Phoenix" => Ok(Region::ArizonaPhoenix),
        "Arizona - Tucson" => Ok(Region::ArizonaTucson),
        "Nevada - Las Vegas" => Ok(Region::NevadaLasVegas),
        "Nevada - Reno" => Ok(Region::NevadaReno),
        other => Err(StorageError::Bson(format!("unknown region: {other}"))),
    }
}

fn offset_to_bson(dt: OffsetDateTime) -> BsonDateTime {
    BsonDateTime::from_system_time(SystemTime::from(dt))
}

fn bson_to_offset(dt: BsonDateTime) -> Result<OffsetDateTime, StorageError> {
    OffsetDateTime::from_unix_timestamp_nanos(dt.timestamp_millis() as i128 * 1_000_000)
        .map_err(|e| StorageError::Bson(format!("timestamp out of range: {e}")))
}

// --- Repository ----------------------------------------------------------

/// Tenant-scoped estimation-session repository. Like [`crate::ChatRepo`],
/// every method filters by `ctx.tenant_id()` — cross-tenant reads are
/// impossible without forging a [`TenantContext`], which the crate's module
/// boundary prevents.
#[derive(Clone)]
pub struct EstimationRepo {
    mongo: MongoAdapter,
}

impl EstimationRepo {
    pub fn new(mongo: MongoAdapter) -> Self {
        Self { mongo }
    }

    fn collection(&self) -> mongodb::Collection<EstimationSessionDoc> {
        self.mongo.database().collection(ESTIMATES_COLLECTION)
    }

    fn docs_collection(&self) -> mongodb::Collection<Document> {
        self.mongo.database().collection(ESTIMATES_COLLECTION)
    }

    /// Idempotent index creation. Call on startup.
    pub async fn ensure_indexes(&self) -> Result<(), StorageError> {
        let unique_tenant_session = IndexModel::builder()
            .keys(doc! { "tenant_id": 1, "session_id": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build();

        // TTL is a partial index on non-null expires_at so historical rows
        // without an explicit expiry don't get reaped.
        let ttl = IndexModel::builder()
            .keys(doc! { "expires_at": 1 })
            .options(
                IndexOptions::builder()
                    .expire_after(std::time::Duration::from_secs(0))
                    .partial_filter_expression(doc! { "expires_at": { "$type": "date" } })
                    .build(),
            )
            .build();

        self.collection()
            .create_indexes(vec![unique_tenant_session, ttl])
            .await?;
        Ok(())
    }

    /// Insert a fresh estimation session. Caller constructs the full
    /// [`EstimationSession`] — timestamps, status, prompt version, etc. —
    /// because the orchestrating service owns the policy. The `tenant_id` on
    /// the session must match `ctx`; we enforce this to keep a stray
    /// misuse from writing across tenants.
    pub async fn save(
        &self,
        ctx: &TenantContext,
        session: &EstimationSession,
    ) -> Result<(), StorageError> {
        if session.tenant_id != ctx.tenant_id() {
            return Err(StorageError::Bson(
                "session.tenant_id mismatches TenantContext".into(),
            ));
        }
        let doc = EstimationSessionDoc::from_domain(session);
        self.collection().insert_one(doc).await?;
        Ok(())
    }

    /// Fetch a session by id within the tenant scope. Returns
    /// [`StorageError::NotFound`] when missing or tenant-scoped out.
    ///
    /// If `expires_at` is past, the returned session's status is
    /// [`EstimationStatus::Expired`] and the Mongo row is updated in place
    /// — matching FastAPI's expire-on-read behavior in
    /// `EstimationService.get_estimation`.
    pub async fn get(
        &self,
        ctx: &TenantContext,
        session_id: &EstimationSessionId,
    ) -> Result<EstimationSession, StorageError> {
        let doc = self
            .collection()
            .find_one(doc! {
                "tenant_id": ctx.tenant_id().to_string(),
                "session_id": session_id.as_str(),
            })
            .await?
            .ok_or(StorageError::NotFound)?;
        let mut session = doc.into_domain()?;
        if session.expire_if_past(OffsetDateTime::now_utc()) {
            self.mark_expired(ctx, session_id, session.updated_at)
                .await?;
        }
        Ok(session)
    }

    /// Atomically flip a session to [`EstimationStatus::Expired`] without
    /// reading. Used by [`Self::get`] and exposed for background sweeps.
    pub async fn mark_expired(
        &self,
        ctx: &TenantContext,
        session_id: &EstimationSessionId,
        updated_at: OffsetDateTime,
    ) -> Result<(), StorageError> {
        let now = offset_to_bson(updated_at);
        let opts = mongodb::options::FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::After)
            .build();
        let _ = self
            .docs_collection()
            .find_one_and_update(
                doc! {
                    "tenant_id": ctx.tenant_id().to_string(),
                    "session_id": session_id.as_str(),
                },
                doc! {
                    "$set": {
                        "status": status_to_str(EstimationStatus::Expired),
                        "updated_at": now,
                    }
                },
            )
            .with_options(opts)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use efofx_domain::{EstimationSessionId, Region, TenantId};
    use time::Duration as TimeDuration;
    use uuid::Uuid;

    fn sample(now: OffsetDateTime, tenant: TenantId) -> EstimationSession {
        EstimationSession {
            id: EstimationSessionId::new(),
            tenant_id: tenant,
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
    fn dto_round_trip_preserves_fields() {
        let now = OffsetDateTime::now_utc();
        let tenant = TenantId(Uuid::new_v4());
        let original = sample(now, tenant);
        let doc = EstimationSessionDoc::from_domain(&original);
        let restored = doc.into_domain().expect("into domain");

        assert_eq!(restored.id, original.id);
        assert_eq!(restored.tenant_id, original.tenant_id);
        assert_eq!(restored.status, original.status);
        assert_eq!(restored.description, original.description);
        assert_eq!(restored.region, original.region);
        assert_eq!(restored.reference_class, original.reference_class);
        assert_eq!(restored.confidence_threshold, original.confidence_threshold);
        assert_eq!(restored.prompt_version, original.prompt_version);
    }

    #[test]
    fn round_trip_through_bson() {
        let now = OffsetDateTime::now_utc();
        let tenant = TenantId(Uuid::new_v4());
        let original = sample(now, tenant);

        let doc = EstimationSessionDoc::from_domain(&original);
        let bson = mongodb::bson::serialize_to_document(&doc).expect("encode to bson");
        let decoded: EstimationSessionDoc =
            mongodb::bson::deserialize_from_document(bson).expect("decode from bson");
        let restored = decoded.into_domain().expect("back to domain");

        assert_eq!(restored.id, original.id);
        assert_eq!(restored.region, original.region);
        assert_eq!(restored.status, original.status);
    }

    #[test]
    fn region_wire_matches_fastapi_display() {
        // This is the hotspot: if someone rewrites these mappings in
        // snake_case, FastAPI-writ docs will stop decoding.
        assert_eq!(region_to_str(Region::SoCalCoastal), "SoCal - Coastal");
        assert_eq!(region_to_str(Region::NorCalBayArea), "NorCal - Bay Area");
        assert_eq!(region_to_str(Region::NevadaLasVegas), "Nevada - Las Vegas");
    }

    #[test]
    fn all_status_variants_round_trip() {
        for s in [
            EstimationStatus::Initiated,
            EstimationStatus::InProgress,
            EstimationStatus::Completed,
            EstimationStatus::Cancelled,
            EstimationStatus::Expired,
        ] {
            let back = status_from_str(status_to_str(s)).unwrap();
            assert_eq!(back, s);
        }
    }

    #[test]
    fn all_region_variants_round_trip() {
        for r in [
            Region::SoCalCoastal,
            Region::SoCalInland,
            Region::NorCalBayArea,
            Region::NorCalCentral,
            Region::ArizonaPhoenix,
            Region::ArizonaTucson,
            Region::NevadaLasVegas,
            Region::NevadaReno,
        ] {
            let back = region_from_str(region_to_str(r)).unwrap();
            assert_eq!(back, r);
        }
    }

    #[test]
    fn unknown_status_rejected() {
        let err = status_from_str("WRONG").unwrap_err();
        match err {
            StorageError::Bson(msg) => assert!(msg.contains("WRONG")),
            other => panic!("wrong error: {other:?}"),
        }
    }

    #[test]
    fn unknown_region_rejected() {
        let err = region_from_str("Jupiter").unwrap_err();
        match err {
            StorageError::Bson(msg) => assert!(msg.contains("Jupiter")),
            other => panic!("wrong error: {other:?}"),
        }
    }

    #[test]
    fn optional_fields_omit_when_none() {
        let now = OffsetDateTime::now_utc();
        let tenant = TenantId(Uuid::new_v4());
        let mut s = sample(now, tenant);
        s.reference_class = None;
        s.prompt_version = None;
        s.completed_at = None;
        s.expires_at = None;

        let doc = EstimationSessionDoc::from_domain(&s);
        let bson = mongodb::bson::serialize_to_document(&doc).unwrap();
        assert!(!bson.contains_key("reference_class"));
        assert!(!bson.contains_key("prompt_version"));
        assert!(!bson.contains_key("completed_at"));
        assert!(!bson.contains_key("expires_at"));
    }
}
