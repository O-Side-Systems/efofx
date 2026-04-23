//! Chat session service.
//!
//! Owns the state-machine transitions for `/v1/chat/sessions/*` and coordinates
//! three collaborators:
//!
//! - [`ChatRepo`] — persistence (session doc + append + budget counters).
//! - [`PromptRegistry`] — versioned scoping prompt.
//! - [`LlmProvider`] — the assistant follow-up generator.
//!
//! Every public method returns [`ChatServiceError`], which knows how to
//! render itself into a classified HTTP response via `IntoResponse`.

use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use efofx_config::LlmConfig;
use efofx_domain::{
    extract_scoping, is_explicit_estimate_trigger, ChatMessage, ChatSession, ChatStatus,
    MessageRole, SessionId,
};
use efofx_llm::{ChatRequest as LlmReq, LlmError, LlmProvider};
use efofx_openapi::{ApiError, ErrorCode};
use efofx_prompts::{PromptError, PromptRegistry};
use efofx_storage::{ChatRepo, StorageError, TenantContext};
use time::OffsetDateTime;
use tracing::{info, warn};

/// Max characters accepted in a single user message. Mirrors FastAPI's
/// `min_length=1, max_length=2000` on `ChatRequest.message`.
pub const MAX_MESSAGE_CHARS: usize = 2000;

#[derive(Debug, thiserror::Error)]
pub enum ChatServiceError {
    #[error("session not found")]
    NotFound,
    #[error("session expired")]
    Expired,
    #[error("empty message")]
    EmptyMessage,
    #[error("message exceeds {MAX_MESSAGE_CHARS} character limit")]
    MessageTooLong,
    #[error("message count limit ({limit}) reached on session")]
    MessageLimitExceeded { limit: u32 },
    #[error("token budget ({limit}) exceeded on session")]
    TokenBudgetExceeded { limit: u64, used: u64 },
    #[error("tenant has no BYOK OpenAI key on file")]
    MissingByokKey,
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Prompt(#[from] PromptError),
    #[error("llm call failed: {0}")]
    Llm(LlmError),
}

impl From<LlmError> for ChatServiceError {
    fn from(e: LlmError) -> Self {
        ChatServiceError::Llm(e)
    }
}

impl IntoResponse for ChatServiceError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            ChatServiceError::NotFound => (
                StatusCode::NOT_FOUND,
                ErrorCode::ChatSessionNotFound.as_str(),
                ErrorCode::ChatSessionNotFound.default_message().to_string(),
            ),
            ChatServiceError::Expired => (
                StatusCode::GONE,
                ErrorCode::ChatSessionExpired.as_str(),
                ErrorCode::ChatSessionExpired.default_message().to_string(),
            ),
            ChatServiceError::EmptyMessage => (
                StatusCode::BAD_REQUEST,
                ErrorCode::ChatEmptyMessage.as_str(),
                ErrorCode::ChatEmptyMessage.default_message().to_string(),
            ),
            ChatServiceError::MessageTooLong => (
                StatusCode::BAD_REQUEST,
                ErrorCode::ValidationFailed.as_str(),
                format!("Message exceeds {MAX_MESSAGE_CHARS}-character limit"),
            ),
            ChatServiceError::MessageLimitExceeded { limit } => (
                StatusCode::TOO_MANY_REQUESTS,
                ErrorCode::ChatMessageLimitExceeded.as_str(),
                format!("Message limit of {limit} reached on session."),
            ),
            ChatServiceError::TokenBudgetExceeded { limit, used } => (
                StatusCode::TOO_MANY_REQUESTS,
                ErrorCode::ChatTokenBudgetExceeded.as_str(),
                format!("Session token budget exceeded ({used}/{limit})."),
            ),
            ChatServiceError::MissingByokKey => (
                StatusCode::PAYMENT_REQUIRED,
                ErrorCode::ChatLlmInvalidKey.as_str(),
                "No OpenAI API key on file. Add one in Settings.".to_string(),
            ),
            ChatServiceError::Llm(e) => {
                let (code, http) = match e {
                    LlmError::InvalidKey(_) => {
                        (ErrorCode::ChatLlmInvalidKey, StatusCode::PAYMENT_REQUIRED)
                    }
                    LlmError::QuotaExhausted(_) => (
                        ErrorCode::ChatLlmQuotaExhausted,
                        StatusCode::PAYMENT_REQUIRED,
                    ),
                    LlmError::Transient(_) => {
                        (ErrorCode::ChatLlmTransient, StatusCode::SERVICE_UNAVAILABLE)
                    }
                    LlmError::SchemaParse(_) | LlmError::Unknown(_) => {
                        (ErrorCode::ChatLlmUnknown, StatusCode::INTERNAL_SERVER_ERROR)
                    }
                };
                (http, code.as_str(), code.default_message().to_string())
            }
            ChatServiceError::Storage(StorageError::NotFound) => (
                StatusCode::NOT_FOUND,
                ErrorCode::ChatSessionNotFound.as_str(),
                ErrorCode::ChatSessionNotFound.default_message().to_string(),
            ),
            ChatServiceError::Storage(e) => {
                tracing::error!(error = %e, "chat storage failure");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "common.internal",
                    "An internal error occurred".to_string(),
                )
            }
            ChatServiceError::Prompt(e) => {
                tracing::error!(error = %e, "prompt render failure");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "common.internal",
                    "An internal error occurred".to_string(),
                )
            }
        };
        (status, Json(ApiError::single(code, message))).into_response()
    }
}

