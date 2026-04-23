//! Supabase JWT claim shape.
//!
//! We only require the fields we actually consume. Supabase ships many more
//! claims (`role`, `aal`, `amr`, `session_id`, etc.); we deserialize just
//! what we need and let `serde` ignore the rest.

use serde::Deserialize;

/// Claims we extract from a verified Supabase JWT. `aud`, `iss`, `exp`,
/// `nbf` are checked by `jsonwebtoken::Validation` and therefore do not need
/// to appear on this struct.
#[derive(Debug, Clone, Deserialize)]
pub struct SupabaseClaims {
    /// Supabase user id (UUID string). Stable per user.
    pub sub: String,
    /// Email claim. Required — tenant provisioning uses it as a fallback for
    /// `company_name` and stores it verbatim on the tenant record.
    pub email: String,
    /// Supabase-side user metadata set at sign-up time. We read
    /// `company_name` here; additional keys are ignored.
    #[serde(default)]
    pub user_metadata: UserMetadata,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UserMetadata {
    #[serde(default)]
    pub company_name: Option<String>,
}
