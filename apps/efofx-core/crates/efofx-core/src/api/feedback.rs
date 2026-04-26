//! Feedback endpoints (Phase 2E).
//!
//! Mix of authenticated JSON endpoints (submit, summary, email-requests)
//! and token-authenticated HTML form endpoints for customer-facing flows.

use std::sync::Arc;

use axum::extract::{Extension, Form, Path, State};
use axum::http::{header, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use efofx_auth::middleware::{either_auth, supabase_jwt};
use efofx_domain::{
    BrandingConfig, EstimationSessionId, FeedbackDocument, FeedbackSubmission, FeedbackSummary,
};
use efofx_email::EmailMessage;
use efofx_openapi::{ApiError, ErrorCode};
use efofx_storage::{
    EstimateSnapshotDoc, MagicLinkDoc, NewFeedback, NewFeedbackWithSnapshot, NewMagicLink,
    TenantContext, TokenState,
};

use crate::services::EstimationServiceError;
use crate::AppState;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateFeedbackRequest {
    pub estimation_session_id: String,
    /// Free-text category, e.g. `"accuracy"` or `"timeline"`.
    pub feedback_type: String,
    pub rating: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_cost: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_timeline_weeks: Option<u32>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreateFeedbackResponse {
    pub feedback_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct FeedbackEmailRequest {
    pub estimation_session_id: String,
    pub customer_email: String,
    #[serde(default = "default_project_name")]
    pub project_name: String,
}

fn default_project_name() -> String {
    "Your Project".into()
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FeedbackEmailResponse {
    pub message: String,
    /// SHA-256 hash of the minted magic-link token. The raw token is sent
    /// only in the email; never returned to the requester.
    pub token_hash: String,
}

/// Submit feedback on an existing estimation session. Accepts either a
/// dashboard-issued Supabase JWT or a widget API key — both flows produce
/// a [`TenantContext`] before the handler runs.
#[utoipa::path(
    post,
    path = "/v1/feedback",
    tag = "feedback",
    request_body = CreateFeedbackRequest,
    security(("supabase_jwt" = []), ("widget_api_key" = [])),
    responses(
        (status = 201, description = "Feedback captured", body = CreateFeedbackResponse),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
    ),
)]
pub async fn create_feedback(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
    Json(body): Json<CreateFeedbackRequest>,
) -> Response {
    if let Err(resp) = validate_create_feedback(&body) {
        return resp;
    }

    let new = NewFeedback {
        estimation_session_id: body.estimation_session_id,
        feedback_type: body.feedback_type,
        rating: body.rating,
        comment: body.comment,
        actual_cost: body.actual_cost,
        actual_timeline_weeks: body.actual_timeline_weeks,
        // The 2E.2 wire DTO doesn't carry these; magic-link / future
        // widget flows fill them in.
        actual_team_size: None,
        cost_accuracy: None,
        timeline_accuracy: None,
        reference_class_accuracy: None,
    };

    match state.feedback.insert_basic(&ctx, &new).await {
        Ok(feedback_id) => (
            StatusCode::CREATED,
            Json(CreateFeedbackResponse {
                feedback_id,
                message: "Feedback recorded successfully".into(),
            }),
        )
            .into_response(),
        Err(err) => {
            tracing::error!(error = %err, "feedback insert failed");
            internal_error()
        }
    }
}

/// Aggregate feedback summary for the authenticated tenant.
#[utoipa::path(
    get,
    path = "/v1/feedback/summary",
    tag = "feedback",
    security(("supabase_jwt" = [])),
    responses(
        (status = 200, description = "Aggregate stats", body = FeedbackSummary),
        (status = 401, description = "Authentication required", body = ApiError),
    ),
)]
pub async fn feedback_summary(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
) -> Response {
    match state.feedback.summary(&ctx).await {
        Ok(stats) => Json(FeedbackSummary {
            total_feedback: stats.total_feedback,
            average_rating: stats.average_rating,
            cost_accuracy_avg: stats.cost_accuracy_avg,
            timeline_accuracy_avg: stats.timeline_accuracy_avg,
            reference_class_accuracy_avg: stats.reference_class_accuracy_avg,
            feedback_by_type: stats.feedback_by_type,
        })
        .into_response(),
        Err(err) => {
            tracing::error!(error = %err, "feedback summary failed");
            internal_error()
        }
    }
}

