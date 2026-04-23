//! `async-openai` 0.35 implementation of [`LlmProvider`].
//!
//! The provider struct is cheap to clone and carries no per-tenant state.
//! Each call constructs a per-request `Client` with the supplied BYOK key.

use std::time::Duration;

use async_openai::config::OpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CompletionUsage,
    CreateChatCompletionRequest, CreateChatCompletionRequestArgs, ResponseFormat,
    ResponseFormatJsonSchema,
};
use async_openai::Client;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use tracing::debug;

use crate::error::{classify_openai_error, LlmError};
use crate::provider::LlmProvider;
use crate::types::{
    ChatMessage, ChatRequest, ChatResponse, Role, StreamEvent, StructuredResponse,
    StructuredSchema, Usage,
};

/// Concrete provider backed by OpenAI (or an API-compatible endpoint, if
/// `api_base` is overridden).
#[derive(Debug, Clone)]
pub struct OpenAiProvider {
    api_base: Option<String>,
    request_timeout: Duration,
}

impl OpenAiProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_api_base(mut self, base: impl Into<String>) -> Self {
        self.api_base = Some(base.into());
        self
    }

    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    fn build_client(&self, api_key: &str) -> Client<OpenAIConfig> {
        let mut cfg = OpenAIConfig::default().with_api_key(api_key);
        if let Some(base) = &self.api_base {
            cfg = cfg.with_api_base(base);
        }
        Client::with_config(cfg)
    }

    /// Wrap a future with the provider's configured timeout. Times out as
    /// `LlmError::Transient`.
    async fn with_timeout<F, T>(&self, fut: F) -> Result<T, LlmError>
    where
        F: std::future::Future<Output = Result<T, LlmError>>,
    {
        match tokio::time::timeout(self.request_timeout, fut).await {
            Ok(res) => res,
            Err(_) => Err(LlmError::Transient(format!(
                "request exceeded {}ms timeout",
                self.request_timeout.as_millis()
            ))),
        }
    }
}

impl Default for OpenAiProvider {
    fn default() -> Self {
        Self {
            api_base: None,
            request_timeout: Duration::from_secs(30),
        }
    }
}

fn to_openai_messages(
    messages: &[ChatMessage],
) -> Result<Vec<ChatCompletionRequestMessage>, LlmError> {
    messages
        .iter()
        .map(|m| -> Result<_, LlmError> {
            Ok(match m.role {
                Role::System => ChatCompletionRequestSystemMessageArgs::default()
                    .content(m.content.as_str())
                    .build()
                    .map_err(|e| LlmError::Unknown(format!("build system msg: {e}")))?
                    .into(),
                Role::User => ChatCompletionRequestUserMessageArgs::default()
                    .content(m.content.as_str())
                    .build()
                    .map_err(|e| LlmError::Unknown(format!("build user msg: {e}")))?
                    .into(),
                Role::Assistant => ChatCompletionRequestAssistantMessageArgs::default()
                    .content(m.content.as_str())
                    .build()
                    .map_err(|e| LlmError::Unknown(format!("build assistant msg: {e}")))?
                    .into(),
            })
        })
        .collect()
}

fn build_request(
    req: &ChatRequest,
    response_format: Option<ResponseFormat>,
    stream: bool,
) -> Result<CreateChatCompletionRequest, LlmError> {
    let messages = to_openai_messages(&req.messages)?;
    let mut builder = CreateChatCompletionRequestArgs::default();
    builder.model(&req.model).messages(messages).stream(stream);

    if let Some(t) = req.temperature {
        builder.temperature(t);
    }
    if let Some(n) = req.max_tokens {
        builder.max_tokens(n);
    }
    if let Some(rf) = response_format {
        builder.response_format(rf);
    }

    builder
        .build()
        .map_err(|e| LlmError::Unknown(format!("build request: {e}")))
}

