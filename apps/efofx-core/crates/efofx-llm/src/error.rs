//! Classified LLM error surface. Mirrors FastAPI's `classify_openai_error`:
//! auth failures → 402, quota exhaustion → 402, timeouts/rate-limits → 503,
//! everything else → 500. Handlers map these variants to their own `ApiError`
//! error codes.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    /// OpenAI rejected the API key. Surface to the user as "re-enter your key."
    /// Maps to HTTP 402 at the edge.
    #[error("LLM authentication failed: {0}")]
    InvalidKey(String),

    /// OpenAI signaled `insufficient_quota`. Maps to HTTP 402.
    #[error("LLM quota exhausted: {0}")]
    QuotaExhausted(String),

    /// Rate limit, timeout, or transient network error. Caller may retry.
    /// Maps to HTTP 503.
    #[error("transient LLM error: {0}")]
    Transient(String),

    /// The structured response could not be parsed or the model refused.
    /// Maps to HTTP 502 (bad upstream response).
    #[error("LLM structured-output parse failure: {0}")]
    SchemaParse(String),

    /// Unexpected error. Maps to HTTP 500.
    #[error("LLM error: {0}")]
    Unknown(String),
}

impl LlmError {
    /// Stable short tag for logging and metrics. One of `invalid_key`,
    /// `quota_exhausted`, `transient`, `schema_parse`, `unknown`.
    pub fn tag(&self) -> &'static str {
        match self {
            LlmError::InvalidKey(_) => "invalid_key",
            LlmError::QuotaExhausted(_) => "quota_exhausted",
            LlmError::Transient(_) => "transient",
            LlmError::SchemaParse(_) => "schema_parse",
            LlmError::Unknown(_) => "unknown",
        }
    }

    /// Conventional HTTP status for this variant.
    pub fn http_status(&self) -> u16 {
        match self {
            LlmError::InvalidKey(_) | LlmError::QuotaExhausted(_) => 402,
            LlmError::Transient(_) => 503,
            LlmError::SchemaParse(_) => 502,
            LlmError::Unknown(_) => 500,
        }
    }
}

/// Classify an `async-openai` error into our domain variants.
///
/// FastAPI reference: `app/services/llm_service.py::classify_openai_error`.
/// - `AuthenticationError` / HTTP 401 / code `"invalid_api_key"` → InvalidKey
/// - `RateLimitError` with `insufficient_quota` → QuotaExhausted
/// - Other rate-limit / timeout / connection errors → Transient
/// - Everything else → Unknown
pub fn classify_openai_error(err: async_openai::error::OpenAIError) -> LlmError {
    use async_openai::error::OpenAIError;

    match err {
        OpenAIError::ApiError(api_err) => {
            let msg = api_err.message.clone();
            let code = api_err.code.as_deref().unwrap_or("");
            let err_type = api_err.r#type.as_deref().unwrap_or("");

            if code == "invalid_api_key"
                || err_type.eq_ignore_ascii_case("invalid_request_error")
                    && msg.to_lowercase().contains("api key")
                || err_type.eq_ignore_ascii_case("authentication_error")
            {
                return LlmError::InvalidKey(msg);
            }
            if code == "insufficient_quota" || msg.to_lowercase().contains("insufficient_quota") {
                return LlmError::QuotaExhausted(msg);
            }
            if code == "rate_limit_exceeded" || err_type.eq_ignore_ascii_case("rate_limit_error") {
                return LlmError::Transient(msg);
            }
            LlmError::Unknown(format!("api_error[{code}]: {msg}"))
        }
        OpenAIError::Reqwest(e) => {
            if e.is_timeout() || e.is_connect() {
                LlmError::Transient(e.to_string())
            } else {
                LlmError::Unknown(format!("network: {e}"))
            }
        }
        OpenAIError::StreamError(e) => LlmError::Transient(format!("stream: {e}")),
        OpenAIError::InvalidArgument(e) => LlmError::Unknown(format!("invalid_argument: {e}")),
        OpenAIError::JSONDeserialize(e, content) => {
            LlmError::SchemaParse(format!("json error `{e}` on content `{content}`"))
        }
        OpenAIError::FileSaveError(e) | OpenAIError::FileReadError(e) => {
            LlmError::Unknown(format!("file: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_stable() {
        assert_eq!(LlmError::InvalidKey("".into()).tag(), "invalid_key");
        assert_eq!(LlmError::QuotaExhausted("".into()).tag(), "quota_exhausted");
        assert_eq!(LlmError::Transient("".into()).tag(), "transient");
        assert_eq!(LlmError::SchemaParse("".into()).tag(), "schema_parse");
        assert_eq!(LlmError::Unknown("".into()).tag(), "unknown");
    }

    #[test]
    fn http_status_matches_fastapi() {
        assert_eq!(LlmError::InvalidKey("".into()).http_status(), 402);
        assert_eq!(LlmError::QuotaExhausted("".into()).http_status(), 402);
        assert_eq!(LlmError::Transient("".into()).http_status(), 503);
        assert_eq!(LlmError::SchemaParse("".into()).http_status(), 502);
        assert_eq!(LlmError::Unknown("".into()).http_status(), 500);
    }
}
