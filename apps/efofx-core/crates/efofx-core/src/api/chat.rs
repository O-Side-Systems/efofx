//! Chat session endpoints (Phase 2B).

use std::sync::Arc;

use axum::{
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use efofx_domain::{ChatMessage, ChatSession, ScopingContext};
use efofx_openapi::ApiError;

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

/// Create a new chat session for the authenticated tenant.
#[utoipa::path(
    post,
    path = "/v1/chat/sessions",
    tag = "chat",
    request_body = CreateChatSessionRequest,
    security(("supabase_jwt" = []), ("widget_api_key" = [])),
    responses(
        (status = 201, description = "Session created", body = ChatSession),
        (status = 401, description = "Authentication required", body = ApiError),
        (status = 429, description = "Rate limit exceeded", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn create_session() -> impl IntoResponse {
    not_implemented("POST /v1/chat/sessions")
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
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn get_session() -> impl IntoResponse {
    not_implemented("GET /v1/chat/sessions/{id}")
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
        (status = 404, description = "Session not found", body = ApiError),
        (status = 410, description = "Session expired", body = ApiError),
        (status = 429, description = "Message limit or rate limit hit", body = ApiError),
        (status = 501, description = "Not yet implemented", body = ApiError),
    ),
)]
pub async fn append_message() -> impl IntoResponse {
    not_implemented("POST /v1/chat/sessions/{id}/messages")
}

/// Generate a structured estimate from a ready chat session. Responds
/// with `text/event-stream`: emits `thinking`, then `estimate`, then
/// streams `narrative` tokens, then `done`. On error, emits `error`
/// and closes.
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

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/chat/sessions", post(create_session))
        .route("/v1/chat/sessions/{session_id}", get(get_session))
        .route(
            "/v1/chat/sessions/{session_id}/messages",
            post(append_message),
        )
        .route(
            "/v1/chat/sessions/{session_id}:generate-estimate",
            post(generate_estimate),
        )
}
