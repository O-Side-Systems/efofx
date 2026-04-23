//! OpenAPI building blocks: envelopes, error codes, and the security-scheme
//! modifier. The `ApiDoc` struct lives in the binary crate (`efofx-core`)
//! because utoipa's `paths(...)` macro needs real handler-function paths,
//! which would create a circular dependency back to this crate.

pub mod envelope;
pub mod errors;
pub mod security;

pub use envelope::{ApiError, ApiErrorDetail, ApiResponse, ResponseMeta};
pub use errors::ErrorCode;
pub use security::SecurityAddon;
