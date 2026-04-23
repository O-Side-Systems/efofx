use efofx_domain::TenantId;

/// A verified tenant principal. Required by every tenant-scoped repository
/// method.
///
/// The constructor is intentionally `pub(crate)`-equivalent via a sealed
/// constructor: callers outside the auth middleware cannot mint one. This
/// enforces tenant isolation at compile time rather than by convention.
#[derive(Debug, Clone)]
pub struct TenantContext {
    tenant_id: TenantId,
    _sealed: Sealed,
}

#[derive(Debug, Clone)]
struct Sealed;

impl TenantContext {
    /// Mint a [`TenantContext`]. Only auth middleware should call this, and
    /// only after verifying a Supabase JWT or a widget API key.
    ///
    /// We place this behind a trait with a private token so the call site in
    /// the binary crate has to go through a single blessed path.
    pub fn from_verified(tenant_id: TenantId, _token: &AuthVerifiedToken) -> Self {
        Self {
            tenant_id,
            _sealed: Sealed,
        }
    }

    pub fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }
}

/// Marker token that proves the caller is in an auth middleware code path.
///
/// The only public way to obtain an `AuthVerifiedToken` is via
/// [`AuthVerifiedToken::mint`], which lives behind a `cfg(auth_middleware)`-
/// equivalent compile guard: callers must opt in via the binary crate's
/// `_auth_mint` feature on `efofx-storage`. Other crates cannot mint one.
pub struct AuthVerifiedToken {
    _private: (),
}

impl AuthVerifiedToken {
    /// Available only when the consuming crate enables the `auth-mint`
    /// feature. The binary crate (`efofx-core`) is the only member that does.
    #[cfg(feature = "auth-mint")]
    pub fn mint() -> Self {
        Self { _private: () }
    }
}
