use efofx_domain::TenantId;

/// A verified tenant principal. Required by every tenant-scoped repository
/// method.
///
/// ## Compile-time isolation guarantee
///
/// The constructor is [`pub(crate)`], which means only code inside this
/// crate (`efofx-storage`) can build one. The server binary goes through
/// the [`auth`](crate::auth) module — the only blessed path.
///
/// Consumers outside the crate *cannot* bypass auth to mint a context:
/// the constructor is simply not visible to them. A compile-fail test in
/// `tests/compile-fail/` pins this invariant.
#[derive(Debug, Clone)]
pub struct TenantContext {
    tenant_id: TenantId,
}

impl TenantContext {
    /// Build a verified tenant context.
    ///
    /// Intentionally `pub(crate)` — only the [`auth`](crate::auth) module
    /// and its sibling modules can call this. Callers outside the crate
    /// must go through an auth-verification entry point.
    //
    // Allowed dead-code: the auth module's stub functions panic before
    // calling new() in Phase 0. Phase 2A wires the real call sites.
    #[allow(dead_code)]
    pub(crate) fn new(tenant_id: TenantId) -> Self {
        Self { tenant_id }
    }

    pub fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }
}
