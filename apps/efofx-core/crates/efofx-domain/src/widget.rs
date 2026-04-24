use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;
use utoipa::ToSchema;

use crate::ids::{SessionId, TenantId};

/// Public branding configuration. Served unauthenticated via
/// `GET /v1/widget/branding/{api_key_prefix}`.
///
/// Fields are whitelisted: only visual/UX tokens, never keys or PII.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BrandingConfig {
    pub primary_color: String,
    pub secondary_color: String,
    pub accent_color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    pub welcome_message: String,
    pub button_text: String,
    pub company_name: String,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consultation_form_labels: Option<HashMap<String, String>>,
}

fn default_locale() -> String {
    "en".into()
}

impl Default for BrandingConfig {
    fn default() -> Self {
        Self {
            primary_color: "#2563eb".into(),
            secondary_color: "#f3f4f6".into(),
            accent_color: "#1d4ed8".into(),
            logo_url: None,
            welcome_message: "Hi! Tell me about your project and I'll help estimate the cost."
                .into(),
            button_text: "Get an Estimate".into(),
            company_name: String::new(),
            locale: default_locale(),
            consultation_form_labels: None,
        }
    }
}

/// Lead lifecycle status. New in Rust — FastAPI never surfaced one.
/// Dashboard drives transitions through `PATCH /v1/leads/{id}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LeadStatus {
    New,
    Contacted,
    Converted,
    Closed,
}

impl Default for LeadStatus {
    fn default() -> Self {
        Self::New
    }
}

/// A captured widget lead. Fields echo the widget lead form plus backend
/// enrichments (tenant id, status, timestamps).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Lead {
    pub id: String,
    pub tenant_id: TenantId,
    pub session_id: SessionId,
    pub name: String,
    pub email: String,
    pub phone: String,
    #[serde(default)]
    pub status: LeadStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consultation_message: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub captured_at: OffsetDateTime,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub updated_at: Option<OffsetDateTime>,
}

/// A consultation request is a lead with a mandatory free-text message.
/// Triggers the notification email to the contractor.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConsultationRequest {
    pub session_id: SessionId,
    pub name: String,
    pub email: String,
    pub phone: String,
    /// Free-text message, 1–2000 chars.
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lead_status_snake_case() {
        assert_eq!(
            serde_json::to_string(&LeadStatus::Converted).unwrap(),
            "\"converted\""
        );
    }

    #[test]
    fn lead_status_default_is_new() {
        assert_eq!(LeadStatus::default(), LeadStatus::New);
    }
}