/// Mint a feedback magic-link and email it to the customer. The raw token
/// is sent only in the email. Token TTL: 72 hours.
///
/// Email send is best-effort — if Resend errors or no sender is wired
/// (NoopSender), the magic link is still persisted and a `202` is
/// returned. This mirrors the consultation flow: the system of record
/// is the database, not the mailer.
#[utoipa::path(
    post,
    path = "/v1/feedback/email-requests",
    tag = "feedback",
    request_body = FeedbackEmailRequest,
    security(("supabase_jwt" = [])),
    responses(
        (status = 202, description = "Email queued", body = FeedbackEmailResponse),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 404, description = "Estimation session not found", body = ApiError),
    ),
)]
pub async fn request_email(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
    Json(body): Json<FeedbackEmailRequest>,
) -> Response {
    if let Err(resp) = validate_email_request(&body) {
        return resp;
    }

    // Confirm the estimation session belongs to this tenant before
    // minting a token. The service surfaces a 404 envelope itself for
    // NotFound; other errors get surfaced as their respective envelopes.
    let session_id = EstimationSessionId(body.estimation_session_id.clone());
    if let Err(err) = state.estimation.get_session(&ctx, &session_id).await {
        if matches!(err, EstimationServiceError::EstimationSessionNotFound) {
            return err.into_response();
        }
        tracing::error!(error = %err, "feedback email: session lookup failed");
        return err.into_response();
    }

    let new = NewMagicLink {
        tenant_id: ctx.tenant_id().to_string(),
        estimation_session_id: body.estimation_session_id.clone(),
        customer_email: body.customer_email.clone(),
        project_name: body.project_name.clone(),
    };
    let minted = match state.magic_link.create(new).await {
        Ok(m) => m,
        Err(err) => {
            tracing::error!(error = %err, "magic link mint failed");
            return internal_error();
        }
    };

    let url = format!(
        "{base}/v1/feedback/forms/{token}",
        base = state.app_base_url,
        token = minted.raw_token,
    );
    let subject = format!("How did {} go?", body.project_name);
    let mail_body = format!(
        "Hi,\n\n\
         We'd love your feedback on the {project} estimate. Tell us how the \
         project actually wrapped up — it takes a couple of minutes and helps \
         us calibrate future estimates.\n\n\
         {url}\n\n\
         This link expires in 72 hours.\n",
        project = body.project_name,
        url = url,
    );
    if let Err(err) = state
        .email
        .send_text(EmailMessage {
            from: state.email_from.as_ref(),
            to: body.customer_email.as_str(),
            subject: &subject,
            body: &mail_body,
        })
        .await
    {
        tracing::error!(
            error = %err,
            token_hash = %minted.token_hash,
            "feedback magic-link email failed"
        );
    }

    (
        StatusCode::ACCEPTED,
        Json(FeedbackEmailResponse {
            message: "Feedback request email queued".into(),
            token_hash: minted.token_hash,
        }),
    )
        .into_response()
}

