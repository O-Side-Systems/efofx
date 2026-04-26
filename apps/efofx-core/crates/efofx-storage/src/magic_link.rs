//! Feedback magic-link tokens collection.
//!
//! Lifecycle: `create` → optional `mark_opened` (idempotent) → `consume`
//! (single-shot). Raw tokens never touch Mongo — only the SHA-256 hash
//! is persisted, the same scheme FastAPI's `MagicLinkService` uses so a
//! token minted by either backend resolves the same way.
//!
//! TTL is enforced by a Mongo TTL index on `expires_at`. Default is 72
//! hours. This module is **not** tenant-scoped at the repository surface
//! — the public form route is unauthenticated; every call carries the
//! tenant_id forward via the [`MagicLinkDoc`] returned from `resolve`.

use std::time::{Duration as StdDuration, SystemTime};

use mongodb::bson::{doc, DateTime as BsonDateTime};
use mongodb::options::IndexOptions;
use mongodb::IndexModel;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{MongoAdapter, StorageError};

pub(crate) const FEEDBACK_TOKENS_COLLECTION: &str = "feedback_tokens";

/// Hard-coded for v1 — matches FastAPI's `MAGIC_LINK_TTL_HOURS`. If we
/// ever want per-tenant overrides, lift this into config.
pub const MAGIC_LINK_TTL_HOURS: u64 = 72;

/// Persisted magic-link document. Schema-identical to FastAPI's
/// `FeedbackMagicLink` so a token minted by either backend resolves
/// against either backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagicLinkDoc {
    /// SHA-256 hex digest of the raw token. The raw token is never
    /// persisted.
    pub token_hash: String,
    pub tenant_id: String,
    pub estimation_session_id: String,
    pub customer_email: String,
    #[serde(default = "default_project_name")]
    pub project_name: String,
    pub expires_at: BsonDateTime,
    #[serde(default)]
    pub opened_at: Option<BsonDateTime>,
    #[serde(default)]
    pub used_at: Option<BsonDateTime>,
    pub created_at: BsonDateTime,
}

fn default_project_name() -> String {
    "Your Project".into()
}

/// Resolution outcome. The doc is returned for `Valid`, `Expired`, and
/// `Used` so callers can render branded follow-up pages without an
/// extra query.
#[derive(Debug, Clone)]
pub enum TokenState {
    Valid(MagicLinkDoc),
    Expired(MagicLinkDoc),
    Used(MagicLinkDoc),
    NotFound,
}

/// Patch payload for [`MagicLinkRepo::create`].
#[derive(Debug, Clone)]
pub struct NewMagicLink {
    pub tenant_id: String,
    pub estimation_session_id: String,
    pub customer_email: String,
    pub project_name: String,
}

/// Result of [`MagicLinkRepo::create`]. The raw token is returned only
/// once — caller embeds it in the email URL and forgets it.
pub struct MintedMagicLink {
    pub raw_token: String,
    pub token_hash: String,
}

/// Repository for `feedback_tokens`.
#[derive(Clone)]
pub struct MagicLinkRepo {
    mongo: MongoAdapter,
}

impl MagicLinkRepo {
    pub fn new(mongo: MongoAdapter) -> Self {
        Self { mongo }
    }

    fn collection(&self) -> mongodb::Collection<MagicLinkDoc> {
        self.mongo.database().collection(FEEDBACK_TOKENS_COLLECTION)
    }

    /// Idempotent index creation. Call on startup.
    pub async fn ensure_indexes(&self) -> Result<(), StorageError> {
        let unique_hash = IndexModel::builder()
            .keys(doc! { "token_hash": 1 })
            .options(
                IndexOptions::builder()
                    .name("feedback_tokens__hash_unique_idx".to_string())
                    .unique(true)
                    .build(),
            )
            .build();
        let ttl = IndexModel::builder()
            .keys(doc! { "expires_at": 1 })
            .options(
                IndexOptions::builder()
                    .name("feedback_tokens__expires_ttl_idx".to_string())
                    .expire_after(StdDuration::from_secs(0))
                    .build(),
            )
            .build();
        self.collection()
            .create_indexes(vec![unique_hash, ttl])
            .await?;
        Ok(())
    }

