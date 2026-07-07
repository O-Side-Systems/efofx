//! Chat session endpoints (Phase 2B).

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use efofx_auth::middleware::either_auth;
use efofx_domain::{ChatMessage, ChatSession, ScopingContext, SessionId};
use efofx_llm::{LlmError, StreamEvent};
use efofx_openapi::{ApiError, ErrorCode};
use efofx_storage::TenantContext;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::services::{routing, ChatService, ChatServiceError, EstimationServiceError};
use crate::AppState;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateChatSessionRequest {
    /// Optional first message. If supplied, the session is created with
    /// this message already appended and readiness evaluation runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AppendMessageRequest {
    /// User message, 1–2000 chars.
    pub message: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AppendMessageResponse {
    pub assistant_message: ChatMessage,
    pub is_ready: bool,
    pub scoping_context: ScopingContext,
}

/// Decrypt the caller's BYOK key. Returns 402 if the tenant has no key on
/// file — LLM-powered flows are gated behind BYOK in v1.
#[allow(clippy::result_large_err)]
async fn byok_plaintext(state: &AppState, ctx: &TenantContext) -> Result<String, Response> {
    match state.byok.decrypt(ctx).await {
        Ok(Some(k)) => Ok(k),
        Ok(None) => Err((
            StatusCode::PAYMENT_REQUIRED,
            Json(ApiError::single(
                ErrorCode::ChatLlmInvalidKey.as_str(),
                "No OpenAI API key on file. Add one in Settings.",
            )),
        )
            .into_response()),
        Err(e) => Err(e.into_response()),
    }
}

#[allow(clippy::result_large_err)]
/// Split the `:generate-estimate` custom-method suffix off a captured
/// path segment. Returns the bare session-id portion, or `None` when the
/// suffix is absent.
fn strip_generate_estimate_suffix(segment: &str) -> Option<&str> {
    segment.strip_suffix(":generate-estimate")
}

fn parse_session_id(raw: &str) -> Result<SessionId, Response> {
    match Uuid::parse_str(raw) {
        Ok(u) => Ok(SessionId(u)),
        Err(_) => Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError::single(
                ErrorCode::ValidationFailed.as_str(),
                "session_id is not a valid UUID",
            )),
        )
            .into_response()),
    }
}

/// Create a new chat session for the authenticated tenant. If
/// `initial_message` is supplied, the append flow runs in the same call so
/// the client gets back a session with both messages + a ready assessment
/// in one round-trip.
#[utoipa::path(
    post,
    path = "/v1/chat/sessions",
    tag = "chat",
    request_body = CreateChatSessionRequest,
    security(("supabase_jwt" = []), ("widget_api_key" = [])),
    responses(
        (status = 201, description = "Session created", body = ChatSession),
        (status = 400, description = "Message validation failed", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 402, description = "BYOK key missing or invalid", body = ApiError),
        (status = 429, description = "Rate limit or budget exceeded", body = ApiError),
    ),
)]
pub async fn create_session(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
    Json(body): Json<CreateChatSessionRequest>,
) -> Response {
    let key = match body.initial_message {
        Some(_) => match byok_plaintext(&state, &ctx).await {
            Ok(k) => k,
            Err(resp) => return resp,
        },
        // No initial message means no LLM call, so we can create the empty
        // session without a BYOK key on file. Subsequent appends will still
        // require one.
        None => String::new(),
    };

    match state
        .chat
        .create_session(&ctx, &key, body.initial_message)
        .await
    {
        Ok(session) => (StatusCode::CREATED, Json(session)).into_response(),
        Err(e) => chat_error_response(e),
    }
}

