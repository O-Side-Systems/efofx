use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;

use crate::ids::TenantId;

/// Subscription tier. Drives rate limits, model selection, and platform-key
/// fallback behavior (see config `features.platform_key_fallback`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TenantTier {
    Trial,
    Alpha,
    Paid,
}

impl Default for TenantTier {
    fn default() -> Self {
        Self::Trial
    }
}

/// Tenant record. Linked to a Supabase user via `supabase_user_id`.
///
/// Credentials (email, password, email_verified flag) live in Supabase; this
/// record stores only domain state the platform core owns.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Tenant {
    pub tenant_id: TenantId,
    pub supabase_user_id: String,
    pub company_name: String,
    pub email: String,
    pub tier: TenantTier,

    /// Last 6 chars of the raw widget API key, for masked display.
    #[serde(default)]
    pub api_key_last6: Option<String>,

    /// Fernet ciphertext of the tenant's BYOK OpenAI key. Decrypted only
    /// within a request scope; never logged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_openai_key: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openai_key_last6: Option<String>,

    #[serde(default = "default_true")]
    pub is_active: bool,

    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,

    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

fn default_true() -> bool {
    true
}
