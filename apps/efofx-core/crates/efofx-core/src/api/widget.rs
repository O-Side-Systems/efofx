//! Widget public + authenticated endpoints (Phase 2D).

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use efofx_auth::middleware::widget_api_key;
use efofx_domain::{AnalyticsEventType, BrandingConfig, ConsultationRequest, SessionId};
use efofx_email::EmailMessage;
use efofx_openapi::{ApiError, ErrorCode};
use efofx_storage::{NewConsultation, NewLead, TenantContext};

use crate::middleware::rate_limit::rate_limit;
use crate::AppState;

use super::not_implemented;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LeadCaptureRequest {
    pub session_id: String,
    /// Customer name, 1–200 chars.
    pub name: String,
    pub email: String,
    /// Phone number, 5–30 chars.
    pub phone: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LeadCaptureResponse {
    pub message: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ConsultationCapturedResponse {
    pub lead_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AnalyticsEventRequest {
    pub event_type: AnalyticsEventType,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AnalyticsDailyBucket {
    /// ISO date `YYYY-MM-DD`.
    pub date: String,
    pub widget_view: u64,
    pub chat_start: u64,
    pub estimate_complete: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AnalyticsSummary {
    pub days: u32,
    pub buckets: Vec<AnalyticsDailyBucket>,
}

/// Public: fetch a tenant's widget branding by their API-key prefix.
/// The prefix is the 32-hex `tenant_id` (no dashes) that appears after
/// `sk_live_` in the widget's stored API key. Rate-limited per IP
/// (30 req/min). Never returns secrets or PII.
#[utoipa::path(
    get,
    path = "/v1/widget/branding/{api_key_prefix}",
    tag = "widget",
    params(
        ("api_key_prefix" = String, Path, description = "32-hex tenant-id prefix (the segment after `sk_live_` in the widget API key)")
    ),
    responses(
        (status = 200, description = "Branding config", body = BrandingConfig),
        (status = 404, description = "No tenant matches that prefix", body = ApiError),
        (status = 429, description = "Rate limit exceeded", body = ApiError),
    ),
)]
pub async fn get_branding(
    State(state): State<Arc<AppState>>,
    Path(api_key_prefix): Path<String>,
) -> Response {
    match state
        .tenants
        .fetch_branding_by_prefix(&api_key_prefix)
        .await
    {
        Ok(Some(resolved)) => Json(resolved.branding).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiError::single(
                ErrorCode::WidgetBrandingNotFound.as_str(),
                ErrorCode::WidgetBrandingNotFound.default_message(),
            )),
        )
            .into_response(),
        Err(err) => {
            tracing::error!(error = %err, "branding lookup failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::single(
                    "common.internal",
                    "An internal error occurred",
                )),
            )
                .into_response()
        }
    }
}

/// Capture a lead from the widget. Persists to `widget_leads` and
/// returns 201 with the same `session_id` echoed back so the widget can
/// thread it into the next step (estimate, consultation, …).
#[utoipa::path(
    post,
    path = "/v1/widget/leads",
    tag = "widget",
    request_body = LeadCaptureRequest,
    security(("widget_api_key" = [])),
    responses(
        (status = 201, description = "Lead captured", body = LeadCaptureResponse),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 401, description = "API key invalid", body = ApiError),
    ),
)]
pub async fn create_lead(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
    Json(body): Json<LeadCaptureRequest>,
) -> Response {
    let session_id = match parse_session_id(&body.session_id) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = validate_contact(&body.name, &body.email, &body.phone) {
        return resp;
    }

    let new = NewLead {
        session_id,
        name: body.name,
        email: body.email,
        phone: body.phone,
    };

    match state.widget_leads.insert_lead(&ctx, &new).await {
        Ok(_lead_id) => (
            StatusCode::CREATED,
            Json(LeadCaptureResponse {
                message: "Lead captured successfully".into(),
                session_id: body.session_id,
            }),
        )
            .into_response(),
        Err(err) => {
            tracing::error!(error = %err, "lead insert failed");
            internal_error()
        }
    }
}

/// Parse and validate a session id, returning a 400 envelope on failure
/// so the widget never silently writes against a malformed key.
#[allow(clippy::result_large_err)]
fn parse_session_id(raw: &str) -> Result<SessionId, Response> {
    Uuid::parse_str(raw).map(SessionId).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::single(
                ErrorCode::ValidationFailed.as_str(),
                "session_id must be a UUID",
            )),
        )
            .into_response()
    })
}

/// Lightweight contact-field validation. Mirrors the FastAPI Pydantic
/// constraints (`min_length=1, max_length=200` for name; `5..=30` for
/// phone) without pulling in an `email` validator — clean strings only.
#[allow(clippy::result_large_err)]
fn validate_contact(name: &str, email: &str, phone: &str) -> Result<(), Response> {
    if name.trim().is_empty() || name.len() > 200 {
        return Err(validation_error("name must be 1–200 chars"));
    }
    if email.trim().is_empty() || !email.contains('@') {
        return Err(validation_error("email must contain '@'"));
    }
    if phone.trim().is_empty() || phone.len() < 5 || phone.len() > 30 {
        return Err(validation_error("phone must be 5–30 chars"));
    }
    Ok(())
}

fn validation_error(msg: &'static str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiError::single(ErrorCode::ValidationFailed.as_str(), msg)),
    )
        .into_response()
}

fn internal_error() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiError::single(
            "common.internal",
            "An internal error occurred",
        )),
    )
        .into_response()
}

