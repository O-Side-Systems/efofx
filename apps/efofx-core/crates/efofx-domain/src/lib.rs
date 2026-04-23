//! Pure domain types for the efofx platform core.
//!
//! This crate has no I/O dependencies. Every type here is a value object or enum
//! that other crates compose. Handlers, repositories, and adapters depend on
//! these types; this crate depends on none of them.

pub mod ids;
pub mod tenant;

pub use ids::{SessionId, TenantId};
pub use tenant::{Tenant, TenantTier};
