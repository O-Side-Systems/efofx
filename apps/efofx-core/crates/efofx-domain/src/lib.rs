//! Pure domain types for the efofx platform core.
//!
//! This crate has no I/O dependencies. Every type here is a value object or
//! enum that other crates compose. Handlers, repositories, and adapters
//! depend on these types; this crate depends on none of them.
//!
//! Wire formats: every externally serialized enum uses `serde` renames that
//! match the FastAPI Pydantic output byte-for-byte. This is deliberate —
//! clients cutting over to Rust must not need to change deserialization
//! logic. See each enum's `#[serde(...)]` attributes for specifics.

pub mod analytics;
pub mod chat;
pub mod estimation;
pub mod feedback;
pub mod geo;
pub mod ids;
pub mod reference;
pub mod scoping;
pub mod tenant;
pub mod widget;

pub use analytics::AnalyticsEventType;
pub use chat::{ChatMessage, ChatSession, ChatStatus, MessageRole, ScopingContext, TokenUsage};
pub use estimation::{
    AdjustmentFactor, CostBreakdownCategory, CostCategoryEstimate, EstimationOutput,
    EstimationSession, EstimationSessionId, EstimationStatus,
};
pub use feedback::{
    DiscrepancyReason, EstimateSnapshot, FeedbackDocument, FeedbackSubmission, FeedbackSummary,
};
pub use geo::Region;
pub use ids::{SessionId, TenantId};
pub use reference::{CostDistribution, ReferenceClass, ReferenceProject, TimelineDistribution};
pub use scoping::{
    extract_scoping, is_confirmation, is_explicit_estimate_trigger, CONFIRMATION_WORDS,
    ESTIMATE_TRIGGER_PHRASES,
};
pub use tenant::{Tenant, TenantTier};
pub use widget::{BrandingConfig, ConsultationRequest, Lead, LeadStatus};
