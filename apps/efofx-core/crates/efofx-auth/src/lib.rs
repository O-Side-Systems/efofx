//! Authentication middleware for the efofx platform core.
//!
//! Two entry points, one output:
//! - [`middleware::supabase_jwt`] verifies a Supabase-issued JWT against the
//!   project JWKS and mints a [`TenantContext`] for the caller.
//! - [`middleware::widget_api_key`] verifies a widget `sk_live_*` key via
//!   HMAC-SHA256 constant-time compare and mints a [`TenantContext`].
//!
//! Downstream handlers accept the context via `axum::Extension<TenantContext>`
//! and have no way to tell which surface authenticated the request — that
//! is the point of unifying them here.
//!
//! The [`TenantContext`] type lives in `efofx-storage` and has a
//! `pub(crate)` constructor; this crate calls blessed resolver functions in
//! [`efofx_storage::auth`] which are the only place in the workspace that
//! is permitted to mint one.

pub mod claims;
pub mod error;
pub mod jwks;
pub mod middleware;

pub use claims::SupabaseClaims;
pub use error::AuthError;
pub use jwks::JwksCache;

/// Re-export the context so handlers can import it from one place.
pub use efofx_storage::TenantContext;