/// Render the feedback form. Idempotent — scanner-safe. Sets `opened_at`
/// on first visit; does not consume the token. Always returns HTTP 200
/// with `text/html`: token state is encoded by the body, not the
/// status, so an email-scanner preview never tells the user something
/// is wrong before the customer clicks through.
#[utoipa::path(
    get,
    path = "/v1/feedback/forms/{token}",
    tag = "feedback",
    params(("token" = String, Path, description = "Raw magic-link token from email")),
    responses(
        (status = 200, description = "HTML page (form / expired / thank-you)", content_type = "text/html"),
    ),
)]
pub async fn render_form(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Response {
    let resolved = match state.magic_link.resolve(&token).await {
        Ok(state) => state,
        Err(err) => {
            tracing::error!(error = %err, "magic link resolve failed");
            return html_response(&render_expired(&BrandingConfig::default()));
        }
    };

    match resolved {
        TokenState::NotFound | TokenState::Expired(_) => {
            let branding = match &resolved {
                TokenState::Expired(doc) => branding_for_tenant(&state, &doc.tenant_id).await,
                _ => BrandingConfig::default(),
            };
            html_response(&render_expired(&branding))
        }
        TokenState::Used(doc) => {
            let branding = branding_for_tenant(&state, &doc.tenant_id).await;
            html_response(&render_thank_you(&branding))
        }
        TokenState::Valid(doc) => {
            // Best-effort opened-at stamp; failure here is logged but
            // never blocks rendering the form. Email scanners hitting
            // the URL stamp opened_at too — that's fine, FastAPI does
            // the same and it's not security-relevant.
            if let Err(err) = state.magic_link.mark_opened(&token).await {
                tracing::warn!(error = %err, "mark_opened failed");
            }
            let branding = branding_for_tenant(&state, &doc.tenant_id).await;
            html_response(&render_form_page(&branding, &doc, &token))
        }
    }
}

/// Submit the feedback form. Consumes the token atomically — resubmits
/// return the thank-you page, never a double-write.
#[utoipa::path(
    post,
    path = "/v1/feedback/forms/{token}",
    tag = "feedback",
    params(("token" = String, Path)),
    request_body(content = FeedbackSubmission, content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = 200, description = "HTML thank-you page", content_type = "text/html"),
        (status = 400, description = "Validation failed", body = ApiError),
    ),
)]
pub async fn submit_form(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    Form(submission): Form<FeedbackSubmission>,
) -> Response {
    if let Err(resp) = validate_submission(&submission) {
        return resp;
    }

    let resolved = match state.magic_link.resolve(&token).await {
        Ok(s) => s,
        Err(err) => {
            tracing::error!(error = %err, "magic link resolve failed");
            return html_response(&render_expired(&BrandingConfig::default()));
        }
    };

    let doc = match resolved {
        TokenState::Valid(doc) => doc,
        TokenState::Used(doc) => {
            // Idempotent resubmit — render the thank-you page so the
            // customer can't tell whether their first POST was the
            // winner. No double-write.
            let branding = branding_for_tenant(&state, &doc.tenant_id).await;
            return html_response(&render_thank_you(&branding));
        }
        TokenState::Expired(doc) => {
            let branding = branding_for_tenant(&state, &doc.tenant_id).await;
            return html_response(&render_expired(&branding));
        }
        TokenState::NotFound => {
            return html_response(&render_expired(&BrandingConfig::default()));
        }
    };

    match state.magic_link.consume(&token).await {
        Ok(true) => {}
        Ok(false) => {
            // Lost the race — another POST claimed the token first.
            // Render thank-you, never double-insert.
            let branding = branding_for_tenant(&state, &doc.tenant_id).await;
            return html_response(&render_thank_you(&branding));
        }
        Err(err) => {
            tracing::error!(error = %err, "consume token failed");
            let branding = branding_for_tenant(&state, &doc.tenant_id).await;
            return html_response(&render_expired(&branding));
        }
    }

    let new = NewFeedbackWithSnapshot {
        estimation_session_id: doc.estimation_session_id.clone(),
        reference_class_id: None,
        actual_cost: submission.actual_cost,
        actual_timeline_weeks: submission.actual_timeline_weeks,
        rating: submission.rating,
        discrepancy_reason_primary: submission.discrepancy_reason_primary,
        discrepancy_reason_secondary: submission.discrepancy_reason_secondary,
        comment: submission.comment,
        // EstimationOutput is not persisted alongside the session in the
        // current schema (see api/estimation.rs:71). Until that gap
        // closes, the snapshot fields go in zero — calibration in 2E.5
        // will simply find no useful variance for tokens minted from
        // sessions without a stored result. The wire shape is preserved
        // so a future migration is additive.
        estimate_snapshot: EstimateSnapshotDoc {
            total_cost_p50: 0.0,
            total_cost_p80: 0.0,
            timeline_weeks_p50: 0,
            timeline_weeks_p80: 0,
            cost_breakdown: Vec::new(),
            assumptions: Vec::new(),
            confidence_score: 0.0,
        },
    };

    if let Err(err) = state
        .feedback
        .insert_with_snapshot(&doc.tenant_id, new)
        .await
    {
        tracing::error!(error = %err, "feedback insert_with_snapshot failed");
        let branding = branding_for_tenant(&state, &doc.tenant_id).await;
        return html_response(&render_expired(&branding));
    }

    let branding = branding_for_tenant(&state, &doc.tenant_id).await;
    html_response(&render_thank_you(&branding))
}

async fn branding_for_tenant(state: &AppState, tenant_id: &str) -> BrandingConfig {
    match state.tenants.fetch_branding_by_tenant_id(tenant_id).await {
        Ok(Some(b)) => b,
        Ok(None) => BrandingConfig::default(),
        Err(err) => {
            tracing::warn!(error = %err, tenant_id, "branding fetch failed; using defaults");
            BrandingConfig::default()
        }
    }
}

