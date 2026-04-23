//! The `LlmProvider` trait — the seam between the chat/estimation layers
//! and whichever LLM backend is configured. Every method takes a BYOK
//! `api_key` per call: providers are stateless across tenants.

use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::error::LlmError;
use crate::types::{ChatRequest, ChatResponse, StreamEvent, StructuredResponse, StructuredSchema};

/// Abstract LLM backend. Implementors: [`crate::OpenAiProvider`] (real) and
/// [`crate::MockLlmProvider`] (tests). The `Send + Sync` bound lets the
/// provider live behind an `Arc<dyn LlmProvider>` in Axum state.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Free-form text completion. Returns the full response and token usage.
    async fn complete(&self, api_key: &str, req: ChatRequest) -> Result<ChatResponse, LlmError>;

    /// JSON-schema-constrained completion. The provider sets `strict: true`
    /// so the response is guaranteed to match the schema — or the call
    /// fails with [`LlmError::SchemaParse`].
    async fn complete_structured(
        &self,
        api_key: &str,
        req: ChatRequest,
        schema: StructuredSchema,
    ) -> Result<StructuredResponse, LlmError>;

    /// Streaming chat completion. The returned stream yields text deltas
    /// only (role-only and finish-only chunks are filtered). The final item
    /// is always [`StreamEvent::Done`].
    async fn stream(
        &self,
        api_key: &str,
        req: ChatRequest,
    ) -> Result<BoxStream<'static, Result<StreamEvent, LlmError>>, LlmError>;
}
