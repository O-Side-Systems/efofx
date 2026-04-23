//! The single blessed path for minting a [`TenantContext`].
//!
//! Two entry points:
//! 1. [`TenantResolver::resolve_or_provision`] — verified-Supabase-JWT flow.
//!    Looks up or provisions the tenant by `supabase_user_id`.
//! 2. [`TenantResolver::resolve_by_api_key`] — widget API-key flow. Verifies
//!    the raw key with HMAC-SHA256 constant-time compare.
//!
//! Everything that can mint a [`TenantContext`] lives here. The only public
//! constructor on `TenantContext` is `pub(crate)`, so downstream crates
//! literally cannot fabricate one.

use hmac::{Hmac, Mac};
use mongodb::bson::doc;
use mongodb::options::{FindOneAndUpdateOptions, ReturnDocument};
use rand::{rngs::OsRng, RngCore};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use tracing::info;

use efofx_domain::{TenantId, TenantTier};

use crate::tenants::{now_bson, TenantDoc, TENANTS_COLLECTION};
use crate::{MongoAdapter, StorageError, TenantContext};

/// Domain-separation tag for widget API-key HMAC. Fixed forever — if we
/// ever change it we must migrate every stored key.
const API_KEY_HMAC_TAG: &[u8] = b"efofx-api-key:";

/// Master key material. Drives the widget API-key HMAC and — via HKDF —
/// per-tenant Fernet keys for BYOK.
#[derive(Clone)]
pub struct MasterKey(Vec<u8>);

impl MasterKey {
    /// Build from raw bytes. Caller is responsible for supplying at least
    /// 32 bytes of high-entropy material; anything less is rejected.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, StorageError> {
        if bytes.len() < 32 {
            return Err(StorageError::MasterKeyTooShort);
        }
        Ok(Self(bytes))
    }

    /// Build from the typical configuration form — a string that is either
    /// a base64 blob or a plaintext passphrase ≥32 bytes.
    pub fn from_config_str(s: &str) -> Result<Self, StorageError> {
        if s.len() >= 32 {
            Ok(Self(s.as_bytes().to_vec()))
        } else {
            Err(StorageError::MasterKeyTooShort)
        }
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for MasterKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MasterKey")
            .field("len", &self.0.len())
            .finish()
    }
}

/// Identity claims we accept for provisioning. Deliberately framework-free
/// so `efofx-auth` can depend on this crate without a cycle.
#[derive(Debug, Clone)]
pub struct ProvisionInput {
    pub supabase_user_id: String,
    pub email: String,
    /// Falls back to `email` when absent.
    pub company_name: Option<String>,
}

/// Holds the master key and exposes HMAC-verify for widget API keys.
#[derive(Clone, Debug)]
pub struct ApiKeyAuth {
    master_key: MasterKey,
}

impl ApiKeyAuth {
    pub fn new(master_key: MasterKey) -> Self {
        Self { master_key }
    }

    /// Compute the stored-form HMAC-SHA256 of a raw widget API key.
    pub fn hmac_hex(&self, raw_key: &str) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.master_key.as_bytes())
            .expect("hmac accepts any key length");
        mac.update(API_KEY_HMAC_TAG);
        mac.update(raw_key.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    /// Constant-time compare of a raw key against a stored hex HMAC. Returns
    /// `false` for any malformed or mismatched input — no timing oracle.
    pub fn verify(&self, raw_key: &str, stored_hex: &str) -> bool {
        let computed = self.hmac_hex(raw_key);
        let a = computed.as_bytes();
        let b = stored_hex.as_bytes();
        if a.len() != b.len() {
            // Still pay the comparison cost to keep timing flat.
            let _ = a.ct_eq(a);
            return false;
        }
        a.ct_eq(b).into()
    }

    /// Produce a fresh widget API key for `tenant_id`. The raw value is
    /// returned exactly once — the caller is responsible for storing the
    /// HMAC + last-6 on the tenant record and handing the raw back to the
    /// user.
    pub fn generate(&self, tenant_id: TenantId) -> NewApiKey {
        let mut random_bytes = [0u8; 16];
        OsRng.fill_bytes(&mut random_bytes);
        let random_part = url_safe_no_pad(&random_bytes);
        let tid_no_dashes = tenant_id.0.simple().to_string();
        let raw = format!("sk_live_{tid_no_dashes}_{random_part}");
        let hmac_hex = self.hmac_hex(&raw);
        let last6 = raw
            .chars()
            .rev()
            .take(6)
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();
        NewApiKey {
            raw,
            hmac_hex,
            last6,
        }
    }
}

