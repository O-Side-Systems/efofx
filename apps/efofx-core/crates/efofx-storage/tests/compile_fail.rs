//! Tenant-isolation boundary enforced at compile time.
//!
//! These tests use `trybuild` to prove that code attempting to construct a
//! `TenantContext` from outside the `efofx-storage` crate FAILS to compile.
//! The constructor is `pub(crate)`; only the [`auth`] module can mint one.

#[test]
fn cannot_construct_tenant_context_without_auth() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile-fail/*.rs");
}
