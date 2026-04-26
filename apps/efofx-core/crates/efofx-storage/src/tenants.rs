//! Tenants collection: storage DTO and tenant-scoped repository.
//!
//! We keep a dedicated [`TenantDoc`] shape because BSON's default
//! serde-uuid mapping would encode `TenantId` as BSON binary, which
//! mismatches the existing Python schema (string UUIDs in all collections).
//! The DTO pins the wire format to strings and converts in/out of the
//! domain [`Tenant`] on the boundaries.
//!
//! Every method that reads or writes a tenant-scoped field takes a
//! [`TenantContext`] — the context's `tenant_id` is always used as the
//! filter so no caller can accidentally cross tenants. The exception is
//! [`TenantRepo::find_by_supabase_user`], which is only reachable from
//! [`crate::auth::TenantResolver`] (same crate) and gates the provisioning
//! upsert.

use std::time::SystemTime;

use mongodb::bson::{self, doc, DateTime as BsonDateTime, Document};
use mongodb::options::ReturnDocument;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use efofx_domain::{BrandingConfig, Tenant, TenantId, TenantTier};

use crate::{MongoAdapter, StorageError, TenantContext};

pub(crate) const TENANTS_COLLECTION: &str = "tenants";

/// Storage-layer tenant document. All identifiers are stored as strings to
/// match the FastAPI schema; BSON-binary UUIDs would break cross-stack
/// reads during migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TenantDoc {
    pub tenant_id: String,
    pub supabase_user_id: String,
    pub company_name: String,
    pub email: String,
    pub tier: String,

    #[serde(default)]
    pub api_key_last6: Option<String>,
    #[serde(default)]
    pub api_key_hmac: Option<String>,

    #[serde(default)]
    pub encrypted_openai_key: Option<String>,
    #[serde(default)]
    pub openai_key_last6: Option<String>,

    #[serde(default = "default_true")]
    pub is_active: bool,

    /// Tenant-owned settings document. Populated by the dashboard over
    /// `PATCH /v1/me`; read by the widget branding + CORS surfaces.
    /// `None` (or missing field) means every consumer falls back to
    /// defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<TenantSettingsDoc>,

    pub created_at: BsonDateTime,
    pub updated_at: BsonDateTime,
}

/// Subdoc persisted under `tenants.settings`. Everything here is
/// tenant-owned and user-configurable; no secrets.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TenantSettingsDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branding: Option<BrandingConfig>,
    /// Origins this tenant's widget is permitted to be embedded on. Used
    /// by the per-tenant CORS middleware (Phase 2D.5) to reflect
    /// allow-origin on preflight and actual responses.
    #[serde(default)]
    pub allowed_origins: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl TenantDoc {
    pub(crate) fn into_domain(self) -> Result<Tenant, StorageError> {
        let tenant_id: TenantId = self
            .tenant_id
            .parse::<uuid::Uuid>()
            .map(TenantId)
            .map_err(|e| StorageError::Bson(format!("tenant_id not a uuid: {e}")))?;
        let tier = match self.tier.as_str() {
            "trial" => TenantTier::Trial,
            "alpha" => TenantTier::Alpha,
            "paid" => TenantTier::Paid,
            other => return Err(StorageError::Bson(format!("unknown tenant tier: {other}"))),
        };
        let created_at = bson_to_offset(self.created_at)?;
        let updated_at = bson_to_offset(self.updated_at)?;
        Ok(Tenant {
            tenant_id,
            supabase_user_id: self.supabase_user_id,
            company_name: self.company_name,
            email: self.email,
            tier,
            api_key_last6: self.api_key_last6,
            api_key_hmac: self.api_key_hmac,
            encrypted_openai_key: self.encrypted_openai_key,
            openai_key_last6: self.openai_key_last6,
            is_active: self.is_active,
            created_at,
            updated_at,
        })
    }
}

fn bson_to_offset(dt: BsonDateTime) -> Result<OffsetDateTime, StorageError> {
    OffsetDateTime::from_unix_timestamp_nanos(dt.timestamp_millis() as i128 * 1_000_000)
        .map_err(|e| StorageError::Bson(format!("timestamp out of range: {e}")))
}

pub(crate) fn now_bson() -> (OffsetDateTime, BsonDateTime) {
    let now = OffsetDateTime::now_utc();
    let bson_now = BsonDateTime::from_system_time(SystemTime::from(now));
    (now, bson_now)
}

/// Profile patch accepted from `PATCH /v1/me`. Email and password are
/// intentionally absent — those live in Supabase.
#[derive(Debug, Clone, Default)]
pub struct TenantProfilePatch {
    pub company_name: Option<String>,
}

/// Summary of a tenant's stored BYOK OpenAI key, without decrypting.
#[derive(Debug, Clone)]
pub struct OpenAiKeyStatus {
    pub has_key: bool,
    pub last6: Option<String>,
}

