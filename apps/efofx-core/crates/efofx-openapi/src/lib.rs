//! OpenAPI document assembly.
//!
//! The `ApiDoc` struct is the single authoritative OpenAPI source. Handlers
//! register their paths and schemas via `utoipa::path` attributes and are
//! added to this registry. The server serves the resulting document at
//! `/openapi.json`.

use utoipa::OpenApi;

use efofx_domain::{
    AdjustmentFactor, AnalyticsEventType, BrandingConfig, ChatMessage, ChatSession, ChatStatus,
    ConsultationRequest, CostBreakdownCategory, CostCategoryEstimate, DiscrepancyReason,
    EstimateSnapshot, EstimationOutput, EstimationStatus, FeedbackDocument, FeedbackSubmission,
    FeedbackSummary, Lead, LeadStatus, MessageRole, ReferenceClass, ReferenceClassCategory, Region,
    ScopingContext, Tenant, TenantTier,
};
use efofx_storage::HealthStatus;

pub mod envelope;
pub mod errors;

pub use envelope::{ApiError, ApiErrorDetail, ApiResponse, ResponseMeta};
pub use errors::ErrorCode;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "efofx Platform Core API",
        version = "0.1.0",
        description = "Authoritative HTTP API for the efofx platform. All externally consumed endpoints are documented here."
    ),
    components(schemas(
        // Envelopes
        ApiError,
        ApiErrorDetail,
        ResponseMeta,
        // System
        HealthStatus,
        // Identity
        Tenant,
        TenantTier,
        // Chat / scoping
        ChatStatus,
        MessageRole,
        ChatMessage,
        ChatSession,
        ScopingContext,
        // Estimation
        EstimationStatus,
        EstimationOutput,
        CostCategoryEstimate,
        CostBreakdownCategory,
        AdjustmentFactor,
        // Geo / reference
        Region,
        ReferenceClass,
        ReferenceClassCategory,
        // Widget
        BrandingConfig,
        Lead,
        LeadStatus,
        ConsultationRequest,
        AnalyticsEventType,
        // Feedback
        DiscrepancyReason,
        EstimateSnapshot,
        FeedbackSubmission,
        FeedbackDocument,
        FeedbackSummary,
    )),
    tags(
        (name = "system", description = "Health and metadata endpoints"),
        (name = "identity", description = "Authenticated principal — tenant profile and credentials"),
        (name = "chat", description = "Conversational intake sessions"),
        (name = "estimation", description = "Structured estimate generation"),
        (name = "widget", description = "Embeddable widget — public branding, authenticated lead capture, analytics"),
        (name = "leads", description = "Dashboard-facing lead management"),
        (name = "feedback", description = "Customer outcome feedback and calibration"),
        (name = "calibration", description = "Accuracy metrics and trends"),
        (name = "integration", description = "Partner / contractor-directory integrations"),
    )
)]
pub struct ApiDoc;
