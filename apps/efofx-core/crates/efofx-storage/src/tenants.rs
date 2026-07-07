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

use std::collections::BTreeMap;
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
    /// Partner-routing configuration (Phase 2F). `None` means routing is
    /// disabled — the contractor-match endpoint and SSE `routing_tags`
    /// field still respond, but tags resolve to an empty list and the
    /// `directory_url` is `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing: Option<RoutingConfig>,
}

/// Per-tenant partner-routing configuration (Phase 2F.1).
///
/// Persisted under `tenants.settings.routing`. Read by the
/// `RoutingService` (tag derivation) and the `contractor_match`
/// handler (URL substitution). All fields are optional — an explicit
/// `RoutingConfig { enabled: false, .. }` and a missing `routing`
/// document are equivalent.
///
/// Tag overrides map a canonical tag string (e.g. `"pool"`) to the
/// partner's preferred wire string (e.g. `"swimming-pool"`). Cost-tier
/// breakpoints are inclusive lower bounds for `mid` and `high` —
/// values strictly below `cost_tier_breakpoints[0]` are `low`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RoutingConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Template URL with `{name}` placeholders. Recognised placeholders:
    /// `{tags}` (comma-joined), `{region}`, `{project_type}`,
    /// `{cost_tier}`. Unknown placeholders pass through untouched.
    /// `None` means no `directory_url` is returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory_url_template: Option<String>,
    /// Map from canonical tag (without prefix) to the partner's
    /// preferred wire string. Applied after tag derivation, before URL
    /// substitution.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tag_overrides: BTreeMap<String, String>,
    /// Inclusive lower bounds in dollars for the `mid` and `high`
    /// cost tiers. Defaults to `[25_000, 250_000]` when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_tier_breakpoints: Option<[u64; 2]>,
}