fn html_response(body: &str) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        body.to_string(),
    )
        .into_response()
}

fn render_form_page(branding: &BrandingConfig, doc: &MagicLinkDoc, token: &str) -> String {
    FORM_HTML
        .replace("{{primary_color}}", &esc(&branding.primary_color))
        .replace("{{company_name}}", &esc(&render_company_name(branding)))
        .replace("{{project_name}}", &esc(&doc.project_name))
        .replace("{{token}}", &esc(token))
}

fn render_thank_you(branding: &BrandingConfig) -> String {
    THANK_YOU_HTML
        .replace("{{primary_color}}", &esc(&branding.primary_color))
        .replace("{{company_name}}", &esc(&render_company_name(branding)))
}

fn render_expired(branding: &BrandingConfig) -> String {
    EXPIRED_HTML
        .replace("{{primary_color}}", &esc(&branding.primary_color))
        .replace("{{company_name}}", &esc(&render_company_name(branding)))
}

fn render_company_name(branding: &BrandingConfig) -> String {
    if branding.company_name.is_empty() {
        "Your contractor".into()
    } else {
        branding.company_name.clone()
    }
}

/// Minimal HTML escape — sufficient for the four substitution slots
/// (color hex, company name, project name, token). Token is always a
/// 43-char base64url string so it is escape-clean, but we run it
/// through the same path for safety.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

// FeedbackDocument referenced so the OpenAPI component graph keeps it.
#[allow(dead_code)]
fn _keep_feedback_doc_in_openapi(_: &FeedbackDocument) {}

#[allow(clippy::result_large_err)]
fn validate_create_feedback(body: &CreateFeedbackRequest) -> Result<(), Response> {
    if body.estimation_session_id.trim().is_empty() {
        return Err(validation_error("estimation_session_id must not be empty"));
    }
    if body.feedback_type.trim().is_empty() {
        return Err(validation_error("feedback_type must not be empty"));
    }
    if !(1..=5).contains(&body.rating) {
        return Err(validation_error("rating must be between 1 and 5"));
    }
    Ok(())
}

#[allow(clippy::result_large_err)]
fn validate_email_request(body: &FeedbackEmailRequest) -> Result<(), Response> {
    if body.estimation_session_id.trim().is_empty() {
        return Err(validation_error("estimation_session_id must not be empty"));
    }
    if body.customer_email.trim().is_empty() || !body.customer_email.contains('@') {
        return Err(validation_error("customer_email must contain '@'"));
    }
    if body.project_name.trim().is_empty() || body.project_name.len() > 200 {
        return Err(validation_error("project_name must be 1–200 chars"));
    }
    Ok(())
}

