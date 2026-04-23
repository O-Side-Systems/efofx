//! Stable error-code registry.
//!
//! Codes are part of the API contract — once published they cannot be
//! renamed or removed without a major version bump. Messages are defaults;
//! handlers may provide tenant-specific overrides when appropriate.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Machine-readable error code. Rendered as a snake-cased string on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    // Auth
    AuthMissingToken,
    AuthInvalidToken,
    AuthExpiredToken,
    AuthWrongAudience,
    AuthApiKeyInvalid,
    AuthForbidden,

    // Common
    RateLimitExceeded,
    ValidationFailed,

    // Chat
    ChatSessionNotFound,
    ChatSessionExpired,
    ChatSessionNotReady,
    ChatMessageLimitExceeded,
    ChatTokenBudgetExceeded,
    ChatEmptyMessage,
    ChatLlmInvalidKey,
    ChatLlmQuotaExhausted,
    ChatLlmTransient,
    ChatLlmUnknown,

    // Estimation
    EstimationSessionNotFound,
    EstimationLlmInvalidKey,
    EstimationLlmQuotaExhausted,
    EstimationLlmTransient,

    // Feedback
    FeedbackTokenExpired,
    FeedbackTokenUsed,

    // Widget
    WidgetBrandingNotFound,
    WidgetInvalidEventType,
}

impl ErrorCode {
    /// Canonical dot-notation string (e.g. `"auth.invalid_token"`). Used on
    /// the wire by `ApiErrorDetail::code`. The serde `snake_case` rename on
    /// the enum variants omits the group prefix; this method re-attaches it.
    pub fn as_str(&self) -> &'static str {
        use ErrorCode::*;
        match self {
            AuthMissingToken => "auth.missing_token",
            AuthInvalidToken => "auth.invalid_token",
            AuthExpiredToken => "auth.expired_token",
            AuthWrongAudience => "auth.wrong_audience",
            AuthApiKeyInvalid => "auth.api_key_invalid",
            AuthForbidden => "auth.forbidden",
            RateLimitExceeded => "rate_limit.exceeded",
            ValidationFailed => "validation.failed",
            ChatSessionNotFound => "chat.session_not_found",
            ChatSessionExpired => "chat.session_expired",
            ChatSessionNotReady => "chat.session_not_ready",
            ChatMessageLimitExceeded => "chat.message_limit_exceeded",
            ChatTokenBudgetExceeded => "chat.token_budget_exceeded",
            ChatEmptyMessage => "chat.empty_message",
            ChatLlmInvalidKey => "chat.llm_invalid_key",
            ChatLlmQuotaExhausted => "chat.llm_quota_exhausted",
            ChatLlmTransient => "chat.llm_transient",
            ChatLlmUnknown => "chat.llm_unknown",
            EstimationSessionNotFound => "estimation.session_not_found",
            EstimationLlmInvalidKey => "estimation.llm_invalid_key",
            EstimationLlmQuotaExhausted => "estimation.llm_quota_exhausted",
            EstimationLlmTransient => "estimation.llm_transient",
            FeedbackTokenExpired => "feedback.token_expired",
            FeedbackTokenUsed => "feedback.token_used",
            WidgetBrandingNotFound => "widget.branding_not_found",
            WidgetInvalidEventType => "widget.invalid_event_type",
        }
    }

    /// Default human-readable message. Handlers may override.
    pub fn default_message(&self) -> &'static str {
        use ErrorCode::*;
        match self {
            AuthMissingToken => "Authentication required.",
            AuthInvalidToken => "Authentication token is invalid.",
            AuthExpiredToken => "Authentication token has expired.",
            AuthWrongAudience => "Authentication token not issued for this service.",
            AuthApiKeyInvalid => "Widget API key is invalid.",
            AuthForbidden => "Not authorized.",
            RateLimitExceeded => "Too many requests. Try again later.",
            ValidationFailed => "Request validation failed.",
            ChatSessionNotFound => "Chat session not found.",
            ChatSessionExpired => "Chat session has expired.",
            ChatSessionNotReady => "Chat session is not ready for estimation.",
            ChatMessageLimitExceeded => "Message limit for this session reached.",
            ChatTokenBudgetExceeded => "Token budget for this session has been reached.",
            ChatEmptyMessage => "Chat message must be non-empty.",
            ChatLlmInvalidKey => "OpenAI API key missing or invalid. Update it in Settings.",
            ChatLlmQuotaExhausted => "OpenAI quota exhausted. Recharge your OpenAI account.",
            ChatLlmTransient => "We're having trouble generating a response. Please try again.",
            ChatLlmUnknown => "An unexpected LLM error occurred.",
            EstimationSessionNotFound => "Estimation session not found.",
            EstimationLlmInvalidKey => "OpenAI API key missing or invalid. Update it in Settings.",
            EstimationLlmQuotaExhausted => "OpenAI quota exhausted. Recharge your OpenAI account.",
            EstimationLlmTransient => {
                "We're having trouble generating a response. Please try again."
            }
            FeedbackTokenExpired => "This feedback link has expired.",
            FeedbackTokenUsed => "This feedback has already been submitted.",
            WidgetBrandingNotFound => "Widget branding not found for the supplied API key prefix.",
            WidgetInvalidEventType => "Analytics event type is not recognized.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_use_dot_notation() {
        assert_eq!(ErrorCode::AuthInvalidToken.as_str(), "auth.invalid_token");
        assert_eq!(
            ErrorCode::ChatSessionExpired.as_str(),
            "chat.session_expired"
        );
    }

    #[test]
    fn every_code_has_message() {
        for code in [
            ErrorCode::AuthMissingToken,
            ErrorCode::AuthInvalidToken,
            ErrorCode::AuthExpiredToken,
            ErrorCode::AuthWrongAudience,
            ErrorCode::AuthApiKeyInvalid,
            ErrorCode::AuthForbidden,
            ErrorCode::RateLimitExceeded,
            ErrorCode::ValidationFailed,
            ErrorCode::ChatSessionNotFound,
            ErrorCode::ChatSessionExpired,
            ErrorCode::ChatSessionNotReady,
            ErrorCode::ChatMessageLimitExceeded,
            ErrorCode::ChatTokenBudgetExceeded,
            ErrorCode::ChatEmptyMessage,
            ErrorCode::ChatLlmInvalidKey,
            ErrorCode::ChatLlmQuotaExhausted,
            ErrorCode::ChatLlmTransient,
            ErrorCode::ChatLlmUnknown,
            ErrorCode::EstimationSessionNotFound,
            ErrorCode::EstimationLlmInvalidKey,
            ErrorCode::EstimationLlmQuotaExhausted,
            ErrorCode::EstimationLlmTransient,
            ErrorCode::FeedbackTokenExpired,
            ErrorCode::FeedbackTokenUsed,
            ErrorCode::WidgetBrandingNotFound,
            ErrorCode::WidgetInvalidEventType,
        ] {
            assert!(!code.default_message().is_empty());
            assert!(code.as_str().contains('.'));
        }
    }
}