    /// Mint and persist a magic link. Returns the raw token (for email)
    /// and its hash (for caller telemetry / response body).
    pub async fn create(&self, new: NewMagicLink) -> Result<MintedMagicLink, StorageError> {
        let raw = generate_raw_token();
        let hash = hash_token(&raw);
        let now_st = SystemTime::now();
        let expires_st = now_st + StdDuration::from_secs(MAGIC_LINK_TTL_HOURS * 3600);

        let doc = MagicLinkDoc {
            token_hash: hash.clone(),
            tenant_id: new.tenant_id,
            estimation_session_id: new.estimation_session_id,
            customer_email: new.customer_email,
            project_name: new.project_name,
            expires_at: BsonDateTime::from_system_time(expires_st),
            opened_at: None,
            used_at: None,
            created_at: BsonDateTime::from_system_time(now_st),
        };
        self.collection().insert_one(&doc).await?;
        Ok(MintedMagicLink {
            raw_token: raw,
            token_hash: hash,
        })
    }

    /// Resolve a raw token to one of the four states. The returned doc
    /// is the same Mongo row for `Valid`, `Expired`, and `Used`.
    pub async fn resolve(&self, raw_token: &str) -> Result<TokenState, StorageError> {
        let hash = hash_token(raw_token);
        let Some(doc) = self
            .collection()
            .find_one(doc! { "token_hash": &hash })
            .await?
        else {
            return Ok(TokenState::NotFound);
        };

        let now_ms = BsonDateTime::now().timestamp_millis();
        if now_ms > doc.expires_at.timestamp_millis() {
            return Ok(TokenState::Expired(doc));
        }
        if doc.used_at.is_some() {
            return Ok(TokenState::Used(doc));
        }
        Ok(TokenState::Valid(doc))
    }

    /// Stamp `opened_at` on the first GET. Idempotent — never overwrites.
    pub async fn mark_opened(&self, raw_token: &str) -> Result<(), StorageError> {
        let hash = hash_token(raw_token);
        let now = BsonDateTime::from_system_time(SystemTime::now());
        self.collection()
            .update_one(
                doc! { "token_hash": &hash, "opened_at": mongodb::bson::Bson::Null },
                doc! { "$set": { "opened_at": now } },
            )
            .await?;
        Ok(())
    }

    /// Atomically claim the token. Returns `true` only the first time —
    /// concurrent submits get `false` and should render the thank-you
    /// page instead of double-writing.
    pub async fn consume(&self, raw_token: &str) -> Result<bool, StorageError> {
        let hash = hash_token(raw_token);
        let now = BsonDateTime::from_system_time(SystemTime::now());
        let res = self
            .collection()
            .update_one(
                doc! { "token_hash": &hash, "used_at": mongodb::bson::Bson::Null },
                doc! { "$set": { "used_at": now } },
            )
            .await?;
        Ok(res.modified_count > 0)
    }
}

/// 32-byte random, base64url-no-pad encoded. ~43 chars, FastAPI parity
/// with `secrets.token_urlsafe(32)`.
fn generate_raw_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    url_safe_no_pad(&bytes)
}

/// SHA-256 lowercase hex of the raw token. Identical to Python's
/// `hashlib.sha256(raw.encode()).hexdigest()` so a token minted by
/// either backend resolves the same row.
pub fn hash_token(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex_encode(&hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

/// base64url(no-pad) encoder. Avoids pulling the `base64` crate into
/// `efofx-storage`'s dependency graph for a single 32-byte payload.
fn url_safe_no_pad(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity((bytes.len() * 4).div_ceil(3));
    let mut chunks = bytes.chunks_exact(3);
    for chunk in &mut chunks {
        let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 63) as usize] as char);
        out.push(ALPHABET[(n & 63) as usize] as char);
    }
    let rem = chunks.remainder();
    match rem.len() {
        1 => {
            let n = (rem[0] as u32) << 16;
            out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
            out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
        }
        2 => {
            let n = ((rem[0] as u32) << 16) | ((rem[1] as u32) << 8);
            out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
            out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
            out.push(ALPHABET[((n >> 6) & 63) as usize] as char);
        }
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_token_matches_python_sha256_hex() {
        // python: hashlib.sha256(b"hello").hexdigest()
        assert_eq!(
            hash_token("hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn raw_tokens_are_url_safe_and_distinct() {
        let a = generate_raw_token();
        let b = generate_raw_token();
        assert_ne!(a, b);
        assert!(a
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        // 32 bytes => 43-char base64url no-pad
        assert_eq!(a.len(), 43);
    }
}
