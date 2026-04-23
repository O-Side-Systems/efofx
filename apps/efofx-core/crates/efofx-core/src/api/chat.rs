//! Chat session endpoints (Phase 2B).

use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use efofx_auth::middleware::either_auth;
use efofx_domain::{ChatMessage, ChatSession, ScopingContext, SessionId};
use efofx_openapi::{ApiError, ErrorCode};
use efofx_storage::TenantContext;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::services::{ChatService, ChatServiceError};
use crate::AppState;

use super::not_implemented;

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
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 402, description = "BYOK key invalid or quota exhausted", body = ApiError),
        (status = 404, description = "Session not found", body = ApiError),
        (status = 409, description = "Session not ready", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn generate_estimate() -> impl IntoResponse {
    not_implemented("POST /v1/chat/sessions/{id}:generate-estimate")
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
        .route("/v1/chat/sessions/{session_id}", get(get_session_handler))
        .route(
            "/v1/chat/sessions/{session_id}/messages",
            post(append_message),
        )
        .route(
            "/v1/chat/sessions/{session_id}:generate-estimate",
            post(generate_estimate),
        )
        .route_layer(from_fn_with_state(state.auth.clone(), either_auth))
}