/// Fetch a chat session with its full message history.
#[utoipa::path(
    get,
    path = "/v1/chat/sessions/{session_id}",
    tag = "chat",
    params(
        ("session_id" = String, Path, description = "Chat session identifier")
    ),
    security(("supabase_jwt" = []), ("widget_api_key" = [])),
    responses(
        (status = 200, description = "Chat session", body = ChatSession),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 404, description = "Session not found", body = ApiError),
    ),
)]
pub async fn get_session_handler(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
    Path(session_id): Path<String>,
) -> Response {
    let session_id = match parse_session_id(&session_id) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match state.chat.get_session(&ctx, session_id).await {
        Ok(session) => Json(session).into_response(),
        Err(e) => chat_error_response(e),
    }
}

/// Append a user message to an active session. Runs readiness evaluation
/// and may transition the session from `active` to `ready`.
#[utoipa::path(
    post,
    path = "/v1/chat/sessions/{session_id}/messages",
    tag = "chat",
    params(
        ("session_id" = String, Path, description = "Chat session identifier")
    ),
    request_body = AppendMessageRequest,
    security(("supabase_jwt" = []), ("widget_api_key" = [])),
    responses(
        (status = 200, description = "Assistant reply + updated scoping", body = AppendMessageResponse),
        (status = 400, description = "Message validation failed", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 402, description = "BYOK key missing or invalid", body = ApiError),
        (status = 404, description = "Session not found", body = ApiError),
        (status = 410, description = "Session expired", body = ApiError),
        (status = 429, description = "Message or token limit hit", body = ApiError),
    ),
)]
pub async fn append_message(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
    Path(session_id): Path<String>,
    Json(body): Json<AppendMessageRequest>,
) -> Response {
    let session_id = match parse_session_id(&session_id) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let key = match byok_plaintext(&state, &ctx).await {
        Ok(k) => k,
        Err(resp) => return resp,
    };
    match state
        .chat
        .append_message(&ctx, &key, session_id, body.message)
        .await
    {
        Ok(outcome) => Json(AppendMessageResponse {
            assistant_message: outcome.assistant_message,
            is_ready: outcome.session.is_ready,
            scoping_context: outcome.session.scoping_context,
        })
        .into_response(),
        Err(e) => chat_error_response(e),
    }
}

fn chat_error_response(err: ChatServiceError) -> Response {
    // ChatServiceError already knows how to render itself. Kept as a named
    // helper so the handlers read the same as the rest of the codebase.
    err.into_response()
}

