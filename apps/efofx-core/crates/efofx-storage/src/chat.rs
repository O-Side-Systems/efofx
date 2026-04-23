//! Chat sessions collection: storage DTO and tenant-scoped repository.
//!
//! Layout mirrors the FastAPI schema so a clean-slate Rust deployment can
//! read any legacy documents if a migration lands later:
//!
//! - `session_id`, `tenant_id` are stored as strings.
//! - `status` is a lowercase enum string.
//! - `messages` is an embedded array of `{role, content, timestamp}`.
//! - `scoping_context` is an embedded document with 5 optional fields.
//! - `expires_at` is the anchor for the collection TTL index.
//!
//! Indexes created by [`ChatRepo::ensure_indexes`]:
//! - `{tenant_id: 1, session_id: 1}` (unique)
//! - `{expires_at: 1}` with `expireAfterSeconds: 0` (TTL)

use std::time::SystemTime;

use mongodb::bson::{doc, DateTime as BsonDateTime, Document};
use mongodb::options::{IndexOptions, ReturnDocument};
use mongodb::IndexModel;
use serde::{Deserialize, Serialize};
use time::{Duration as TimeDuration, OffsetDateTime};

use efofx_domain::{
    ChatMessage, ChatSession, ChatStatus, MessageRole, ScopingContext, SessionId, TenantId,
    TokenUsage,
};

use crate::{MongoAdapter, StorageError, TenantContext};

pub(crate) const CHAT_SESSIONS_COLLECTION: &str = "chat_sessions";

/// Default session lifetime. FastAPI uses 24h; keep parity.
pub const DEFAULT_SESSION_TTL_HOURS: i64 = 24;

/// Storage-layer chat-session document. All identifiers are strings to match
/// FastAPI's schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ChatSessionDoc {
    pub session_id: String,
    pub tenant_id: String,
    pub status: String,
    #[serde(default)]
    pub messages: Vec<ChatMessageDoc>,
    #[serde(default)]
    pub scoping_context: ScopingContextDoc,
    #[serde(default)]
    pub is_ready: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_version: Option<String>,
    #[serde(default)]
    pub token_usage: TokenUsageDoc,
    pub created_at: BsonDateTime,
    pub updated_at: BsonDateTime,
    pub expires_at: BsonDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ChatMessageDoc {
    pub role: String,
    pub content: String,
    pub timestamp: BsonDateTime,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct ScopingContextDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_size: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub special_conditions: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub(crate) struct TokenUsageDoc {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
}

// --- DTO ↔ domain conversions --------------------------------------------

impl ChatSessionDoc {
    pub(crate) fn from_domain(session: &ChatSession) -> Self {
        Self {
            session_id: session.session_id.to_string(),
            tenant_id: session.tenant_id.to_string(),
            status: status_to_str(session.status).to_string(),
            messages: session
                .messages
                .iter()
                .map(ChatMessageDoc::from_domain)
                .collect(),
            scoping_context: ScopingContextDoc::from_domain(&session.scoping_context),
            is_ready: session.is_ready,
            prompt_version: session.prompt_version.clone(),
            token_usage: TokenUsageDoc::from_domain(&session.token_usage),
            created_at: offset_to_bson(session.created_at),
            updated_at: offset_to_bson(session.updated_at),
            expires_at: offset_to_bson(session.expires_at),
        }
    }

    pub(crate) fn into_domain(self) -> Result<ChatSession, StorageError> {
        Ok(ChatSession {
            session_id: SessionId(
                self.session_id
                    .parse()
                    .map_err(|e| StorageError::Bson(format!("session_id not a uuid: {e}")))?,
            ),
            tenant_id: TenantId(
                self.tenant_id
                    .parse()
                    .map_err(|e| StorageError::Bson(format!("tenant_id not a uuid: {e}")))?,
            ),
            status: status_from_str(&self.status)?,
            messages: self
                .messages
                .into_iter()
                .map(ChatMessageDoc::into_domain)
                .collect::<Result<_, _>>()?,
            scoping_context: self.scoping_context.into_domain(),
            is_ready: self.is_ready,
            prompt_version: self.prompt_version,
            token_usage: self.token_usage.into_domain(),
            created_at: bson_to_offset(self.created_at)?,
            updated_at: bson_to_offset(self.updated_at)?,
            expires_at: bson_to_offset(self.expires_at)?,
        })
    }
}

impl ChatMessageDoc {
    fn from_domain(m: &ChatMessage) -> Self {
        Self {
            role: role_to_str(m.role).to_string(),
            content: m.content.clone(),
            timestamp: offset_to_bson(m.timestamp),
        }
    }
    fn into_domain(self) -> Result<ChatMessage, StorageError> {
        Ok(ChatMessage {
            role: role_from_str(&self.role)?,
            content: self.content,
            timestamp: bson_to_offset(self.timestamp)?,
        })
    }
}

impl ScopingContextDoc {
    fn from_domain(c: &ScopingContext) -> Self {
        Self {
            project_type: c.project_type.clone(),
            project_size: c.project_size.clone(),
            location: c.location.clone(),
            timeline: c.timeline.clone(),
            special_conditions: c.special_conditions.clone(),
        }
    }
    fn into_domain(self) -> ScopingContext {
        ScopingContext {
            project_type: self.project_type,
            project_size: self.project_size,
            location: self.location,
            timeline: self.timeline,
            special_conditions: self.special_conditions,
        }
    }
}

impl TokenUsageDoc {
    fn from_domain(u: &TokenUsage) -> Self {
        Self {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }
    }
    fn into_domain(self) -> TokenUsage {
        TokenUsage {
            prompt_tokens: self.prompt_tokens,
            completion_tokens: self.completion_tokens,
            total_tokens: self.total_tokens,
        }
    }
}

fn status_to_str(s: ChatStatus) -> &'static str {
    match s {
        ChatStatus::Active => "active",
        ChatStatus::Ready => "ready",
        ChatStatus::Completed => "completed",
        ChatStatus::Expired => "expired",
    }
}

