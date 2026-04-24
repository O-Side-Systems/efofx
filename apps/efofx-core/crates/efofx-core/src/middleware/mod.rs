//! Cross-cutting Axum middleware.
//!
//! Phase 2D introduces two concerns:
//! * [`rate_limit`] — IP-keyed request throttling for the public widget
//!   surface (branding fetch today; analytics read in 2D.4).
//! * tenant-scoped CORS — ships with 2D.5.

pub mod rate_limit;
