//! MongoDB adapter + tenant-aware repository primitives.
//!
//! ## Tenant isolation
//!
//! [`TenantContext`]'s constructor is `pub(crate)`. The only way to obtain
//! one outside this crate is to call a verified-auth entry point in the
//! [`auth`] module. The compiler refuses any attempt to construct a
//! context directly from downstream code — see
//! `tests/compile-fail/` for negative coverage.

pub mod auth;
pub mod chat;
pub mod estimation;
pub mod health;
pub mod mongo;
pub mod reference;
pub mod tenant_context;
pub mod tenants;
pub mod widget;

pub use chat::{ChatRepo, DEFAULT_SESSION_TTL_HOURS};
pub use estimation::EstimationRepo;
pub use health::HealthStatus;
pub use mongo::MongoAdapter;
pub use reference::{decode_reference_class, ReferenceRepo, UpsertStats};
pub use tenant_context::TenantContext;
pub use tenants::{BrandingWithOrigins, TenantRepo};
pub use widget::{NewConsultation, NewLead, WidgetLeadRepo};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error(transparent)]
    Mongo(#[from] mongodb::error::Error),
    #[error("not found")]
    NotFound,
    #[error("widget API key invalid or not found")]
    ApiKeyInvalid,
    #[error("master encryption key must be at least 32 bytes")]
    MasterKeyTooShort,
    #[error("bson serialization failed: {0}")]
    Bson(String),
}