fn status_from_str(s: &str) -> Result<ChatStatus, StorageError> {
    match s {
        "active" => Ok(ChatStatus::Active),
        "ready" => Ok(ChatStatus::Ready),
        "completed" => Ok(ChatStatus::Completed),
        "expired" => Ok(ChatStatus::Expired),
        other => Err(StorageError::Bson(format!("unknown chat status: {other}"))),
    }
}

fn role_to_str(r: MessageRole) -> &'static str {
    match r {
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::System => "system",
    }
}

fn role_from_str(s: &str) -> Result<MessageRole, StorageError> {
    match s {
        "user" => Ok(MessageRole::User),
        "assistant" => Ok(MessageRole::Assistant),
        "system" => Ok(MessageRole::System),
        other => Err(StorageError::Bson(format!("unknown message role: {other}"))),
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

/// Tenant-scoped chat-session repository. Every method filters by
/// `ctx.tenant_id()` so cross-tenant access is impossible without forging a
/// [`TenantContext`] — which `efofx-storage`'s module boundary prevents.
#[derive(Clone)]
pub struct ChatRepo {
    mongo: MongoAdapter,
}

impl ChatRepo {
    pub fn new(mongo: MongoAdapter) -> Self {
        Self { mongo }
    }

    fn collection(&self) -> mongodb::Collection<ChatSessionDoc> {
        self.mongo.database().collection(CHAT_SESSIONS_COLLECTION)
    }

    fn docs_collection(&self) -> mongodb::Collection<Document> {
        self.mongo.database().collection(CHAT_SESSIONS_COLLECTION)
    }

    /// Create both indexes required by this collection. Idempotent. Call
    /// from startup.
    pub async fn ensure_indexes(&self) -> Result<(), StorageError> {
        let unique_tenant_session = IndexModel::builder()
            .keys(doc! { "tenant_id": 1, "session_id": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build();

        let ttl = IndexModel::builder()
            .keys(doc! { "expires_at": 1 })
            .options(
                IndexOptions::builder()
                    .expire_after(std::time::Duration::from_secs(0))
                    .build(),
            )
            .build();

        self.collection()
            .create_indexes(vec![unique_tenant_session, ttl])
            .await?;
        Ok(())
    }

    /// Create a fresh session for this tenant. `ttl_hours` defaults to 24h
    /// when `None` — matches FastAPI behavior.
    pub async fn create(
        &self,
        ctx: &TenantContext,
        prompt_version: Option<String>,
        ttl_hours: Option<i64>,
    ) -> Result<ChatSession, StorageError> {
        let now = OffsetDateTime::now_utc();
        let expires_at = now + TimeDuration::hours(ttl_hours.unwrap_or(DEFAULT_SESSION_TTL_HOURS));
        let session = ChatSession {
            session_id: SessionId::new(),
            tenant_id: ctx.tenant_id(),
            status: ChatStatus::Active,
            messages: Vec::new(),
            scoping_context: ScopingContext::default(),
            is_ready: false,
            prompt_version,
            token_usage: TokenUsage::default(),
            created_at: now,
            updated_at: now,
            expires_at,
        };
        let doc = ChatSessionDoc::from_domain(&session);
        self.collection().insert_one(doc).await?;
        Ok(session)
    }

    /// Fetch a session by id. Returns `StorageError::NotFound` if missing or
    /// if it belongs to another tenant.
    pub async fn get(
        &self,
        ctx: &TenantContext,
        session_id: SessionId,
    ) -> Result<ChatSession, StorageError> {
        let doc = self
            .collection()
            .find_one(doc! {
                "tenant_id": ctx.tenant_id().to_string(),
                "session_id": session_id.to_string(),
            })
            .await?
            .ok_or(StorageError::NotFound)?;
        doc.into_domain()
    }

    /// Append a single message. Bumps `updated_at`. Does not touch
    /// scoping_context, is_ready, status, or token_usage.
    pub async fn append_message(
        &self,
        ctx: &TenantContext,
        session_id: SessionId,
        message: &ChatMessage,
    ) -> Result<ChatSession, StorageError> {
        let msg_doc = ChatMessageDoc::from_domain(message);
        let msg_bson = mongodb::bson::serialize_to_bson(&msg_doc)
            .map_err(|e| StorageError::Bson(format!("encode message: {e}")))?;
        let now = offset_to_bson(OffsetDateTime::now_utc());

        let opts = mongodb::options::FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::After)
            .build();
        let updated = self
            .collection()
            .find_one_and_update(
                doc! {
                    "tenant_id": ctx.tenant_id().to_string(),
                    "session_id": session_id.to_string(),
                },
                doc! {
                    "$push": { "messages": msg_bson },
                    "$set": { "updated_at": now },
                },
            )
            .with_options(opts)
            .await?
            .ok_or(StorageError::NotFound)?;
        updated.into_domain()
    }

    /// Replace the scoping_context and is_ready flag atomically.
    pub async fn update_scoping_context(
        &self,
        ctx: &TenantContext,
        session_id: SessionId,
        scoping_context: &ScopingContext,
        is_ready: bool,
    ) -> Result<(), StorageError> {
        let ctx_doc = ScopingContextDoc::from_domain(scoping_context);
        let ctx_bson = mongodb::bson::serialize_to_bson(&ctx_doc)
            .map_err(|e| StorageError::Bson(format!("encode scoping context: {e}")))?;
        let now = offset_to_bson(OffsetDateTime::now_utc());

        self.docs_collection()
            .update_one(
                doc! {
                    "tenant_id": ctx.tenant_id().to_string(),
                    "session_id": session_id.to_string(),
                },
                doc! {
                    "$set": {
                        "scoping_context": ctx_bson,
                        "is_ready": is_ready,
                        "updated_at": now,
                    }
                },
            )
            .await?;
        Ok(())
    }

    /// Transition the session status. Caller verifies the transition is
    /// legal (active→ready, ready→completed, anything→expired).
    pub async fn update_status(
        &self,
        ctx: &TenantContext,
        session_id: SessionId,
        status: ChatStatus,
    ) -> Result<(), StorageError> {
        let now = offset_to_bson(OffsetDateTime::now_utc());
        self.docs_collection()
            .update_one(
                doc! {
                    "tenant_id": ctx.tenant_id().to_string(),
                    "session_id": session_id.to_string(),
                },
                doc! {
                    "$set": {
                        "status": status_to_str(status),
                        "updated_at": now,
                    }
                },
            )
            .await?;
        Ok(())
    }

    /// Increment token-usage counters atomically. Returns the post-increment
    /// totals so budget enforcement can act on the fresh value.
    pub async fn increment_token_usage(
        &self,
        ctx: &TenantContext,
        session_id: SessionId,
        prompt_delta: u64,
        completion_delta: u64,
    ) -> Result<TokenUsage, StorageError> {
        let now = offset_to_bson(OffsetDateTime::now_utc());
        let total_delta = prompt_delta + completion_delta;

        let opts = mongodb::options::FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::After)
            .build();
        let updated = self
            .collection()
            .find_one_and_update(
                doc! {
                    "tenant_id": ctx.tenant_id().to_string(),
                    "session_id": session_id.to_string(),
                },
                doc! {
                    "$inc": {
                        "token_usage.prompt_tokens": prompt_delta as i64,
                        "token_usage.completion_tokens": completion_delta as i64,
                        "token_usage.total_tokens": total_delta as i64,
                    },
                    "$set": { "updated_at": now },
                },
            )
            .with_options(opts)
            .await?
            .ok_or(StorageError::NotFound)?;
        Ok(updated.token_usage.into_domain())
    }

    /// Sum `token_usage.total_tokens` across every session this tenant has.
    /// Returns 0 when the tenant has no sessions.
    ///
    /// This is an O(n_sessions) aggregation — acceptable at demo scale
    /// (sessions per tenant measured in dozens, TTL-expired after 24h).
    /// If a tenant's cardinality climbs, swap this for a dedicated
    /// counter on the tenant doc maintained by
    /// `increment_token_usage`.
    pub async fn tenant_total_tokens(&self, ctx: &TenantContext) -> Result<u64, StorageError> {
        use futures::stream::TryStreamExt;

        let pipeline = vec![
            doc! { "$match": { "tenant_id": ctx.tenant_id().to_string() } },
            doc! {
                "$group": {
                    "_id": null,
                    "total": { "$sum": "$token_usage.total_tokens" },
                }
            },
        ];
        let mut cursor = self.docs_collection().aggregate(pipeline).await?;
        if let Some(doc) = cursor.try_next().await? {
            // `$sum` returns i64 from mongo; clamp to u64.
            if let Ok(n) = doc.get_i64("total") {
                return Ok(n.max(0) as u64);
            }
            if let Ok(n) = doc.get_i32("total") {
                return Ok(n.max(0) as u64);
            }
        }
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use efofx_domain::MessageRole;
    use uuid::Uuid;

    fn sample_session() -> ChatSession {
        let now = OffsetDateTime::now_utc();
        let mut scoping = ScopingContext::default();
        scoping.project_type = Some("pool".into());
        scoping.project_size = Some("15x30 feet".into());
        let mut usage = TokenUsage::default();
        usage.add(120, 40);
        ChatSession {
            session_id: SessionId(Uuid::new_v4()),
            tenant_id: TenantId(Uuid::new_v4()),
            status: ChatStatus::Active,
            messages: vec![
                ChatMessage {
                    role: MessageRole::System,
                    content: "be helpful".into(),
                    timestamp: now,
                },
                ChatMessage {
                    role: MessageRole::User,
                    content: "I want a pool 🏊".into(),
                    timestamp: now,
                },
            ],
            scoping_context: scoping,
            is_ready: false,
            prompt_version: Some("1.0.0".into()),
            token_usage: usage,
            created_at: now,
            updated_at: now,
            expires_at: now + TimeDuration::hours(24),
        }
    }

    #[test]
    fn dto_round_trip_preserves_fields() {
        let original = sample_session();
        let doc = ChatSessionDoc::from_domain(&original);
        let restored = doc.into_domain().expect("round trip");

        assert_eq!(restored.session_id, original.session_id);
        assert_eq!(restored.tenant_id, original.tenant_id);
        assert_eq!(restored.status, ChatStatus::Active);
        assert_eq!(restored.messages.len(), 2);
        assert_eq!(restored.messages[1].content, "I want a pool 🏊");
        assert_eq!(
            restored.scoping_context.project_type.as_deref(),
            Some("pool")
        );
        assert_eq!(
            restored.scoping_context.project_size.as_deref(),
            Some("15x30 feet")
        );
        assert!(restored.scoping_context.location.is_none());
        assert_eq!(restored.prompt_version.as_deref(), Some("1.0.0"));
        assert_eq!(restored.token_usage, original.token_usage);
    }

    #[test]
    fn round_trip_through_bson() {
        let original = sample_session();
        let doc = ChatSessionDoc::from_domain(&original);

        let bson = mongodb::bson::serialize_to_document(&doc).expect("encode to bson");
        let decoded: ChatSessionDoc =
            mongodb::bson::deserialize_from_document(bson).expect("decode from bson");
        let restored = decoded.into_domain().expect("back to domain");

        assert_eq!(restored.session_id, original.session_id);
        assert_eq!(restored.status, original.status);
        assert_eq!(restored.messages.len(), original.messages.len());
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
    fn unknown_role_rejected() {
        let err = role_from_str("spectator").unwrap_err();
        match err {
            StorageError::Bson(msg) => assert!(msg.contains("spectator")),
            other => panic!("wrong error: {other:?}"),
        }
    }

    #[test]
    fn all_status_variants_round_trip() {
        for s in [
            ChatStatus::Active,
            ChatStatus::Ready,
            ChatStatus::Completed,
            ChatStatus::Expired,
        ] {
            let back = status_from_str(status_to_str(s)).unwrap();
            assert_eq!(back, s);
        }
    }

    #[test]
    fn all_role_variants_round_trip() {
        for r in [
            MessageRole::User,
            MessageRole::Assistant,
            MessageRole::System,
        ] {
            let back = role_from_str(role_to_str(r)).unwrap();
            assert_eq!(back, r);
        }
    }
}
