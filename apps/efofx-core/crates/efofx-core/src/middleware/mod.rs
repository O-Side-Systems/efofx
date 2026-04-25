//! Cross-cutting Axum middleware.
//!
//! * [`rate_limit`] — IP-keyed request throttling for the public widget
//!   surface (branding fetch + analytics read).
//! * [`tenant_cors`] — runtime-mutable allowlist of origins per tenant,
//!   used by the widget-API-key surface.

pub mod rate_limit;
pub mod tenant_cors;