/// Tenant-scoped repository. Every reader/writer takes a
/// [`TenantContext`]; the only way outside this crate to obtain one is to
/// go through [`crate::auth`].
#[derive(Clone)]
pub struct TenantRepo {
    mongo: MongoAdapter,
}

impl TenantRepo {
    pub fn new(mongo: MongoAdapter) -> Self {
        Self { mongo }
    }

    fn collection(&self) -> mongodb::Collection<TenantDoc> {
        self.mongo.database().collection(TENANTS_COLLECTION)
    }

    fn docs_collection(&self) -> mongodb::Collection<Document> {
        self.mongo.database().collection(TENANTS_COLLECTION)
    }

    /// Get the full tenant record for the authenticated principal.
    pub async fn get(&self, ctx: &TenantContext) -> Result<Tenant, StorageError> {
        let doc = self
            .collection()
            .find_one(doc! { "tenant_id": ctx.tenant_id().to_string() })
            .await?
            .ok_or(StorageError::NotFound)?;
        doc.into_domain()
    }

    /// Update tenant-owned profile fields. Returns the fresh record.
    pub async fn update_profile(
        &self,
        ctx: &TenantContext,
        patch: &TenantProfilePatch,
    ) -> Result<Tenant, StorageError> {
        let (_, bson_now) = now_bson();
        let mut set_doc = doc! { "updated_at": bson_now };
        if let Some(ref cn) = patch.company_name {
            set_doc.insert("company_name", cn);
        }

        let update = doc! { "$set": set_doc };
        let opts = mongodb::options::FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::After)
            .build();
        let updated = self
            .collection()
            .find_one_and_update(doc! { "tenant_id": ctx.tenant_id().to_string() }, update)
            .with_options(opts)
            .await?
            .ok_or(StorageError::NotFound)?;
        updated.into_domain()
    }

    /// Persist a fresh widget API key HMAC + last6. Called by the rotate
    /// endpoint; the raw plaintext is returned to the user once and never
    /// stored.
    pub async fn set_api_key(
        &self,
        ctx: &TenantContext,
        hmac_hex: &str,
        last6: &str,
    ) -> Result<(), StorageError> {
        let (_, bson_now) = now_bson();
        self.docs_collection()
            .update_one(
                doc! { "tenant_id": ctx.tenant_id().to_string() },
                doc! {
                    "$set": {
                        "api_key_hmac": hmac_hex,
                        "api_key_last6": last6,
                        "updated_at": bson_now,
                    }
                },
            )
            .await?;
        Ok(())
    }

    /// Persist encrypted BYOK OpenAI key ciphertext and its last-6 for
    /// masked display.
    pub async fn set_openai_key(
        &self,
        ctx: &TenantContext,
        encrypted: &str,
        last6: &str,
    ) -> Result<(), StorageError> {
        let (_, bson_now) = now_bson();
        self.docs_collection()
            .update_one(
                doc! { "tenant_id": ctx.tenant_id().to_string() },
                doc! {
                    "$set": {
                        "encrypted_openai_key": encrypted,
                        "openai_key_last6": last6,
                        "updated_at": bson_now,
                    }
                },
            )
            .await?;
        Ok(())
    }

    /// Clear the stored BYOK OpenAI key. Idempotent.
    pub async fn clear_openai_key(&self, ctx: &TenantContext) -> Result<(), StorageError> {
        let (_, bson_now) = now_bson();
        self.docs_collection()
            .update_one(
                doc! { "tenant_id": ctx.tenant_id().to_string() },
                doc! {
                    "$set": {
                        "encrypted_openai_key": bson::Bson::Null,
                        "openai_key_last6": bson::Bson::Null,
                        "updated_at": bson_now,
                    }
                },
            )
            .await?;
        Ok(())
    }

    /// Return the BYOK key status without decrypting the ciphertext.
    pub async fn openai_key_status(
        &self,
        ctx: &TenantContext,
    ) -> Result<OpenAiKeyStatus, StorageError> {
        let tenant = self.get(ctx).await?;
        Ok(OpenAiKeyStatus {
            has_key: tenant.encrypted_openai_key.is_some(),
            last6: tenant.openai_key_last6,
        })
    }

    /// Fetch the encrypted BYOK ciphertext for decryption in request scope.
    pub async fn fetch_openai_ciphertext(
        &self,
        ctx: &TenantContext,
    ) -> Result<Option<String>, StorageError> {
        let tenant = self.get(ctx).await?;
        Ok(tenant.encrypted_openai_key)
    }

    /// Fetch only `tenants.settings.allowed_origins` for the
    /// authenticated tenant. Used by the per-tenant CORS middleware to
    /// seed its origin cache after the widget-API-key auth resolves a
    /// tenant; cheap projection so the hot path never decodes the full
    /// tenant doc.
    pub async fn fetch_allowed_origins(
        &self,
        ctx: &TenantContext,
    ) -> Result<Vec<String>, StorageError> {
        let projection = doc! { "settings.allowed_origins": 1, "_id": 0 };
        let opts = mongodb::options::FindOneOptions::builder()
            .projection(projection)
            .build();
        let maybe_doc = self
            .docs_collection()
            .find_one(doc! { "tenant_id": ctx.tenant_id().to_string() })
            .with_options(opts)
            .await?;
        let Some(doc) = maybe_doc else {
            return Ok(Vec::new());
        };
        let Some(settings) = doc.get_document("settings").ok() else {
            return Ok(Vec::new());
        };
        let Some(arr) = settings.get_array("allowed_origins").ok() else {
            return Ok(Vec::new());
        };
        Ok(arr
            .iter()
            .filter_map(|b| b.as_str().map(|s| s.to_string()))
            .collect())
    }

    /// Resolve a tenant's branding by the public API-key prefix — the
    /// 32-hex `tenant_id` without dashes that appears after `sk_live_` in
    /// their widget key. Used by the public, unauthenticated
    /// `GET /v1/widget/branding/{prefix}` endpoint (Phase 2D.1).
    ///
    /// Returns `None` when:
    /// * the prefix is not 32 hex chars,
    /// * the prefix does not parse as a UUID,
    /// * no tenant exists for that id,
    /// * or the tenant is deactivated.
    ///
    /// When present, `BrandingConfig::company_name` falls back to the
    /// tenant's top-level `company_name` if the tenant has no stored
    /// branding override (matching FastAPI behaviour).
    pub async fn fetch_branding_by_prefix(
        &self,
        api_key_prefix: &str,
    ) -> Result<Option<BrandingWithOrigins>, StorageError> {
        let Some(tenant_id) = parse_tenant_id_prefix(api_key_prefix) else {
            return Ok(None);
        };
        let tenant_doc = match self
            .collection()
            .find_one(doc! { "tenant_id": tenant_id.0.to_string() })
            .await?
        {
            Some(doc) => doc,
            None => return Ok(None),
        };
        if !tenant_doc.is_active {
            return Ok(None);
        }

        let fallback_company = tenant_doc.company_name.clone();
        let (branding, allowed_origins) = match tenant_doc.settings {
            Some(s) => (s.branding, s.allowed_origins),
            None => (None, Vec::new()),
        };

        let branding = match branding {
            Some(mut b) => {
                if b.company_name.is_empty() {
                    b.company_name = fallback_company;
                }
                b
            }
            None => BrandingConfig {
                company_name: fallback_company,
                ..BrandingConfig::default()
            },
        };

        Ok(Some(BrandingWithOrigins {
            tenant_id,
            branding,
            allowed_origins,
        }))
    }

    /// Resolve branding by raw `tenant_id` string — used by the public,
    /// token-gated feedback form route which has no [`TenantContext`]
    /// (the magic-link doc carries the tenant id forward).
    ///
    /// Same fallback semantics as
    /// [`Self::fetch_branding_by_prefix`]: empty `company_name` is
    /// filled from the top-level tenant doc; deactivated tenants
    /// resolve to `None`.
    pub async fn fetch_branding_by_tenant_id(
        &self,
        tenant_id: &str,
    ) -> Result<Option<BrandingConfig>, StorageError> {
        let tenant_doc = match self
            .collection()
            .find_one(doc! { "tenant_id": tenant_id })
            .await?
        {
            Some(doc) => doc,
            None => return Ok(None),
        };
        if !tenant_doc.is_active {
            return Ok(None);
        }

        let fallback_company = tenant_doc.company_name.clone();
        let branding = tenant_doc.settings.and_then(|s| s.branding);
        let branding = match branding {
            Some(mut b) => {
                if b.company_name.is_empty() {
                    b.company_name = fallback_company;
                }
                b
            }
            None => BrandingConfig {
                company_name: fallback_company,
                ..BrandingConfig::default()
            },
        };
        Ok(Some(branding))
    }
}

/// Result of [`TenantRepo::fetch_branding_by_prefix`]. Carries the
/// `allowed_origins` so the CORS middleware can populate its cache on
/// the same lookup — avoids a second read per branding fetch.
#[derive(Debug, Clone)]
pub struct BrandingWithOrigins {
    pub tenant_id: TenantId,
    pub branding: BrandingConfig,
    pub allowed_origins: Vec<String>,
}

/// Parse the public 32-hex tenant-id prefix used by
/// `GET /v1/widget/branding/{prefix}`. Matches the `simple` UUID
/// formatting used by [`crate::auth::ApiKeyAuth::generate`].
fn parse_tenant_id_prefix(prefix: &str) -> Option<TenantId> {
    if prefix.len() != 32 || !prefix.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    uuid::Uuid::parse_str(prefix).ok().map(TenantId)
}