impl RoutingConfig {
    /// Effective breakpoints — `cost_tier_breakpoints` if set, else
    /// the documented defaults `[25_000, 250_000]`.
    pub fn effective_breakpoints(&self) -> [u64; 2] {
        self.cost_tier_breakpoints.unwrap_or([25_000, 250_000])
    }
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
///
/// Settings subfields are replace-if-present: an omitted field leaves the
/// stored value untouched, so a partial patch can never clobber sibling
/// settings (`$set` targets `settings.<field>`, not the whole subdoc).
#[derive(Debug, Clone, Default)]
pub struct TenantProfilePatch {
    pub company_name: Option<String>,
    pub branding: Option<BrandingConfig>,
    pub allowed_origins: Option<Vec<String>>,
    pub routing: Option<RoutingConfig>,
}

/// Build the `$set` document for [`TenantRepo::update_profile`]. Dotted
/// `settings.<field>` paths mean Mongo materialises the `settings` subdoc
/// on first write and sibling fields survive a partial patch.
fn profile_set_doc(
    patch: &TenantProfilePatch,
    bson_now: BsonDateTime,
) -> Result<Document, StorageError> {
    let mut set_doc = doc! { "updated_at": bson_now };
    if let Some(ref cn) = patch.company_name {
        set_doc.insert("company_name", cn);
    }
    if let Some(ref branding) = patch.branding {
        set_doc.insert("settings.branding", to_bson(branding)?);
    }
    if let Some(ref origins) = patch.allowed_origins {
        set_doc.insert("settings.allowed_origins", to_bson(origins)?);
    }
    if let Some(ref routing) = patch.routing {
        set_doc.insert("settings.routing", to_bson(routing)?);
    }
    Ok(set_doc)
}

fn to_bson<T: Serialize>(value: &T) -> Result<bson::Bson, StorageError> {
    bson::serialize_to_bson(value).map_err(|e| StorageError::Bson(e.to_string()))
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
        let update = doc! { "$set": profile_set_doc(patch, bson_now)? };
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

    /// Fetch the partner-routing config for the authenticated tenant
    /// (Phase 2F). Returns `None` when the tenant has no `settings`
    /// document or has not configured routing — the caller treats
    /// `None` as "routing disabled" and proceeds with empty tags +
    /// no `directory_url`.
    pub async fn fetch_routing_config(
        &self,
        ctx: &TenantContext,
    ) -> Result<Option<RoutingConfig>, StorageError> {
        let projection = doc! { "settings.routing": 1, "_id": 0 };
        let opts = mongodb::options::FindOneOptions::builder()
            .projection(projection)
            .build();
        let maybe_doc = self
            .docs_collection()
            .find_one(doc! { "tenant_id": ctx.tenant_id().to_string() })
            .with_options(opts)
            .await?;
        let Some(doc) = maybe_doc else {
            return Ok(None);
        };
        let Some(settings) = doc.get_document("settings").ok() else {
            return Ok(None);
        };
        let Some(routing_doc) = settings.get_document("routing").ok() else {
            return Ok(None);
        };
        let routing: RoutingConfig = bson::deserialize_from_document(routing_doc.clone())
            .map_err(|e| StorageError::Bson(format!("decode routing config: {e}")))?;
        Ok(Some(routing))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_config_default_is_disabled() {
        let cfg = RoutingConfig::default();
        assert!(!cfg.enabled);
        assert!(cfg.directory_url_template.is_none());
        assert!(cfg.tag_overrides.is_empty());
        assert!(cfg.cost_tier_breakpoints.is_none());
        // Default breakpoints fall through to the documented values so
        // the RoutingService doesn't need to know about defaulting.
        assert_eq!(cfg.effective_breakpoints(), [25_000, 250_000]);
    }

    #[test]
    fn routing_config_explicit_breakpoints_win() {
        let cfg = RoutingConfig {
            cost_tier_breakpoints: Some([10_000, 100_000]),
            ..RoutingConfig::default()
        };
        assert_eq!(cfg.effective_breakpoints(), [10_000, 100_000]);
    }

    #[test]
    fn routing_config_round_trip_full() {
        let mut overrides = BTreeMap::new();
        overrides.insert("pool".into(), "swimming-pool".into());
        overrides.insert("residential".into(), "single-family".into());

        let cfg = RoutingConfig {
            enabled: true,
            directory_url_template: Some(
                "https://partner.example/find?tags={tags}&region={region}".into(),
            ),
            tag_overrides: overrides,
            cost_tier_breakpoints: Some([15_000, 150_000]),
        };

        let bson = bson::serialize_to_document(&cfg).unwrap();
        let back: RoutingConfig = bson::deserialize_from_document(bson).unwrap();
        assert!(back.enabled);
        assert_eq!(
            back.directory_url_template.as_deref(),
            Some("https://partner.example/find?tags={tags}&region={region}")
        );
        assert_eq!(
            back.tag_overrides.get("pool").map(String::as_str),
            Some("swimming-pool")
        );
        assert_eq!(back.cost_tier_breakpoints, Some([15_000, 150_000]));
    }

    #[test]
    fn routing_config_omits_empty_optional_fields_on_serialize() {
        // Wire shape stays compact when routing is configured but most
        // fields are defaulted — matters for tenants that flip
        // `enabled = true` without customising anything else.
        let cfg = RoutingConfig {
            enabled: true,
            ..RoutingConfig::default()
        };
        let bson = bson::serialize_to_document(&cfg).unwrap();
        assert_eq!(bson.get_bool("enabled").unwrap(), true);
        assert!(!bson.contains_key("directory_url_template"));
        assert!(!bson.contains_key("tag_overrides"));
        assert!(!bson.contains_key("cost_tier_breakpoints"));
    }

    #[test]
    fn settings_doc_routing_field_optional_for_legacy_tenants() {
        // Tenants provisioned before 2F have no `settings.routing` field;
        // the default Deserialize must not fail.
        let bson = bson::doc! {
            "branding": null,
            "allowed_origins": [],
        };
        let settings: TenantSettingsDoc = bson::deserialize_from_document(bson).unwrap();
        assert!(settings.routing.is_none());
    }

    #[test]
    fn profile_set_doc_company_only_leaves_settings_untouched() {
        let (_, bson_now) = now_bson();
        let patch = TenantProfilePatch {
            company_name: Some("Acme".into()),
            ..TenantProfilePatch::default()
        };
        let set_doc = profile_set_doc(&patch, bson_now).unwrap();
        assert_eq!(set_doc.get_str("company_name").unwrap(), "Acme");
        assert!(!set_doc.keys().any(|k| k.starts_with("settings.")));
    }

    #[test]
    fn profile_set_doc_writes_dotted_settings_paths() {
        let (_, bson_now) = now_bson();
        let patch = TenantProfilePatch {
            company_name: None,
            branding: None,
            allowed_origins: Some(vec!["https://example.com".into()]),
            routing: Some(RoutingConfig {
                enabled: true,
                directory_url_template: Some("https://x/{tags}".into()),
                tag_overrides: BTreeMap::new(),
                cost_tier_breakpoints: Some([10_000, 100_000]),
            }),
        };
        let set_doc = profile_set_doc(&patch, bson_now).unwrap();
        // Dotted paths pinned: `fetch_allowed_origins` / `fetch_routing_config`
        // project on these exact keys.
        let origins = set_doc.get_array("settings.allowed_origins").unwrap();
        assert_eq!(origins.len(), 1);
        let routing = set_doc.get_document("settings.routing").unwrap();
        assert_eq!(routing.get_bool("enabled").unwrap(), true);
        assert!(!set_doc.contains_key("company_name"));
        assert!(!set_doc.contains_key("settings.branding"));
    }

    #[test]
    fn settings_doc_routing_round_trips_through_settings_subdoc() {
        let cfg = RoutingConfig {
            enabled: true,
            directory_url_template: Some("https://x/{cost_tier}".into()),
            tag_overrides: BTreeMap::new(),
            cost_tier_breakpoints: None,
        };
        let settings = TenantSettingsDoc {
            branding: None,
            allowed_origins: vec!["https://example.com".into()],
            routing: Some(cfg),
        };
        let bson = bson::serialize_to_document(&settings).unwrap();
        // Field name pinned — `fetch_routing_config` projects on
        // `settings.routing`, so a typo here would silently disable
        // routing for every tenant.
        assert!(bson.contains_key("routing"));
        let back: TenantSettingsDoc = bson::deserialize_from_document(bson).unwrap();
        let routing = back.routing.expect("routing round-trips");
        assert!(routing.enabled);
        assert_eq!(
            routing.directory_url_template.as_deref(),
            Some("https://x/{cost_tier}")
        );
    }
}