/// Minimal base64url(no-pad) encoder. Keeps the `base64` crate out of the
/// storage dependency graph.
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

/// A freshly generated widget API key. The `raw` value is the only thing
/// the user ever sees; the rest is what we persist.
pub struct NewApiKey {
    pub raw: String,
    pub hmac_hex: String,
    pub last6: String,
}

/// Owns the Mongo handle and provides the two blessed mint paths.
#[derive(Clone)]
pub struct TenantResolver {
    mongo: MongoAdapter,
}

impl TenantResolver {
    pub fn new(mongo: MongoAdapter) -> Self {
        Self { mongo }
    }

    /// Look up or provision a tenant from verified Supabase claims.
    ///
    /// Provisioning is race-safe via a `find_one_and_update` upsert: two
    /// concurrent first-login requests end up with the same tenant record.
    ///
    /// Emits a single `tenant.provisioned` log event when a new record is
    /// inserted.
    pub async fn resolve_or_provision(
        &self,
        input: ProvisionInput,
    ) -> Result<TenantContext, StorageError> {
        let coll = self
            .mongo
            .database()
            .collection::<TenantDoc>(TENANTS_COLLECTION);

        let tenant_id = TenantId::new();
        let (_now, bson_now) = now_bson();

        let fallback_company = input.company_name.unwrap_or_else(|| input.email.clone());

        let filter = doc! { "supabase_user_id": &input.supabase_user_id };
        let update = doc! {
            "$setOnInsert": {
                "tenant_id": tenant_id.0.to_string(),
                "supabase_user_id": &input.supabase_user_id,
                "company_name": &fallback_company,
                "email": &input.email,
                "tier": "trial",
                "api_key_last6": mongodb::bson::Bson::Null,
                "api_key_hmac": mongodb::bson::Bson::Null,
                "encrypted_openai_key": mongodb::bson::Bson::Null,
                "openai_key_last6": mongodb::bson::Bson::Null,
                "is_active": true,
                "created_at": bson_now,
                "updated_at": bson_now,
            }
        };

        let opts = FindOneAndUpdateOptions::builder()
            .upsert(true)
            .return_document(ReturnDocument::After)
            .build();

        // `find_one_and_update` with `upsert=true` + `ReturnDocument::After`
        // returns the inserted-or-existing document. Timestamps-match
        // heuristically distinguishes our-insert from existing-record so we
        // only log the provisioning event once.
        let tenant_doc = coll
            .find_one_and_update(filter, update)
            .with_options(opts)
            .await?
            .ok_or(StorageError::NotFound)?;

        if tenant_doc.created_at == tenant_doc.updated_at {
            info!(
                event = "tenant.provisioned",
                tenant_id = %tenant_doc.tenant_id,
                supabase_user_id = %input.supabase_user_id,
                "provisioned tenant on first Supabase login"
            );
        }

        let tenant = tenant_doc.into_domain()?;
        Ok(TenantContext::new(tenant.tenant_id))
    }

    /// Verify a raw widget API key and mint the tenant context.
    ///
    /// Parses the embedded tenant id for O(1) lookup, then verifies the
    /// HMAC in constant time. Any malformed or mismatched input returns
    /// `StorageError::ApiKeyInvalid`.
    pub async fn resolve_by_api_key(
        &self,
        raw_key: &str,
        auth: &ApiKeyAuth,
    ) -> Result<TenantContext, StorageError> {
        let tenant_id = parse_api_key_tenant(raw_key).ok_or(StorageError::ApiKeyInvalid)?;

        let coll = self
            .mongo
            .database()
            .collection::<TenantDoc>(TENANTS_COLLECTION);

        let tenant_doc = coll
            .find_one(doc! { "tenant_id": tenant_id.0.to_string() })
            .await?
            .ok_or(StorageError::ApiKeyInvalid)?;

        let stored = tenant_doc
            .api_key_hmac
            .as_deref()
            .ok_or(StorageError::ApiKeyInvalid)?;

        if !auth.verify(raw_key, stored) {
            return Err(StorageError::ApiKeyInvalid);
        }
        if !tenant_doc.is_active {
            return Err(StorageError::ApiKeyInvalid);
        }

        let tenant = tenant_doc.into_domain()?;
        Ok(TenantContext::new(tenant.tenant_id))
    }
}

