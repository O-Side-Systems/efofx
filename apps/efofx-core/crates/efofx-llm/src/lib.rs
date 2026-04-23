//! LLM provider abstraction for the efofx platform.
//!
//! - [`LlmProvider`] is the trait every backend implements. Methods take a
//!   BYOK `api_key` per call; providers hold no per-tenant state.
//! - [`OpenAiProvider`] wraps `async-openai` 0.35 with classified errors,
//!   structured-output, and streaming support.
//! - [`MockLlmProvider`] is a deterministic test double with call recording.
//! - [`tokens`] counts tokens via `tiktoken-rs` for budget enforcement.
//!
//! # Design notes
//!
//! The trait intentionally does not expose `async-openai` types: the whole
//! OpenAI SDK is an implementation detail. Adding another provider (Anthropic,
//! local) would only require a new struct implementing [`LlmProvider`].

pub mod error;
pub mod mock;
pub mod openai;
pub mod provider;
pub mod tokens;
pub mod types;

pub use error::{classify_openai_error, LlmError};
pub use mock::{MockLlmProvider, MockScript, Recorded};
pub use openai::OpenAiProvider;
pub use provider::LlmProvider;
pub use tokens::{count_chat_tokens, count_tokens, TokenError};
pub use types::{
    ChatMessage, ChatRequest, ChatResponse, Role, StreamEvent, StructuredResponse,
    StructuredSchema, Usage,
};
