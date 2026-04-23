use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;

use crate::ids::{SessionId, TenantId};

/// Chat session state-machine status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ChatStatus {
    Active,
    Ready,
    Completed,
    Expired,
}

/// Role of a message author. Matches OpenAI chat-completion conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Fields extracted from the conversation that drive the readiness check.
/// `is_ready` returns true when the four load-bearing fields are populated.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ScopingContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_size: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub special_conditions: Option<String>,
}

impl ScopingContext {
    /// True when all four load-bearing fields have values. Mirrors the
    /// FastAPI `ScopingContext.is_ready()` method.
    pub fn is_ready(&self) -> bool {
        self.project_type.is_some()
            && self.project_size.is_some()
            && self.location.is_some()
            && self.timeline.is_some()
    }

    /// Ordered list of fields still missing, for the prompt to target.
    pub fn missing_fields(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.project_type.is_none() {
            missing.push("project_type");
        }
        if self.project_size.is_none() {
            missing.push("project_size");
        }
        if self.location.is_none() {
            missing.push("location");
        }
        if self.timeline.is_none() {
            missing.push("timeline");
        }
        if self.special_conditions.is_none() {
            missing.push("special_conditions");
        }
        missing
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChatSession {
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    pub status: ChatStatus,
    #[serde(default)]
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub scoping_context: ScopingContext,
    pub is_ready: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_version: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoping_context_is_ready_requires_four_fields() {
        let mut ctx = ScopingContext::default();
        assert!(!ctx.is_ready());
        ctx.project_type = Some("pool".into());
        ctx.project_size = Some("15x30".into());
        ctx.location = Some("91234".into());
        assert!(!ctx.is_ready());
        ctx.timeline = Some("spring 2026".into());
        assert!(ctx.is_ready());
    }

    #[test]
    fn missing_fields_priority_order() {
        let ctx = ScopingContext::default();
        let missing = ctx.missing_fields();
        assert_eq!(
            missing,
            vec![
                "project_type",
                "project_size",
                "location",
                "timeline",
                "special_conditions"
            ]
        );
    }

    #[test]
    fn chat_status_wire_format() {
        let active = serde_json::to_string(&ChatStatus::Active).unwrap();
        assert_eq!(active, "\"active\"");
        let round: ChatStatus = serde_json::from_str("\"ready\"").unwrap();
        assert_eq!(round, ChatStatus::Ready);
    }
}