/// Orchestrates chat session state transitions.
#[derive(Clone)]
pub struct ChatService {
    chat_repo: ChatRepo,
    prompts: Arc<PromptRegistry>,
    llm: Arc<dyn LlmProvider>,
    llm_cfg: LlmConfig,
}

impl ChatService {
    pub fn new(
        chat_repo: ChatRepo,
        prompts: Arc<PromptRegistry>,
        llm: Arc<dyn LlmProvider>,
        llm_cfg: LlmConfig,
    ) -> Self {
        Self {
            chat_repo,
            prompts,
            llm,
            llm_cfg,
        }
    }

    /// Create a new session for this tenant. If `initial_message` is supplied
    /// we append it and run the normal append-flow so the caller gets back a
    /// session with one user message + assistant follow-up in one round-trip.
    pub async fn create_session(
        &self,
        ctx: &TenantContext,
        api_key: &str,
        initial_message: Option<String>,
    ) -> Result<ChatSession, ChatServiceError> {
        let prompt_version = self.prompts.latest("scoping").map(|p| p.version.clone());
        let session = self.chat_repo.create(ctx, prompt_version, None).await?;
        info!(session_id = %session.session_id, "chat session created");

        if let Some(msg) = initial_message {
            return self
                .append_message(ctx, api_key, session.session_id, msg)
                .await
                .map(|o| o.session);
        }
        Ok(session)
    }

    /// Fetch a session by id. Fails with `NotFound` if missing or belongs to
    /// another tenant.
    pub async fn get_session(
        &self,
        ctx: &TenantContext,
        session_id: SessionId,
    ) -> Result<ChatSession, ChatServiceError> {
        self.chat_repo.get(ctx, session_id).await.map_err(|e| {
            if matches!(e, StorageError::NotFound) {
                ChatServiceError::NotFound
            } else {
                e.into()
            }
        })
    }

    /// Append a user message. Runs readiness extraction, generates the
    /// assistant follow-up via the LLM, persists both messages and the
    /// updated scoping context, and returns the fresh session along with
    /// the assistant message for the handler to echo.
    pub async fn append_message(
        &self,
        ctx: &TenantContext,
        api_key: &str,
        session_id: SessionId,
        user_message: String,
    ) -> Result<AppendOutcome, ChatServiceError> {
        // --- validate input ---
        let message = user_message.trim();
        if message.is_empty() {
            return Err(ChatServiceError::EmptyMessage);
        }
        if message.chars().count() > MAX_MESSAGE_CHARS {
            return Err(ChatServiceError::MessageTooLong);
        }

        // --- load + verify session status ---
        let mut session = self.get_session(ctx, session_id).await?;
        let now = OffsetDateTime::now_utc();
        if session.expires_at <= now || session.status == ChatStatus::Expired {
            return Err(ChatServiceError::Expired);
        }

        // --- enforce budgets before we do anything that costs money ---
        let budgets = &self.llm_cfg.budgets;
        if budgets.per_session_messages > 0
            && session.messages.len() as u32 >= budgets.per_session_messages
        {
            return Err(ChatServiceError::MessageLimitExceeded {
                limit: budgets.per_session_messages,
            });
        }
        if budgets.per_session_tokens > 0
            && session.token_usage.total_tokens >= budgets.per_session_tokens
        {
            return Err(ChatServiceError::TokenBudgetExceeded {
                limit: budgets.per_session_tokens,
                used: session.token_usage.total_tokens,
            });
        }

        // --- persist the user message ---
        let user_msg = ChatMessage {
            role: MessageRole::User,
            content: message.to_string(),
            timestamp: now,
        };
        session = self
            .chat_repo
            .append_message(ctx, session_id, &user_msg)
            .await?;

        // --- extract scoping + decide readiness ---
        let updated_scoping = extract_scoping(session.scoping_context.clone(), message);
        let ready_by_fields = updated_scoping.is_ready();
        let ready_by_trigger = is_explicit_estimate_trigger(message);
        let is_ready = ready_by_fields || ready_by_trigger;
        self.chat_repo
            .update_scoping_context(ctx, session_id, &updated_scoping, is_ready)
            .await?;
        session.scoping_context = updated_scoping;
        session.is_ready = is_ready;

        // --- generate assistant follow-up ---
        let assistant_content = self
            .generate_assistant_followup(ctx, &session, api_key, is_ready)
            .await?;

        let assistant_msg = ChatMessage {
            role: MessageRole::Assistant,
            content: assistant_content,
            timestamp: OffsetDateTime::now_utc(),
        };
        session = self
            .chat_repo
            .append_message(ctx, session_id, &assistant_msg)
            .await?;

        // --- transition status if ready ---
        if is_ready && session.status == ChatStatus::Active {
            self.chat_repo
                .update_status(ctx, session_id, ChatStatus::Ready)
                .await?;
            session.status = ChatStatus::Ready;
        }

        Ok(AppendOutcome {
            session,
            assistant_message: assistant_msg,
        })
    }

