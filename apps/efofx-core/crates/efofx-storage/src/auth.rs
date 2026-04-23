//! The single blessed path for minting a [`TenantContext`].
//!
//! Phase 0 scaffolding: real JWKS verification and API-key lookup land in
//! Phase 2A. Until then, this module exposes only the structural shape —
//! nothing in the crate API lets downstream code construct a context
//! without calling one of these verified-entry functions.

use crate::TenantContext;

#[cfg(test)]
use efofx_domain::TenantId;

/// Verified-Supabase-JWT → tenant context. Phase 2A fills in the real JWT
/// verification; this stub enforces only the signature so downstream code
/// already depends on the correct entry point.
///
/// # Panics in stub form
/// Unimplemented until Phase 2A. Do not wire handlers to this yet.
pub fn tenant_from_supabase_claims(_jwt_sub: &str) -> TenantContext {
    // Phase 2A: look up tenant by supabase_user_id, provision if first-seen.
    unimplemented!("Phase 2A: Supabase JWT → tenant lookup not yet implemented");
}

/// Widget-API-key → tenant context. Phase 2A fills in the hash lookup.
///
/// # Panics in stub form
/// Unimplemented until Phase 2A.
pub fn tenant_from_api_key(_raw_key: &str) -> TenantContext {
    // Phase 2A: hash + lookup + construct context.
    unimplemented!("Phase 2A: widget API key → tenant lookup not yet implemented");
}

/// Test-only helper. Compiled only under `cfg(test)`, so it is invisible
/// to any other workspace member's build graph — even when they run
/// `cargo test --workspace` with feature unification enabled.
#[cfg(test)]
pub(crate) fn test_context(tenant_id: TenantId) -> TenantContext {
    TenantContext::new(tenant_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_roundtrip() {
        let id = TenantId::new();
        let ctx = test_context(id);
        assert_eq!(ctx.tenant_id(), id);
    }
}
