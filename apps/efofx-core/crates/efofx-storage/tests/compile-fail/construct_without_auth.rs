// Proves that downstream code cannot construct a TenantContext without
// going through the auth module — the constructor is pub(crate) to
// efofx-storage. If this ever compiles, tenant isolation has regressed.

use efofx_domain::{ids::TenantId, Tenant};
use efofx_storage::TenantContext;

fn main() {
    let _id = TenantId::new();

    // TenantContext::new is pub(crate). External code cannot call it.
    let _ctx = TenantContext::new(_id);

    // And there is no other public constructor on TenantContext.
    // The only public mint path (`efofx_storage::auth::*`) does real JWT
    // or API-key verification and cannot be short-circuited.

    // Silence unused-import diagnostics that would otherwise mask the
    // isolation check we're actually testing.
    let _: Option<Tenant> = None;
}