/// Submit a consultation request. Persists the lead with
/// `lead_type=consultation` and a free-text `message`, then fires a
/// notification email to the tenant's stored contact address. Email is
/// best-effort: a failure is logged but the response is still 201,
/// matching FastAPI's "lead is the source of truth" semantics.
#[utoipa::path(
    post,
    path = "/v1/widget/consultations",
    tag = "widget",
    request_body = ConsultationRequest,
    security(("widget_api_key" = [])),
    responses(
        (status = 201, description = "Consultation captured", body = ConsultationCapturedResponse),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 401, description = "API key invalid", body = ApiError),
    ),
)]
pub async fn create_consultation(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
    Json(body): Json<ConsultationRequest>,
) -> Response {
    if let Err(resp) = validate_contact(&body.name, &body.email, &body.phone) {
        return resp;
    }
    if let Err(resp) = validate_consultation_message(&body.message) {
        return resp;
    }

    // Resolve the contractor email up front so we can include it in the
    // outbound message. A missing/inactive tenant here would also have
    // failed the widget-key middleware, so a NotFound is genuinely
    // surprising — surface as 500.
    let tenant = match state.tenants.get(&ctx).await {
        Ok(t) => t,
        Err(err) => {
            tracing::error!(error = %err, "consultation: tenant lookup failed");
            return internal_error();
        }
    };

    let new = NewConsultation {
        session_id: body.session_id,
        name: body.name.clone(),
        email: body.email.clone(),
        phone: body.phone.clone(),
        message: body.message.clone(),
    };

    let lead_id = match state.widget_leads.insert_consultation(&ctx, &new).await {
        Ok(id) => id,
        Err(err) => {
            tracing::error!(error = %err, "consultation insert failed");
            return internal_error();
        }
    };

    // Fire-and-forget email — log on failure, do not surface to the
    // caller. Lead is already saved.
    let subject = "New consultation request from your widget";
    let mail_body = format!(
        "New consultation request from your estimation widget:\n\n\
         Name: {name}\n\
         Email: {email}\n\
         Phone: {phone}\n\n\
         Message:\n{message}\n",
        name = body.name,
        email = body.email,
        phone = body.phone,
        message = body.message,
    );
    if let Err(err) = state
        .email
        .send_text(EmailMessage {
            from: state.email_from.as_ref(),
            to: tenant.email.as_str(),
            subject,
            body: &mail_body,
        })
        .await
    {
        tracing::error!(error = %err, lead_id = %lead_id, "consultation email failed");
    }

    (
        StatusCode::CREATED,
        Json(ConsultationCapturedResponse {
            lead_id,
            message: "Consultation request captured successfully".into(),
        }),
    )
        .into_response()
}

#[allow(clippy::result_large_err)]
fn validate_consultation_message(msg: &str) -> Result<(), Response> {
    if msg.trim().is_empty() {
        return Err(validation_error("message must not be empty"));
    }
    if msg.len() > 2000 {
        return Err(validation_error("message must be ≤ 2000 chars"));
    }
    Ok(())
}

/// Record a widget analytics event. Fire-and-forget from the widget.
#[utoipa::path(
    post,
    path = "/v1/widget/events",
    tag = "widget",
    request_body = AnalyticsEventRequest,
    security(("widget_api_key" = [])),
    responses(
        (status = 204, description = "Event recorded"),
        (status = 400, description = "Unknown event type", body = ApiError),
        (status = 401, description = "API key invalid", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn record_event() -> impl IntoResponse {
    not_implemented("POST /v1/widget/events")
}

/// Read daily analytics buckets for the authenticated tenant.
#[utoipa::path(
    get,
    path = "/v1/widget/events",
    tag = "widget",
    params(
        ("days" = Option<u32>, Query, description = "Days of history to return (default 30, max 365)")
    ),
    security(("supabase_jwt" = [])),
    responses(
        (status = 200, description = "Daily buckets", body = AnalyticsSummary),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 429, description = "Rate limit exceeded", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn list_events() -> impl IntoResponse {
    not_implemented("GET /v1/widget/events")
}

/// Assemble the widget router.
///
/// Layering (top-down execution order):
/// * `GET /v1/widget/branding/{prefix}` is **public**. It sits behind
///   the per-IP rate limiter only — no auth — so the widget can bootstrap
///   its theme before any user interaction.
/// * `/v1/widget/leads`, `/v1/widget/consultations`, `POST /v1/widget/events`
///   require a widget API key. Added in 2D.2–2D.4.
/// * `GET /v1/widget/events` requires a Supabase JWT (dashboard read).
///   Added in 2D.4.
pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let public = Router::new()
        .route("/v1/widget/branding/{api_key_prefix}", get(get_branding))
        .route_layer(from_fn_with_state(
            state.branding_rate_limiter.clone(),
            rate_limit,
        ));

    // Widget API-key authenticated routes. Stubs (consultations,
    // record_event) get the same auth layer now so the contract is
    // already locked; their handlers replace `not_implemented` in
    // 2D.3 / 2D.4.
    let widget_authed = Router::new()
        .route("/v1/widget/leads", post(create_lead))
        .route("/v1/widget/consultations", post(create_consultation))
        .route("/v1/widget/events", post(record_event))
        .route_layer(from_fn_with_state(state.auth.clone(), widget_api_key));

    // GET /v1/widget/events lands in 2D.4 with Supabase JWT auth — kept
    // off the widget-key router so the layering stays correct.
    let dashboard = Router::new().route("/v1/widget/events", get(list_events));

    public.merge(widget_authed).merge(dashboard)
}
