//! OpenAPI document assembly.
//!
//! The `ApiDoc` struct is the single authoritative OpenAPI source. Handlers
//! register their paths and schemas via `utoipa::path` attributes and are
//! added to this registry. The server serves the resulting document at
//! `/openapi.json`.

use utoipa::OpenApi;

use efofx_domain::{Tenant, TenantTier};
use efofx_storage::HealthStatus;

pub mod envelope;

pub use envelope::{ApiError, ApiErrorDetail, ApiResponse, ResponseMeta};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "efofx Platform Core API",
        version = "0.1.0",
        description = "Authoritative HTTP API for the efofx platform. All externally consumed endpoints are documented here."
    ),
    components(schemas(
        Tenant,
        TenantTier,
        HealthStatus,
        ApiError,
        ApiErrorDetail,
        ResponseMeta,
    )),
    tags(
        (name = "system", description = "Health and metadata endpoints"),
    )
)]
pub struct ApiDoc;
