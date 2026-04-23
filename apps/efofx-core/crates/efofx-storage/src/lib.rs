//! MongoDB adapter + tenant-aware repository primitives.
//!
//! ## Tenant isolation
//!
//! The [`TenantContext`] newtype has a private constructor. Every tenant-scoped
//! repository method requires `&TenantContext` in its signature. Because only
//! the auth middleware (in the `efofx-core` binary crate) can mint one, there
//! is no way to call a tenant repo without a verified principal — the compiler
//! refuses.
//!
//! See `tests/compile-fail/` for negative coverage: code that tries to
//! construct a context directly must fail to compile.

pub mod health;
pub mod mongo;
pub mod tenant_context;

pub use health::HealthStatus;
pub use mongo::MongoAdapter;
pub use tenant_context::TenantContext;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error(transparent)]
    Mongo(#[from] mongodb::error::Error),
    #[error("not found")]
    NotFound,
}