/// Generate a structured estimate from a ready chat session. Responds
/// with `text/event-stream`: emits `thinking`, then `estimate`, then
/// streams `narrative` tokens, then `done`. On error, emits `error`
/// and closes. Wired in Phase 2C.
#[utoipa::path(
    post,
    path = "/v1/chat/sessions/{session_id}:generate-estimate",
    tag = "estimation",
    params(
        ("session_id" = String, Path, description = "Chat session identifier")
    ),
    security(("supabase_jwt" = []), ("widget_api_key" = [])),
    responses(
        (status = 200, description = "SSE stream", content_type = "text/event-stream"),
        (status = 400, description = "Invalid session id", body = ApiError),
        (status = 401, description = "Authentication required", body = ApiError),
    ),
)]
pub async fn generate_estimate(
    State(state): State<Arc<AppState>>,
    Extension(ctx): Extension<TenantContext>,
    Path(raw_path): Path<String>,
) -> Response {
    // The route captures `<session_id>:generate-estimate` as one segment
    // (see `routes`); a POST without the custom-method suffix is not a
    // defined operation.
    let Some(raw_id) = strip_generate_estimate_suffix(&raw_path) else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiError::single(
                "common.not_found",
                "POST /v1/chat/sessions/{session_id} is not a supported operation; \
                 did you mean :generate-estimate?",
            )),
        )
            .into_response();
    };
    // Pre-stream validation: bad path params return plain JSON, not SSE.
    let session_id = match parse_session_id(raw_id) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let state = Arc::clone(&state);
    let ctx = ctx.clone();

    let stream = async_stream::stream! {
        // 1. thinking — mirrors FastAPI NARR-04.
        yield Ok::<Event, Infallible>(Event::default().event("thinking").data("{}"));

        // 2. BYOK decrypt. No key on file = typed SSE error, not HTTP 402,
        //    because the stream is already committed.
        let api_key = match state.byok.decrypt(&ctx).await {
            Ok(Some(k)) => k,
            Ok(None) => {
                yield Ok(sse_error(
                    "invalid_key",
                    "No OpenAI API key on file. Add one in Settings.",
                    402,
                ));
                return;
            }
            Err(err) => {
                tracing::error!(error = %err, "byok decrypt failed in SSE stream");
                yield Ok(sse_error(
                    "unknown",
                    "Failed to decrypt stored OpenAI key.",
                    500,
                ));
                return;
            }
        };

        // 3. Structured estimate: verifies ready, classifies, loads refs,
        //    calls the structured-output LLM, and persists the session.
        let outcome = match state
            .estimation
            .generate_from_chat(&ctx, &api_key, session_id)
            .await
        {
            Ok(o) => o,
            Err(err) => {
                tracing::warn!(
                    chat_session = %session_id,
                    error = %err,
                    "estimate generation failed before narrative stream",
                );
                yield Ok(map_estimation_error(&err));
                return;
            }
        };

        // 4. estimate event: structured JSON payload clients render as soon
        //    as it lands (before narrative tokens start).
        match Event::default().event("estimate").json_data(&outcome.output) {
            Ok(ev) => yield Ok(ev),
            Err(err) => {
                tracing::error!(error = %err, "estimate event json_data failed");
                yield Ok(sse_error(
                    "unknown",
                    "Failed to serialize estimate.",
                    500,
                ));
                return;
            }
        }

        // 5. Render the narrative prompt against the just-persisted estimate.
        let (narr_req, _prompt_version) = match state
            .estimation
            .build_narrative_request(&outcome.session.description, &outcome.output)
        {
            Ok(pair) => pair,
            Err(err) => {
                tracing::error!(error = %err, "narrative prompt render failed");
                yield Ok(sse_error(
                    "unknown",
                    "Failed to render narrative prompt.",
                    500,
                ));
                return;
            }
        };

        // 6. Stream narrative tokens — FastAPI emits a bare `data: <text>`
        //    for each delta, no named event. Newlines are escaped so the
        //    SSE framing survives; `Event::data` already splits on `\n`,
        //    but FastAPI's escape preserves them as literal `\n` characters
        //    in the payload, which is what the widget expects.
        let mut narrative_stream = match state
            .estimation
            .stream_narrative(&api_key, narr_req)
            .await
        {
            Ok(s) => s,
            Err(err) => {
                tracing::warn!(error = %err, "opening narrative stream failed");
                yield Ok(map_llm_error(&err));
                return;
            }
        };

        while let Some(chunk) = narrative_stream.next().await {
            match chunk {
                Ok(StreamEvent::Delta(text)) => {
                    let escaped = text.replace('\n', "\\n");
                    yield Ok(Event::default().data(escaped));
                }
                Ok(StreamEvent::Done { .. }) => break,
                Err(err) => {
                    tracing::warn!(error = %err, "narrative stream token error");
                    yield Ok(map_llm_error(&err));
                    return;
                }
            }
        }

        // 7. Mark chat session completed — best-effort; a failure here does
        //    not roll back the persisted estimate.
        if let Err(err) = state.estimation.mark_chat_completed(&ctx, session_id).await {
            tracing::warn!(
                chat_session = %session_id,
                error = %err,
                "mark_chat_completed failed (stream continues)",
            );
        }

        // 8. done — includes routing_tags for partner integrations
        //    (Phase 2F.2). Empty array when routing is disabled or
        //    unconfigured; the field is always present so partners
        //    don't need conditional parsing.
        let routing_cfg = match state.tenants.fetch_routing_config(&ctx).await {
            Ok(cfg) => cfg,
            Err(err) => {
                tracing::warn!(
                    tenant_id = %ctx.tenant_id(),
                    error = %err,
                    "fetch_routing_config failed; emitting done with empty routing_tags",
                );
                None
            }
        };
        let routing_data = routing::derive(
            &outcome.session,
            &outcome.scoping,
            &outcome.output,
            routing_cfg.as_ref(),
        );
        let done = Event::default()
            .event("done")
            .json_data(json!({
                "session_id": outcome.session.id.as_str(),
                "routing_tags": routing_data.tags,
            }))
            .unwrap_or_else(|_| Event::default().event("done").data("{}"));
        yield Ok(done);
    };

    Sse::new(stream).into_response()
}