fn usage_from_openai(u: Option<CompletionUsage>) -> Usage {
    u.map(|u| Usage {
        prompt_tokens: u.prompt_tokens,
        completion_tokens: u.completion_tokens,
        total_tokens: u.total_tokens,
    })
    .unwrap_or_default()
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(&self, api_key: &str, req: ChatRequest) -> Result<ChatResponse, LlmError> {
        let client = self.build_client(api_key);
        let openai_req = build_request(&req, None, false)?;

        debug!(model = %req.model, messages = req.messages.len(), "llm complete");

        self.with_timeout(async move {
            let resp = client
                .chat()
                .create(openai_req)
                .await
                .map_err(classify_openai_error)?;

            let choice = resp
                .choices
                .into_iter()
                .next()
                .ok_or_else(|| LlmError::Unknown("no choices returned".into()))?;
            let content = choice.message.content.unwrap_or_default();

            Ok(ChatResponse {
                content,
                usage: usage_from_openai(resp.usage),
            })
        })
        .await
    }

    async fn complete_structured(
        &self,
        api_key: &str,
        req: ChatRequest,
        schema: StructuredSchema,
    ) -> Result<StructuredResponse, LlmError> {
        let client = self.build_client(api_key);

        let response_format = ResponseFormat::JsonSchema {
            json_schema: ResponseFormatJsonSchema {
                name: schema.name.clone(),
                description: schema.description.clone(),
                schema: Some(schema.schema.clone()),
                strict: Some(true),
            },
        };

        let openai_req = build_request(&req, Some(response_format), false)?;

        debug!(model = %req.model, schema = %schema.name, "llm complete_structured");

        self.with_timeout(async move {
            let resp = client
                .chat()
                .create(openai_req)
                .await
                .map_err(classify_openai_error)?;

            let choice = resp
                .choices
                .into_iter()
                .next()
                .ok_or_else(|| LlmError::Unknown("no choices returned".into()))?;

            if let Some(refusal) = choice.message.refusal {
                if !refusal.is_empty() {
                    return Err(LlmError::SchemaParse(format!("model refusal: {refusal}")));
                }
            }

            let raw = choice
                .message
                .content
                .ok_or_else(|| LlmError::SchemaParse("empty structured response".into()))?;
            let data: serde_json::Value = serde_json::from_str(&raw)
                .map_err(|e| LlmError::SchemaParse(format!("parse json: {e}")))?;

            Ok(StructuredResponse {
                data,
                usage: usage_from_openai(resp.usage),
            })
        })
        .await
    }

    async fn stream(
        &self,
        api_key: &str,
        req: ChatRequest,
    ) -> Result<BoxStream<'static, Result<StreamEvent, LlmError>>, LlmError> {
        let client = self.build_client(api_key);
        let openai_req = build_request(&req, None, true)?;

        debug!(model = %req.model, "llm stream");

        let mut upstream = client
            .chat()
            .create_stream(openai_req)
            .await
            .map_err(classify_openai_error)?;

        // Filter to text deltas; emit a single Done at the end.
        let out = async_stream::try_stream! {
            let mut final_usage: Option<Usage> = None;
            while let Some(item) = upstream.next().await {
                let chunk = item.map_err(classify_openai_error)?;
                if let Some(u) = chunk.usage {
                    final_usage = Some(Usage {
                        prompt_tokens: u.prompt_tokens,
                        completion_tokens: u.completion_tokens,
                        total_tokens: u.total_tokens,
                    });
                }
                if let Some(choice) = chunk.choices.into_iter().next() {
                    if let Some(content) = choice.delta.content {
                        if !content.is_empty() {
                            yield StreamEvent::Delta(content);
                        }
                    }
                }
            }
            yield StreamEvent::Done { usage: final_usage };
        };

        Ok(out.boxed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_sets_fields() {
        let p = OpenAiProvider::new()
            .with_api_base("https://example.test/v1")
            .with_request_timeout(Duration::from_millis(123));
        assert_eq!(p.api_base.as_deref(), Some("https://example.test/v1"));
        assert_eq!(p.request_timeout, Duration::from_millis(123));
    }

    #[tokio::test]
    async fn timeout_returns_transient() {
        // Point at a non-routable address with a tiny timeout so the request
        // cannot resolve before we give up.
        let p = OpenAiProvider::new()
            .with_api_base("http://10.255.255.1:1/v1")
            .with_request_timeout(Duration::from_millis(20));
        let err = p
            .complete("sk-test", ChatRequest::simple("gpt-4o-mini", None, "hi"))
            .await
            .unwrap_err();
        assert_eq!(err.tag(), "transient", "expected transient, got {err:?}");
    }
}
