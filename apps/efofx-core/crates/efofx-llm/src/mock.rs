//! Test double for [`LlmProvider`]. Deterministic; records every inbound
//! request so tests can assert on what the service sent.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt};

use crate::error::LlmError;
use crate::provider::LlmProvider;
use crate::types::{
    ChatRequest, ChatResponse, StreamEvent, StructuredResponse, StructuredSchema, Usage,
};

/// What the mock should return for the next call of each kind. Tests set
/// these via the builder. A missing value errors with `LlmError::Unknown`.
#[derive(Default)]
pub struct MockScript {
    pub complete: Option<Result<ChatResponse, LlmError>>,
    pub complete_structured: Option<Result<StructuredResponse, LlmError>>,
    pub stream_chunks: Option<Vec<Result<StreamEvent, LlmError>>>,
}

/// Captured inbound call. Useful for `assert_eq!(recorded.api_key, "sk-...")`.
#[derive(Debug, Clone)]
pub struct Recorded {
    pub api_key: String,
    pub request: ChatRequest,
    pub schema_name: Option<String>,
    pub kind: &'static str,
}

#[derive(Default, Clone)]
pub struct MockLlmProvider {
    inner: Arc<Mutex<MockInner>>,
}

#[derive(Default)]
struct MockInner {
    script: MockScript,
    recorded: Vec<Recorded>,
}

impl MockLlmProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue a complete() response.
    pub fn on_complete(self, resp: Result<ChatResponse, LlmError>) -> Self {
        self.inner.lock().unwrap().script.complete = Some(resp);
        self
    }

    /// Queue a complete_structured() response.
    pub fn on_complete_structured(self, resp: Result<StructuredResponse, LlmError>) -> Self {
        self.inner.lock().unwrap().script.complete_structured = Some(resp);
        self
    }

    /// Queue the items a stream() call should emit (in order). A single
    /// `StreamEvent::Done` is appended automatically if none is present.
    pub fn on_stream(self, chunks: Vec<Result<StreamEvent, LlmError>>) -> Self {
        let mut chunks = chunks;
        let has_done = chunks
            .iter()
            .any(|c| matches!(c, Ok(StreamEvent::Done { .. })));
        if !has_done {
            chunks.push(Ok(StreamEvent::Done { usage: None }));
        }
        self.inner.lock().unwrap().script.stream_chunks = Some(chunks);
        self
    }

    /// Canned successful text response with default usage.
    pub fn canned_text(self, text: impl Into<String>) -> Self {
        self.on_complete(Ok(ChatResponse {
            content: text.into(),
            usage: Usage::default(),
        }))
    }

    pub fn recorded(&self) -> Vec<Recorded> {
        self.inner.lock().unwrap().recorded.clone()
    }

    fn push_recorded(&self, r: Recorded) {
        self.inner.lock().unwrap().recorded.push(r);
    }
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn complete(&self, api_key: &str, req: ChatRequest) -> Result<ChatResponse, LlmError> {
        self.push_recorded(Recorded {
            api_key: api_key.to_string(),
            request: req,
            schema_name: None,
            kind: "complete",
        });
        self.inner
            .lock()
            .unwrap()
            .script
            .complete
            .take()
            .unwrap_or_else(|| {
                Err(LlmError::Unknown(
                    "mock: no complete response queued".into(),
                ))
            })
    }

    async fn complete_structured(
        &self,
        api_key: &str,
        req: ChatRequest,
        schema: StructuredSchema,
    ) -> Result<StructuredResponse, LlmError> {
        self.push_recorded(Recorded {
            api_key: api_key.to_string(),
            request: req,
            schema_name: Some(schema.name),
            kind: "complete_structured",
        });
        self.inner
            .lock()
            .unwrap()
            .script
            .complete_structured
            .take()
            .unwrap_or_else(|| {
                Err(LlmError::Unknown(
                    "mock: no complete_structured response queued".into(),
                ))
            })
    }

    async fn stream(
        &self,
        api_key: &str,
        req: ChatRequest,
    ) -> Result<BoxStream<'static, Result<StreamEvent, LlmError>>, LlmError> {
        self.push_recorded(Recorded {
            api_key: api_key.to_string(),
            request: req,
            schema_name: None,
            kind: "stream",
        });
        let chunks = self
            .inner
            .lock()
            .unwrap()
            .script
            .stream_chunks
            .take()
            .unwrap_or_else(|| vec![Ok(StreamEvent::Done { usage: None })]);
        Ok(stream::iter(chunks).boxed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn canned_text_is_returned() {
        let mock = MockLlmProvider::new().canned_text("hello world");
        let resp = mock
            .complete("sk-test", ChatRequest::simple("gpt-4", None, "hi"))
            .await
            .unwrap();
        assert_eq!(resp.content, "hello world");
        let recorded = mock.recorded();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].api_key, "sk-test");
        assert_eq!(recorded[0].kind, "complete");
    }

    #[tokio::test]
    async fn stream_emits_queued_chunks_then_done() {
        let mock = MockLlmProvider::new().on_stream(vec![
            Ok(StreamEvent::Delta("Hel".into())),
            Ok(StreamEvent::Delta("lo".into())),
        ]);
        let mut s = mock
            .stream("sk", ChatRequest::simple("gpt-4", None, "hi"))
            .await
            .unwrap();
        let mut seen = Vec::new();
        while let Some(ev) = s.next().await {
            seen.push(ev.unwrap());
        }
        assert_eq!(seen.len(), 3);
        matches!(seen[0], StreamEvent::Delta(ref s) if s == "Hel");
        matches!(seen[2], StreamEvent::Done { .. });
    }

    #[tokio::test]
    async fn missing_script_returns_unknown_error() {
        let mock = MockLlmProvider::new();
        let err = mock
            .complete("sk", ChatRequest::simple("gpt-4", None, "hi"))
            .await
            .unwrap_err();
        assert_eq!(err.tag(), "unknown");
    }
}
