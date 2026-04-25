//! Email-sending abstraction.
//!
//! Two implementations live behind [`EmailSender`]:
//!
//! * [`ResendSender`] — calls the Resend HTTP API. Used in production.
//! * [`NoopSender`] — logs the attempt and returns success. Used in tests
//!   and any deployment where `email.resend_api_key` is absent. Mirrors
//!   FastAPI's "skip if MAIL_* unset" behaviour.
//!
//! The trait is intentionally minimal — a single fire-and-forget
//! `send_text` call. The consultation handler treats every error as
//! non-fatal: the lead is already persisted by the time we get here.

use async_trait::async_trait;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("email transport failed: {0}")]
    Transport(String),
    #[error("email rejected by provider: status={status} body={body}")]
    Rejected { status: u16, body: String },
}

/// Plain-text email payload. Subject and body are short ASCII; a future
/// HTML/template variant will sit beside this rather than replacing it.
#[derive(Debug, Clone)]
pub struct EmailMessage<'a> {
    pub from: &'a str,
    pub to: &'a str,
    pub subject: &'a str,
    pub body: &'a str,
}

#[async_trait]
pub trait EmailSender: Send + Sync + 'static {
    async fn send_text(&self, msg: EmailMessage<'_>) -> Result<(), EmailError>;
}

/// No-op sender. Logs the recipient + subject and returns Ok. Used when
/// `email.resend_api_key` is absent so we can still wire `AppState`
/// without taking a hard dependency on a live Resend account in dev.
#[derive(Debug, Clone, Default)]
pub struct NoopSender;

#[async_trait]
impl EmailSender for NoopSender {
    async fn send_text(&self, msg: EmailMessage<'_>) -> Result<(), EmailError> {
        tracing::warn!(
            to = %msg.to,
            subject = %msg.subject,
            "email not configured (NoopSender) — skipping send"
        );
        Ok(())
    }
}

/// Resend HTTP API client. POSTs to `https://api.resend.com/emails` with
/// a Bearer-token API key. Treat the API key as secret — never log it.
#[derive(Clone)]
pub struct ResendSender {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl ResendSender {
    /// Build a sender pointing at the public Resend endpoint.
    pub fn new(http: reqwest::Client, api_key: String) -> Self {
        Self {
            http,
            api_key,
            base_url: "https://api.resend.com".to_string(),
        }
    }

    /// Override the base URL — used by the wiremock test only.
    #[doc(hidden)]
    pub fn with_base_url(mut self, base: impl Into<String>) -> Self {
        self.base_url = base.into();
        self
    }
}

#[derive(Serialize)]
struct ResendBody<'a> {
    from: &'a str,
    to: [&'a str; 1],
    subject: &'a str,
    text: &'a str,
}

#[async_trait]
impl EmailSender for ResendSender {
    async fn send_text(&self, msg: EmailMessage<'_>) -> Result<(), EmailError> {
        let url = format!("{}/emails", self.base_url);
        let body = ResendBody {
            from: msg.from,
            to: [msg.to],
            subject: msg.subject,
            text: msg.body,
        };
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| EmailError::Transport(e.to_string()))?;

        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        let body = resp.text().await.unwrap_or_default();
        Err(EmailError::Rejected {
            status: status.as_u16(),
            body,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn noop_returns_ok() {
        let s = NoopSender;
        let res = s
            .send_text(EmailMessage {
                from: "a@b.com",
                to: "c@d.com",
                subject: "hi",
                body: "ok",
            })
            .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn resend_posts_authenticated_request() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/emails"))
            .and(header("authorization", "Bearer secret-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "abc"
            })))
            .mount(&server)
            .await;

        let http = reqwest::Client::new();
        let sender = ResendSender::new(http, "secret-key".into()).with_base_url(server.uri());
        let res = sender
            .send_text(EmailMessage {
                from: "noreply@efofx.test",
                to: "tenant@example.com",
                subject: "hi",
                body: "ok",
            })
            .await;
        assert!(res.is_ok(), "{res:?}");
    }

    #[tokio::test]
    async fn resend_surfaces_rejection() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/emails"))
            .respond_with(ResponseTemplate::new(422).set_body_string("invalid"))
            .mount(&server)
            .await;

        let http = reqwest::Client::new();
        let sender = ResendSender::new(http, "k".into()).with_base_url(server.uri());
        let err = sender
            .send_text(EmailMessage {
                from: "a@b",
                to: "c@d",
                subject: "x",
                body: "y",
            })
            .await
            .unwrap_err();
        assert!(matches!(err, EmailError::Rejected { status: 422, .. }));
    }
}