/// Parse the embedded tenant id from `sk_live_{32_hex}_{random}` — the
/// first 32 chars after `sk_live_` are the UUID without dashes.
fn parse_api_key_tenant(raw: &str) -> Option<TenantId> {
    let rest = raw.strip_prefix("sk_live_")?;
    // Expect `{32-hex}_{random}` — require the underscore so a short key
    // can't match.
    let (hex32, _random) = rest.split_once('_')?;
    if hex32.len() != 32 || !hex32.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let uuid = uuid::Uuid::parse_str(hex32).ok()?;
    Some(TenantId(uuid))
}

/// Test-only helper. Compiled only under `cfg(test)`, so it is invisible
/// to any other workspace member's build graph — even when they run
/// `cargo test --workspace` with feature unification enabled.
#[cfg(test)]
pub(crate) fn test_context(tenant_id: TenantId) -> TenantContext {
    TenantContext::new(tenant_id)
}

/// Trivial tier lookup — for now every new tenant starts on trial. Kept as
/// a function to give us a single place to add trial-end / upgrade logic.
#[allow(dead_code)]
pub(crate) fn default_tier() -> TenantTier {
    TenantTier::Trial
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_master_key() -> MasterKey {
        MasterKey::from_bytes(vec![0x42; 32]).unwrap()
    }

    #[test]
    fn master_key_rejects_too_short() {
        assert!(MasterKey::from_bytes(vec![0u8; 16]).is_err());
        assert!(MasterKey::from_config_str("short").is_err());
    }

    #[test]
    fn hmac_is_deterministic_and_constant_time_verify() {
        let auth = ApiKeyAuth::new(test_master_key());
        let raw = "sk_live_dead-beef-dead-beef-dead-beef-00_xyz";
        let h1 = auth.hmac_hex(raw);
        let h2 = auth.hmac_hex(raw);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // 32 bytes -> 64 hex

        assert!(auth.verify(raw, &h1));
        assert!(!auth.verify(raw, "deadbeef"));
        assert!(!auth.verify(raw, &h1.replace('a', "b")));
    }

    #[test]
    fn generated_key_roundtrips_verification() {
        let auth = ApiKeyAuth::new(test_master_key());
        let tid = TenantId::new();
        let key = auth.generate(tid);
        assert!(key.raw.starts_with("sk_live_"));
        assert_eq!(key.last6.len(), 6);
        assert!(auth.verify(&key.raw, &key.hmac_hex));
        assert_eq!(parse_api_key_tenant(&key.raw), Some(tid));
    }

    #[test]
    fn parse_api_key_tenant_rejects_garbage() {
        assert!(parse_api_key_tenant("not_a_key").is_none());
        assert!(parse_api_key_tenant("sk_live_short_xxx").is_none());
        // 32 non-hex chars
        assert!(parse_api_key_tenant("sk_live_zzzz-zzzz-zzzz-zzzz-zzzz-zzzz-zz_xxx").is_none());
        // 32 hex but no underscore following
        assert!(parse_api_key_tenant("sk_live_dead-beef-dead-beef-dead-beef-00").is_none());
    }

    #[test]
    fn url_safe_no_pad_is_non_empty_and_alphabet() {
        let s = url_safe_no_pad(&[0xFF; 16]);
        assert!(!s.is_empty());
        assert!(s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn test_context_roundtrip() {
        let id = TenantId::new();
        let ctx = test_context(id);
        assert_eq!(ctx.tenant_id(), id);
    }
}