    /// Generate the next assistant turn. A `ready` session short-circuits
    /// to a canned confirmation so we don't spend tokens re-asking
    /// questions once the user has given us enough context.
    async fn generate_assistant_followup(
        &self,
        ctx: &TenantContext,
        session: &ChatSession,
        api_key: &str,
        is_ready: bool,
    ) -> Result<String, ChatServiceError> {
        if is_ready {
            return Ok("Great — I have enough detail to put together an estimate. \
                 Tap **generate** when you're ready."
                .to_string());
        }

        let prompt = self
            .prompts
            .latest("scoping")
            .ok_or_else(|| PromptError::NotFound {
                name: "scoping".into(),
                version: "latest".into(),
            })?;

        let history = format_history(&session.messages);
        let scoping_ctx = format_scoping(&session.scoping_context);
        let last_user = session
            .messages
            .iter()
            .rev()
            .find(|m| m.role == MessageRole::User)
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let user_prompt = prompt.render_user(&[
            ("conversation_history", history.as_str()),
            ("user_message", last_user),
            ("scoping_context", scoping_ctx.as_str()),
        ])?;

        let mut req = LlmReq::simple(
            &self.llm_cfg.model,
            Some(&prompt.system_prompt),
            user_prompt,
        );
        if let Some(t) = self.llm_cfg.temperature {
            req = req.with_temperature(t);
        }
        if let Some(n) = self.llm_cfg.max_tokens {
            req = req.with_max_tokens(n);
        }

        let resp = self.llm.complete(api_key, req).await.map_err(|e| {
            warn!(error = %e, tag = e.tag(), "llm complete failed");
            e
        })?;

        if resp.usage.total_tokens > 0 {
            // Update cumulative usage so the NEXT request's budget check
            // sees the post-increment total. Cheap single-doc atomic $inc.
            if let Err(e) = self
                .chat_repo
                .increment_token_usage(
                    ctx,
                    session.session_id,
                    resp.usage.prompt_tokens as u64,
                    resp.usage.completion_tokens as u64,
                )
                .await
            {
                // Don't fail the user-facing call just because usage couldn't
                // be written back. Budgets will catch up on the next message.
                warn!(error = %e, "failed to persist token usage");
            }
        }

        Ok(resp.content)
    }
}

/// Return value for [`ChatService::append_message`].
#[derive(Debug, Clone)]
pub struct AppendOutcome {
    pub session: ChatSession,
    pub assistant_message: ChatMessage,
}

/// Build the system/user history string the scoping prompt expects.
fn format_history(messages: &[ChatMessage]) -> String {
    let recent = messages.iter().rev().take(10).collect::<Vec<_>>();
    let mut lines = Vec::with_capacity(recent.len());
    for m in recent.iter().rev() {
        let role = match m.role {
            MessageRole::User => "USER",
            MessageRole::Assistant => "ASSISTANT",
            MessageRole::System => "SYSTEM",
        };
        lines.push(format!("{role}: {}", m.content));
    }
    lines.join("\n")
}

