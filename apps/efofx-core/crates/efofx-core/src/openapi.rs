//! OpenAPI document assembly. Single authoritative source for
//! `/openapi.json`. Every handler that uses `#[utoipa::path(...)]` must be
//! listed in `paths(...)`.

use utoipa::OpenApi;

use efofx_domain::{
    AdjustmentFactor, AnalyticsEventType, BrandingConfig, ChatMessage, ChatSession, ChatStatus,
    ConsultationRequest, CostBreakdownCategory, CostCategoryEstimate, CostDistribution,
    DiscrepancyReason, EstimateSnapshot, EstimationOutput, EstimationStatus, FeedbackDocument,
    FeedbackSubmission, FeedbackSummary, Lead, LeadStatus, MessageRole, ReferenceClass,
    ReferenceProject, Region, ScopingContext, Tenant, TenantTier, TimelineDistribution,
};
use efofx_openapi::{ApiError, ApiErrorDetail, ResponseMeta, SecurityAddon};
use efofx_storage::HealthStatus;

use crate::api;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "efofx Platform Core API",
        version = "0.1.0",
        description = "Authoritative HTTP API for the efofx platform. All externally consumed endpoints are documented here.",
    ),
    modifiers(&SecurityAddon),
    paths(
        // System
        crate::health,
        crate::openapi_json,
        // Identity
        api::identity::get_me,
        api::identity::patch_me,
        api::identity::put_openai_key,
        api::identity::get_openai_key_status,
        api::identity::delete_openai_key,
        api::identity::rotate_api_key,
        // Chat
        api::chat::create_session,
        api::chat::get_session_handler,
        api::chat::append_message,
        api::chat::generate_estimate,
        // Estimation
        api::estimation::get_estimation,
        // Widget
        api::widget::get_branding,
        api::widget::create_lead,
        api::widget::create_consultation,
        api::widget::record_event,
        api::widget::list_events,
        // Leads (dashboard)
        api::leads::list_leads,
        api::leads::get_lead,
        api::leads::patch_lead,
        // Feedback
        api::feedback::create_feedback,
        api::feedback::feedback_summary,
        api::feedback::request_email,
        api::feedback::render_form,
        api::feedback::submit_form,
        // Calibration
        api::calibration::metrics,
        api::calibration::trend,
        // Integration
        api::integration::contractor_match,
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
        api::identity::UpdateTenantRequest,
        api::identity::UpdateTenantSettings,
        efofx_storage::tenants::RoutingConfig,
        api::identity::StoreOpenAiKeyRequest,
        api::identity::StoreOpenAiKeyResponse,
        api::identity::OpenAiKeyStatusResponse,
        api::identity::RotateApiKeyResponse,
        // Chat / scoping
        ChatStatus,
        MessageRole,
        ChatMessage,
        ChatSession,
        ScopingContext,
        api::chat::CreateChatSessionRequest,
        api::chat::AppendMessageRequest,
        api::chat::AppendMessageResponse,
        // Estimation
        EstimationStatus,
        EstimationOutput,
        CostCategoryEstimate,
        CostBreakdownCategory,
        AdjustmentFactor,
        api::estimation::EstimationSessionResponse,
        // Geo / reference
        Region,
        ReferenceClass,
        ReferenceProject,
        CostDistribution,
        TimelineDistribution,
        // Widget
        BrandingConfig,
        Lead,
        LeadStatus,
        ConsultationRequest,
        AnalyticsEventType,
        api::widget::LeadCaptureRequest,
        api::widget::LeadCaptureResponse,
        api::widget::ConsultationCapturedResponse,
        api::widget::AnalyticsEventRequest,
        api::widget::AnalyticsDailyBucket,
        api::widget::AnalyticsSummary,
        // Leads (dashboard)
        api::leads::LeadListResponse,
        api::leads::UpdateLeadRequest,
        // Feedback
        DiscrepancyReason,
        EstimateSnapshot,
        FeedbackSubmission,
        FeedbackDocument,
        FeedbackSummary,
        api::feedback::CreateFeedbackRequest,
        api::feedback::CreateFeedbackResponse,
        api::feedback::FeedbackEmailRequest,
        api::feedback::FeedbackEmailResponse,
        // Calibration
        api::calibration::AccuracyBuckets,
        api::calibration::ReferenceClassAccuracy,
        api::calibration::CalibrationMetrics,
        api::calibration::MetricsBelowThreshold,
        api::calibration::CalibrationTrendPoint,
        api::calibration::CalibrationTrend,
        api::calibration::TrendBelowThreshold,
        // Integration
        api::integration::ContractorMatchRequest,
        api::integration::ContractorMatchResponse,
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
    ),
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the size of the externally consumed API so a stray `paths(...)`
    /// edit doesn't accidentally drop coverage. Update the expected count
    /// only when intentionally adding or removing a path.
    #[test]
    fn openapi_contains_expected_paths() {
        let doc = ApiDoc::openapi();
        let paths: Vec<&str> = doc.paths.paths.keys().map(|s| s.as_str()).collect();

        for required in [
            "/health",
            "/openapi.json",
            "/v1/me",
            "/v1/me/openai-key",
            "/v1/me/openai-key/status",
            "/v1/me/api-keys:rotate",
            "/v1/chat/sessions",
            "/v1/chat/sessions/{session_id}",
            "/v1/chat/sessions/{session_id}/messages",
            "/v1/chat/sessions/{session_id}:generate-estimate",
            "/v1/estimates/{session_id}",
            "/v1/widget/branding/{api_key_prefix}",
            "/v1/widget/leads",
            "/v1/widget/consultations",
            "/v1/widget/events",
            "/v1/leads",
            "/v1/leads/{lead_id}",
            "/v1/feedback",
            "/v1/feedback/summary",
            "/v1/feedback/email-requests",
            "/v1/feedback/forms/{token}",
            "/v1/calibration/metrics",
            "/v1/calibration/trend",
            "/v1/integration/contractor-match",
        ] {
            assert!(
                paths.contains(&required),
                "missing expected path: {required}\nactual paths: {paths:#?}"
            );
        }
    }

    #[test]
    fn openapi_declares_both_security_schemes() {
        let doc = ApiDoc::openapi();
        let components = doc.components.expect("components present");
        assert!(
            components.security_schemes.contains_key("supabase_jwt"),
            "supabase_jwt security scheme missing"
        );
        assert!(
            components.security_schemes.contains_key("widget_api_key"),
            "widget_api_key security scheme missing"
        );
    }

    /// Pin the widget surface contract: per Phase 2D, every widget verb
    /// declares the right auth scheme + status codes. Schemas referenced
    /// by the widget DTOs are required components so the generated
    /// client stays generatable. Update this test only when the widget
    /// contract intentionally changes.
    #[test]
    fn openapi_widget_surface_contract() {
        let doc = ApiDoc::openapi();
        let path_item = |p: &str| {
            doc.paths
                .paths
                .get(p)
                .unwrap_or_else(|| panic!("missing path {p}"))
                .clone()
        };

        // Branding — public, 200 + 404 + 429
        let branding = path_item("/v1/widget/branding/{api_key_prefix}");
        let op = branding.get.as_ref().expect("branding GET op missing");
        assert!(op.security.is_none(), "branding must be public");
        let codes: Vec<&str> = op.responses.responses.keys().map(|s| s.as_str()).collect();
        for code in ["200", "404", "429"] {
            assert!(
                codes.contains(&code),
                "branding missing response {code}: {codes:?}"
            );
        }

        // Leads — widget API key, 201 + 400 + 401
        let leads = path_item("/v1/widget/leads");
        let op = leads.post.as_ref().expect("leads POST op missing");
        assert!(
            has_security(op, "widget_api_key"),
            "leads must require widget_api_key"
        );
        for code in ["201", "400", "401"] {
            assert!(
                op.responses.responses.contains_key(code),
                "leads missing response {code}"
            );
        }

        // Consultations — widget API key, 201 + 400 + 401
        let consult = path_item("/v1/widget/consultations");
        let op = consult
            .post
            .as_ref()
            .expect("consultations POST op missing");
        assert!(
            has_security(op, "widget_api_key"),
            "consultations must require widget_api_key"
        );
        for code in ["201", "400", "401"] {
            assert!(
                op.responses.responses.contains_key(code),
                "consultations missing response {code}"
            );
        }

        // Events — POST is widget API key (204), GET is supabase_jwt (200 + 429)
        let events = path_item("/v1/widget/events");
        let post_op = events.post.as_ref().expect("events POST op missing");
        assert!(
            has_security(post_op, "widget_api_key"),
            "POST events must require widget_api_key"
        );
        for code in ["204", "401"] {
            assert!(
                post_op.responses.responses.contains_key(code),
                "POST events missing response {code}"
            );
        }
        let get_op = events.get.as_ref().expect("events GET op missing");
        assert!(
            has_security(get_op, "supabase_jwt"),
            "GET events must require supabase_jwt"
        );
        for code in ["200", "401", "429"] {
            assert!(
                get_op.responses.responses.contains_key(code),
                "GET events missing response {code}"
            );
        }

        // Schemas referenced by the widget surface must all be present.
        let components = doc.components.as_ref().expect("components present");
        for schema in [
            "BrandingConfig",
            "ConsultationRequest",
            "AnalyticsEventType",
            "LeadCaptureRequest",
            "LeadCaptureResponse",
            "ConsultationCapturedResponse",
            "AnalyticsEventRequest",
            "AnalyticsDailyBucket",
            "AnalyticsSummary",
        ] {
            assert!(
                components.schemas.contains_key(schema),
                "missing widget schema: {schema}"
            );
        }
    }

    /// `value` on `SecurityRequirement` is private; round-trip through
    /// JSON to inspect declared schemes.
    fn has_security(op: &utoipa::openapi::path::Operation, scheme: &str) -> bool {
        let Some(reqs) = op.security.as_ref() else {
            return false;
        };
        reqs.iter().any(|r| {
            serde_json::to_value(r)
                .ok()
                .and_then(|v| v.as_object().map(|m| m.contains_key(scheme)))
                .unwrap_or(false)
        })
    }

    /// Pin the feedback surface contract: per Phase 2E, every feedback
    /// verb declares the right auth scheme + status codes. The public
    /// HTML form route is intentionally unauthenticated and returns
    /// `text/html`. Schemas referenced by the feedback DTOs are required
    /// components so the generated client stays generatable. Update this
    /// test only when the feedback contract intentionally changes.
    #[test]
    fn openapi_feedback_surface_contract() {
        let doc = ApiDoc::openapi();
        let path_item = |p: &str| {
            doc.paths
                .paths
                .get(p)
                .unwrap_or_else(|| panic!("missing path {p}"))
                .clone()
        };

        // POST /v1/feedback — supabase_jwt OR widget_api_key, 201/400/401
        let create = path_item("/v1/feedback");
        let op = create.post.as_ref().expect("POST /v1/feedback missing");
        assert!(
            has_security(op, "supabase_jwt"),
            "create_feedback must accept supabase_jwt"
        );
        assert!(
            has_security(op, "widget_api_key"),
            "create_feedback must accept widget_api_key"
        );
        for code in ["201", "400", "401"] {
            assert!(
                op.responses.responses.contains_key(code),
                "create_feedback missing response {code}"
            );
        }

        // GET /v1/feedback/summary — supabase_jwt only, 200/401
        let summary = path_item("/v1/feedback/summary");
        let op = summary.get.as_ref().expect("GET summary missing");
        assert!(
            has_security(op, "supabase_jwt"),
            "feedback_summary must require supabase_jwt"
        );
        assert!(
            !has_security(op, "widget_api_key"),
            "feedback_summary is dashboard-only — widget_api_key must not appear"
        );
        for code in ["200", "401"] {
            assert!(
                op.responses.responses.contains_key(code),
                "feedback_summary missing response {code}"
            );
        }

        // POST /v1/feedback/email-requests — supabase_jwt only, 202/400/401/404
        let email = path_item("/v1/feedback/email-requests");
        let op = email.post.as_ref().expect("POST email-requests missing");
        assert!(
            has_security(op, "supabase_jwt"),
            "request_email must require supabase_jwt"
        );
        assert!(
            !has_security(op, "widget_api_key"),
            "request_email is dashboard-only — widget_api_key must not appear"
        );
        for code in ["202", "400", "401", "404"] {
            assert!(
                op.responses.responses.contains_key(code),
                "request_email missing response {code}"
            );
        }

        // GET + POST /v1/feedback/forms/{token} — public, HTML responses.
        // The path uses the OpenAPI 3 `{token}` form even though the
        // axum router registers `:token` (axum 0.7 / matchit 0.7
        // constraint, see Phase 2D checkpoint).
        let form = path_item("/v1/feedback/forms/{token}");

        let get_op = form.get.as_ref().expect("GET form missing");
        assert!(
            get_op.security.is_none(),
            "render_form must be public (token-gated, no auth scheme)"
        );
        assert!(
            get_op.responses.responses.contains_key("200"),
            "render_form must declare 200 (HTML always returned, status encodes nothing)"
        );

        let post_op = form.post.as_ref().expect("POST form missing");
        assert!(
            post_op.security.is_none(),
            "submit_form must be public (token-gated, no auth scheme)"
        );
        for code in ["200", "400"] {
            assert!(
                post_op.responses.responses.contains_key(code),
                "submit_form missing response {code}"
            );
        }

        // Schemas referenced by the feedback surface must all be present.
        let components = doc.components.as_ref().expect("components present");
        for schema in [
            "DiscrepancyReason",
            "EstimateSnapshot",
            "FeedbackSubmission",
            "FeedbackDocument",
            "FeedbackSummary",
            "CreateFeedbackRequest",
            "CreateFeedbackResponse",
            "FeedbackEmailRequest",
            "FeedbackEmailResponse",
        ] {
            assert!(
                components.schemas.contains_key(schema),
                "missing feedback schema: {schema}"
            );
        }
    }

    /// Pin the calibration surface contract: per Phase 2E, both
    /// calibration verbs are dashboard-only (supabase_jwt) and declare
    /// the validation 400 envelope alongside the 401. Update this test
    /// only when the calibration contract intentionally changes.
    #[test]
    fn openapi_calibration_surface_contract() {
        let doc = ApiDoc::openapi();
        let path_item = |p: &str| {
            doc.paths
                .paths
                .get(p)
                .unwrap_or_else(|| panic!("missing path {p}"))
                .clone()
        };

        // GET /v1/calibration/metrics — supabase_jwt, 200/400/401
        let metrics = path_item("/v1/calibration/metrics");
        let op = metrics.get.as_ref().expect("GET metrics missing");
        assert!(
            has_security(op, "supabase_jwt"),
            "calibration metrics must require supabase_jwt"
        );
        assert!(
            !has_security(op, "widget_api_key"),
            "calibration metrics is dashboard-only — widget_api_key must not appear"
        );
        for code in ["200", "400", "401"] {
            assert!(
                op.responses.responses.contains_key(code),
                "calibration metrics missing response {code}"
            );
        }

        // GET /v1/calibration/trend — supabase_jwt, 200/400/401
        let trend = path_item("/v1/calibration/trend");
        let op = trend.get.as_ref().expect("GET trend missing");
        assert!(
            has_security(op, "supabase_jwt"),
            "calibration trend must require supabase_jwt"
        );
        assert!(
            !has_security(op, "widget_api_key"),
            "calibration trend is dashboard-only — widget_api_key must not appear"
        );
        for code in ["200", "400", "401"] {
            assert!(
                op.responses.responses.contains_key(code),
                "calibration trend missing response {code}"
            );
        }

        // Schemas referenced by the calibration surface must all be
        // present — including the below-threshold sibling types so the
        // wire union (currently expressed as parallel components) stays
        // generatable.
        let components = doc.components.as_ref().expect("components present");
        for schema in [
            "AccuracyBuckets",
            "ReferenceClassAccuracy",
            "CalibrationMetrics",
            "MetricsBelowThreshold",
            "CalibrationTrendPoint",
            "CalibrationTrend",
            "TrendBelowThreshold",
        ] {
            assert!(
                components.schemas.contains_key(schema),
                "missing calibration schema: {schema}"
            );
        }
    }

    /// Pin the integration surface contract: per Phase 2F, the
    /// contractor-match endpoint declares widget_api_key auth and the
    /// 200 / 400 / 401 / 404 envelope. The 501 response is gone — the
    /// stub has been replaced by a real handler. Schemas referenced by
    /// the integration DTOs are required components so the generated
    /// client stays generatable. Update this test only when the
    /// integration contract intentionally changes.
    #[test]
    fn openapi_integration_surface_contract() {
        let doc = ApiDoc::openapi();
        let path_item = |p: &str| {
            doc.paths
                .paths
                .get(p)
                .unwrap_or_else(|| panic!("missing path {p}"))
                .clone()
        };

        // POST /v1/integration/contractor-match — widget_api_key only,
        // 200 / 400 / 401 / 404. The previous 501 stub response must be
        // gone now that the handler is real.
        let path = path_item("/v1/integration/contractor-match");
        let op = path.post.as_ref().expect("POST contractor-match missing");
        assert!(
            has_security(op, "widget_api_key"),
            "contractor-match must require widget_api_key"
        );
        assert!(
            !has_security(op, "supabase_jwt"),
            "contractor-match is partner-facing — supabase_jwt must not appear"
        );
        for code in ["200", "400", "401", "404"] {
            assert!(
                op.responses.responses.contains_key(code),
                "contractor-match missing response {code}"
            );
        }
        assert!(
            !op.responses.responses.contains_key("501"),
            "contractor-match still advertises 501 — remove the stub response"
        );

        // Schemas referenced by the integration surface must all be
        // present.
        let components = doc.components.as_ref().expect("components present");
        for schema in ["ContractorMatchRequest", "ContractorMatchResponse"] {
            assert!(
                components.schemas.contains_key(schema),
                "missing integration schema: {schema}"
            );
        }
    }
}