/// Classified payload for an SSE `error` event. FastAPI parity shape:
/// `{ error_type, message, status }`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SseErrorKind {
    error_type: &'static str,
    message: &'static str,
    status: u16,
}

impl SseErrorKind {
    fn to_event(&self) -> Event {
        Event::default()
            .event("error")
            .json_data(json!({
                "error_type": self.error_type,
                "message": self.message,
                "status": self.status,
            }))
            .unwrap_or_else(|_| Event::default().event("error").data("{}"))
    }
}

fn sse_error(error_type: &'static str, message: &'static str, status: u16) -> Event {
    SseErrorKind {
        error_type,
        message,
        status,
    }
    .to_event()
}

fn classify_llm_for_sse(err: &LlmError) -> SseErrorKind {
    let (error_type, message, status) = match err {
        LlmError::InvalidKey(_) => (
            "invalid_key",
            "Invalid OpenAI API key. Update your key in Settings.",
            402,
        ),
        LlmError::QuotaExhausted(_) => (
            "quota_exhausted",
            "OpenAI quota exhausted. Recharge your OpenAI account.",
            402,
        ),
        LlmError::Transient(_) => (
            "transient",
            "We're having trouble generating a response. Please try again in a moment.",
            503,
        ),
        LlmError::SchemaParse(_) => ("unknown", "The AI returned an unexpected response.", 502),
        LlmError::Unknown(_) => (
            "unknown",
            "An unexpected error occurred during AI processing.",
            500,
        ),
    };
    SseErrorKind {
        error_type,
        message,
        status,
    }
}

fn classify_estimation_for_sse(err: &EstimationServiceError) -> SseErrorKind {
    let (error_type, message, status) = match err {
        EstimationServiceError::ChatSessionNotFound
        | EstimationServiceError::EstimationSessionNotFound => {
            ("invalid_session", "Chat session not found", 404)
        }
        EstimationServiceError::ChatSessionExpired => {
            ("invalid_session", "Chat session has expired", 410)
        }
        EstimationServiceError::SessionNotReady => (
            "invalid_state",
            "Session is not ready for estimate generation",
            409,
        ),
        EstimationServiceError::MissingByokKey => (
            "invalid_key",
            "No OpenAI API key on file. Add one in Settings.",
            402,
        ),
        EstimationServiceError::Llm(e) => return classify_llm_for_sse(e),
        EstimationServiceError::SchemaParse(_) => (
            "unknown",
            "Estimation response failed schema validation.",
            500,
        ),
        EstimationServiceError::Storage(_) | EstimationServiceError::Prompt(_) => {
            ("unknown", "An unexpected error occurred.", 500)
        }
    };
    SseErrorKind {
        error_type,
        message,
        status,
    }
}

fn map_llm_error(err: &LlmError) -> Event {
    classify_llm_for_sse(err).to_event()
}

fn map_estimation_error(err: &EstimationServiceError) -> Event {
    classify_estimation_for_sse(err).to_event()
}

// Unused silencer so the `ChatService` type import is meaningful in
// rustdoc / IDE-surface even before the 2C handler uses it directly.
#[allow(dead_code)]
fn _assert_chat_service_in_scope(_: &ChatService) {}