/// Pretty-print the current scoping context as "- field: value" lines,
/// matching the FastAPI scoping prompt's expected shape.
fn format_scoping(ctx: &efofx_domain::ScopingContext) -> String {
    let mut lines: Vec<String> = Vec::new();
    let fields: [(&str, Option<&String>); 5] = [
        ("project_type", ctx.project_type.as_ref()),
        ("project_size", ctx.project_size.as_ref()),
        ("location", ctx.location.as_ref()),
        ("timeline", ctx.timeline.as_ref()),
        ("special_conditions", ctx.special_conditions.as_ref()),
    ];
    for (name, val) in fields {
        if let Some(v) = val {
            lines.push(format!("- {name}: {v}"));
        }
    }
    if lines.is_empty() {
        "Nothing gathered yet.".to_string()
    } else {
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use efofx_domain::{ChatMessage, MessageRole, ScopingContext};
    use time::OffsetDateTime;

    fn msg(role: MessageRole, content: &str) -> ChatMessage {
        ChatMessage {
            role,
            content: content.to_string(),
            timestamp: OffsetDateTime::now_utc(),
        }
    }

    #[test]
    fn format_history_labels_roles_and_joins_newlines() {
        let history = format_history(&[
            msg(MessageRole::System, "be terse"),
            msg(MessageRole::User, "hi"),
            msg(MessageRole::Assistant, "hello"),
        ]);
        assert_eq!(history, "SYSTEM: be terse\nUSER: hi\nASSISTANT: hello");
    }

    #[test]
    fn format_history_caps_at_last_ten_messages() {
        let many: Vec<ChatMessage> = (0..15)
            .map(|i| msg(MessageRole::User, &format!("m{i}")))
            .collect();
        let history = format_history(&many);
        // Last 10 only — the first line should be "USER: m5".
        assert!(history.starts_with("USER: m5\n"));
        assert!(history.ends_with("USER: m14"));
        assert_eq!(history.matches('\n').count(), 9);
    }

    #[test]
    fn format_scoping_empty_prints_fallback() {
        let ctx = ScopingContext::default();
        assert_eq!(format_scoping(&ctx), "Nothing gathered yet.");
    }

    #[test]
    fn format_scoping_lists_only_populated_fields() {
        let mut ctx = ScopingContext::default();
        ctx.project_type = Some("pool".into());
        ctx.timeline = Some("spring 2026".into());
        let out = format_scoping(&ctx);
        assert!(out.contains("- project_type: pool"));
        assert!(out.contains("- timeline: spring 2026"));
        assert!(!out.contains("project_size"));
        assert!(!out.contains("location"));
    }

    #[test]
    fn append_outcome_is_cloneable_and_serializable_via_session() {
        // Smoke-test: AppendOutcome wraps a ChatSession; the session must be
        // serializable for the handler to return it over JSON.
        let session = ChatSession {
            session_id: efofx_domain::SessionId::new(),
            tenant_id: efofx_domain::TenantId::new(),
            status: ChatStatus::Ready,
            messages: vec![msg(MessageRole::User, "hello")],
            scoping_context: ScopingContext::default(),
            is_ready: true,
            prompt_version: Some("1.0.0".into()),
            token_usage: Default::default(),
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
            expires_at: OffsetDateTime::now_utc() + time::Duration::hours(24),
        };
        let outcome = AppendOutcome {
            session: session.clone(),
            assistant_message: msg(MessageRole::Assistant, "hi back"),
        };
        let json = serde_json::to_string(&outcome.session).expect("serialize session");
        assert!(json.contains("\"status\":\"ready\""));
    }

    #[test]
    fn error_response_codes_map_correctly() {
        let cases = [
            (ChatServiceError::NotFound, StatusCode::NOT_FOUND),
            (ChatServiceError::Expired, StatusCode::GONE),
            (ChatServiceError::EmptyMessage, StatusCode::BAD_REQUEST),
            (
                ChatServiceError::MessageLimitExceeded { limit: 50 },
                StatusCode::TOO_MANY_REQUESTS,
            ),
            (
                ChatServiceError::TokenBudgetExceeded {
                    limit: 1000,
                    used: 1200,
                },
                StatusCode::TOO_MANY_REQUESTS,
            ),
            (
                ChatServiceError::MissingByokKey,
                StatusCode::PAYMENT_REQUIRED,
            ),
        ];
        for (err, expected) in cases {
            let resp = err.into_response();
            assert_eq!(resp.status(), expected);
        }
    }
}
