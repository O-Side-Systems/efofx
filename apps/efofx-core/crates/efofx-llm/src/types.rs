//! Crate-native request/response types. These intentionally do not expose
//! `async-openai` concrete types, so the [`LlmProvider`] trait remains a
//! stable seam for mocking and provider swaps.

use serde::{Deserialize, Serialize};

/// Role of a message author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// One message in a chat transcript.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

/// Request body for a chat-style completion.
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    /// 0.0–2.0; `None` lets the provider apply its default.
    pub temperature: Option<f32>,
    /// Upper bound on completion tokens.
    pub max_tokens: Option<u32>,
}

impl ChatRequest {
    /// Build a simple request with an optional system prompt and one user message.
    pub fn simple(
        model: impl Into<String>,
        system_prompt: Option<&str>,
        user_message: impl Into<String>,
    ) -> Self {
        let mut messages = Vec::with_capacity(2);
        if let Some(sys) = system_prompt {
            messages.push(ChatMessage::system(sys));
        }
        messages.push(ChatMessage::user(user_message));
        Self {
            model: model.into(),
            messages,
            temperature: None,
            max_tokens: None,
        }
    }

    pub fn with_temperature(mut self, t: f32) -> Self {
        self.temperature = Some(t);
        self
    }

    pub fn with_max_tokens(mut self, n: u32) -> Self {
        self.max_tokens = Some(n);
        self
    }
}

/// OpenAI-style usage record attached to every completion.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Completion result for a non-streaming text call.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: String,
    pub usage: Usage,
}

/// JSON-schema description for a structured-output call. The `name` is the
/// identifier OpenAI uses for the schema (<=64 chars, `[a-zA-Z0-9_-]`).
#[derive(Debug, Clone)]
pub struct StructuredSchema {
    pub name: String,
    pub description: Option<String>,
    pub schema: serde_json::Value,
}

/// Result of a structured-output call. `data` is the parsed JSON, not yet
/// typed — callers deserialize to their own struct.
#[derive(Debug, Clone)]
pub struct StructuredResponse {
    pub data: serde_json::Value,
    pub usage: Usage,
}

/// One event from a streaming completion. Role-only and finish-only chunks
/// are filtered upstream so downstream code only sees content.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// A chunk of assistant text.
    Delta(String),
    /// Final event; `usage` may be present if the provider reported it.
    Done { usage: Option<Usage> },
}