/// Assemble the chat router. All endpoints accept either a Supabase JWT or
/// a widget API key — the tenant is the same principal either way.
pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/chat/sessions", post(create_session))
        // Wire paths use `:generate-estimate` as an AIP-136 custom-method
        // suffix. axum 0.7's matchit can't express "param + literal suffix"
        // in one segment, so POST captures the whole segment and
        // `generate_estimate` validates the suffix itself.
        .route(
            "/v1/chat/sessions/:session_id",
            get(get_session_handler).post(generate_estimate),
        )
        .route(
            "/v1/chat/sessions/:session_id/messages",
            post(append_message),
        )
        .route_layer(from_fn_with_state(state.auth.clone(), either_auth))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_estimate_suffix_strips_cleanly() {
        assert_eq!(
            strip_generate_estimate_suffix("0f8fad5b-d9cb-469f-a165-70867728950e:generate-estimate"),
            Some("0f8fad5b-d9cb-469f-a165-70867728950e")
        );
    }

    #[test]
    fn generate_estimate_suffix_missing_is_none() {
        assert_eq!(
            strip_generate_estimate_suffix("0f8fad5b-d9cb-469f-a165-70867728950e"),
            None
        );
        assert_eq!(strip_generate_estimate_suffix(""), None);
    }

    #[test]
    fn llm_invalid_key_maps_to_402() {
        let k = classify_llm_for_sse(&LlmError::InvalidKey("".into()));
        assert_eq!(k.error_type, "invalid_key");
        assert_eq!(k.status, 402);
    }

    #[test]
    fn llm_quota_exhausted_maps_to_402() {
        let k = classify_llm_for_sse(&LlmError::QuotaExhausted("".into()));
        assert_eq!(k.error_type, "quota_exhausted");
        assert_eq!(k.status, 402);
    }

    #[test]
    fn llm_transient_maps_to_503() {
        let k = classify_llm_for_sse(&LlmError::Transient("".into()));
        assert_eq!(k.error_type, "transient");
        assert_eq!(k.status, 503);
    }

    #[test]
    fn llm_schema_parse_maps_to_unknown_502() {
        let k = classify_llm_for_sse(&LlmError::SchemaParse("".into()));
        assert_eq!(k.error_type, "unknown");
        assert_eq!(k.status, 502);
    }

    #[test]
    fn llm_unknown_maps_to_500() {
        let k = classify_llm_for_sse(&LlmError::Unknown("".into()));
        assert_eq!(k.error_type, "unknown");
        assert_eq!(k.status, 500);
    }

    #[test]
    fn estimation_session_not_ready_maps_to_invalid_state_409() {
        let k = classify_estimation_for_sse(&EstimationServiceError::SessionNotReady);
        assert_eq!(k.error_type, "invalid_state");
        assert_eq!(k.status, 409);
    }

    #[test]
    fn estimation_chat_expired_maps_to_invalid_session_410() {
        let k = classify_estimation_for_sse(&EstimationServiceError::ChatSessionExpired);
        assert_eq!(k.error_type, "invalid_session");
        assert_eq!(k.status, 410);
    }

    #[test]
    fn estimation_chat_not_found_maps_to_invalid_session_404() {
        let k = classify_estimation_for_sse(&EstimationServiceError::ChatSessionNotFound);
        assert_eq!(k.error_type, "invalid_session");
        assert_eq!(k.status, 404);
    }

    #[test]
    fn estimation_missing_byok_maps_to_invalid_key_402() {
        let k = classify_estimation_for_sse(&EstimationServiceError::MissingByokKey);
        assert_eq!(k.error_type, "invalid_key");
        assert_eq!(k.status, 402);
    }

    #[test]
    fn estimation_llm_error_delegates_to_llm_mapping() {
        let k = classify_estimation_for_sse(&EstimationServiceError::Llm(LlmError::Transient(
            "".into(),
        )));
        assert_eq!(k.error_type, "transient");
        assert_eq!(k.status, 503);
    }
}