#[allow(clippy::result_large_err)]
fn validate_submission(body: &FeedbackSubmission) -> Result<(), Response> {
    if !(1..=5).contains(&body.rating) {
        return Err(validation_error("rating must be between 1 and 5"));
    }
    if !(body.actual_cost.is_finite() && body.actual_cost > 0.0) {
        return Err(validation_error("actual_cost must be greater than 0"));
    }
    if body.actual_timeline_weeks == 0 {
        return Err(validation_error(
            "actual_timeline_weeks must be greater than 0",
        ));
    }
    if let Some(c) = &body.comment {
        if c.len() > 2000 {
            return Err(validation_error("comment must be ≤ 2000 chars"));
        }
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

/// Assemble the feedback router.
///
/// Layering, outer to inner:
/// * `POST /v1/feedback` accepts either Supabase JWT or widget API key —
///   contractor dashboards and embedded widgets both call this.
/// * `GET /v1/feedback/summary` is dashboard-only; supabase_jwt enforced.
/// * `POST /v1/feedback/email-requests` is dashboard-only (2E.3 stub).
/// * `GET|POST /v1/feedback/forms/{token}` is public, token-gated. No
///   auth layer applied here — the token *is* the auth.
pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let either_authed = Router::new()
        .route("/v1/feedback", post(create_feedback))
        .route_layer(from_fn_with_state(state.auth.clone(), either_auth));

    let dashboard = Router::new()
        .route("/v1/feedback/summary", get(feedback_summary))
        .route("/v1/feedback/email-requests", post(request_email))
        .route_layer(from_fn_with_state(state.auth.clone(), supabase_jwt));

    let public = Router::new().route(
        "/v1/feedback/forms/:token",
        get(render_form).post(submit_form),
    );

    either_authed.merge(dashboard).merge(public)
}

// ----------------------------------------------------------------------
// Inline HTML pages
//
// Three short pages embedded as `&'static str`. We avoid an HTML
// template engine (askama / minijinja) on purpose — adding a fourth
// page is the right time to swap in a real engine. Total weight is
// well under 4 KB each. Substitution is naive `.replace("{{...}}", ...)`
// because we control every placeholder source and run untrusted strings
// through `esc()` before substitution.
// ----------------------------------------------------------------------

const FORM_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Project feedback</title>
<style>
  body { font-family: system-ui, -apple-system, Segoe UI, Roboto, sans-serif; background: #f8fafc; margin: 0; padding: 2rem; color: #1f2937; }
  .card { max-width: 540px; margin: 0 auto; background: #fff; border-radius: 12px; padding: 2rem; box-shadow: 0 1px 3px rgba(0,0,0,.08); }
  h1 { color: {{primary_color}}; margin-top: 0; }
  label { display: block; margin: 1rem 0 .25rem; font-weight: 600; }
  input, select, textarea { width: 100%; padding: .6rem; border: 1px solid #d1d5db; border-radius: 6px; font: inherit; box-sizing: border-box; }
  textarea { min-height: 6rem; resize: vertical; }
  button { background: {{primary_color}}; color: #fff; border: 0; padding: .75rem 1.5rem; border-radius: 6px; font-size: 1rem; font-weight: 600; cursor: pointer; margin-top: 1.5rem; }
  .meta { color: #6b7280; font-size: .9rem; }
</style>
</head>
<body>
<div class="card">
  <h1>How did {{project_name}} go?</h1>
  <p class="meta">{{company_name}} would love your feedback. It takes about a minute.</p>
  <form method="post" action="/v1/feedback/forms/{{token}}">
    <label>Overall rating (1–5)</label>
    <input type="number" name="rating" min="1" max="5" required>

    <label>Actual cost (USD)</label>
    <input type="number" name="actual_cost" min="0" step="0.01" required>

    <label>Actual timeline (weeks)</label>
    <input type="number" name="actual_timeline_weeks" min="1" required>

    <label>Primary reason the estimate was off</label>
    <select name="discrepancy_reason_primary" required>
      <option value="estimate_was_accurate">Estimate was accurate</option>
      <option value="scope_changed">Scope changed</option>
      <option value="unforeseen_issues">Unforeseen issues</option>
      <option value="timeline_pressure">Timeline pressure</option>
      <option value="vendor_material_costs">Vendor or material costs</option>
      <option value="client_changes">Client changes</option>
    </select>

    <label>Anything else? (optional)</label>
    <textarea name="comment" maxlength="2000"></textarea>

    <button type="submit">Submit feedback</button>
  </form>
</div>
</body>
</html>
"#;

const THANK_YOU_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Thank you</title>
<style>
  body { font-family: system-ui, -apple-system, Segoe UI, Roboto, sans-serif; background: #f8fafc; margin: 0; padding: 2rem; color: #1f2937; }
  .card { max-width: 540px; margin: 0 auto; background: #fff; border-radius: 12px; padding: 2rem; box-shadow: 0 1px 3px rgba(0,0,0,.08); text-align: center; }
  h1 { color: {{primary_color}}; margin-top: 0; }
</style>
</head>
<body>
<div class="card">
  <h1>Thank you</h1>
  <p>Your feedback has been recorded. {{company_name}} appreciates the help calibrating future estimates.</p>
</div>
</body>
</html>
"#;

const EXPIRED_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Link expired</title>
<style>
  body { font-family: system-ui, -apple-system, Segoe UI, Roboto, sans-serif; background: #f8fafc; margin: 0; padding: 2rem; color: #1f2937; }
  .card { max-width: 540px; margin: 0 auto; background: #fff; border-radius: 12px; padding: 2rem; box-shadow: 0 1px 3px rgba(0,0,0,.08); text-align: center; }
  h1 { color: {{primary_color}}; margin-top: 0; }
</style>
</head>
<body>
<div class="card">
  <h1>This link has expired</h1>
  <p>The feedback link you followed is no longer valid. Reach out to {{company_name}} if you'd still like to share how the project went.</p>
</div>
</body>
</html>
"#;
