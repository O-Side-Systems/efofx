//! BYOK (Bring Your Own Key) service.
//!
//! Orchestrates the validate → encrypt → store pipeline for tenant-supplied
//! OpenAI keys. Decryption happens only within request scope; plaintext
//! never leaves this service's return values.

use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use efofx_openapi::{ApiError, ErrorCode};
use efofx_storage::auth::MasterKey;
use efofx_storage::{TenantContext, TenantRepo};
use tracing::warn;

#[derive(Debug, thiserror::Error)]
pub enum ByokError {
    #[error("openai rejected the supplied key")]
    InvalidKey,
    #[error("openai validation unavailable: {0}")]
    Upstream(String),
    #[error("byok crypto failure: {0}")]
    Crypto(#[from] efofx_crypto::CryptoError),
    #[error(transparent)]
    Storage(#[from] efofx_storage::StorageError),
}

impl IntoResponse for ByokError {
    fn into_response(self) -> Response {
        match self {
            ByokError::InvalidKey => (
                StatusCode::BAD_REQUEST,
                Json(ApiError::single(
                    ErrorCode::EstimationLlmInvalidKey.as_str(),
                    "Invalid OpenAI API key",
                )),
            )
                .into_response(),
            ByokError::Upstream(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiError::single(
                    ErrorCode::EstimationLlmTransient.as_str(),
                    msg,
                )),
            )
                .into_response(),
            ByokError::Crypto(_) | ByokError::Storage(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::single(
                    "common.internal",
                    "An internal error occurred",
                )),
            )
                .into_response(),
        }
    }
}

/// BYOK orchestration. Cheap to clone — everything inside is either an
/// `Arc`, a `Clone`-y handle, or primitive.
#[derive(Clone)]
pub struct ByokService {
    tenants: TenantRepo,
    master_key: Arc<MasterKey>,
    http: reqwest::Client,
}

impl ByokService {
    pub fn new(tenants: TenantRepo, master_key: Arc<MasterKey>, http: reqwest::Client) -> Self {
        Self {
            tenants,
            master_key,
            http,
        }
    }

    /// Validate the supplied raw OpenAI key with a lightweight `GET
    /// /v1/models` call, encrypt with the tenant's derived Fernet key, and
    /// persist ciphertext + last-6.
    ///
    /// Returns the masked display form on success.
    pub async fn store(&self, ctx: &TenantContext, raw_key: &str) -> Result<String, ByokError> {
        self.validate_upstream(raw_key).await?;
        let ciphertext =
            efofx_crypto::encrypt_openai_key(self.master_key.as_bytes(), ctx.tenant_id(), raw_key)?;
        let last6 = raw_key
            .chars()
            .rev()
            .take(6)
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();
        self.tenants
            .set_openai_key(ctx, &ciphertext, &last6)
            .await?;
        Ok(efofx_crypto::mask_openai_key(raw_key))
    }

    /// Clear the stored BYOK key for this tenant. Idempotent.
    pub async fn clear(&self, ctx: &TenantContext) -> Result<(), ByokError> {
        self.tenants.clear_openai_key(ctx).await?;
        Ok(())
    }

    /// Decrypt the stored ciphertext for use inside a single request. Call
    /// only in request scope; never persist the returned plaintext.
    #[allow(dead_code)] // Wired in Phase 2C when LLM calls start flowing.
    pub async fn decrypt(&self, ctx: &TenantContext) -> Result<Option<String>, ByokError> {
        let Some(ct) = self.tenants.fetch_openai_ciphertext(ctx).await? else {
            return Ok(None);
        };
        let plain =
            efofx_crypto::decrypt_openai_key(self.master_key.as_bytes(), ctx.tenant_id(), &ct)?;
        Ok(Some(plain))
    }

    async fn validate_upstream(&self, raw_key: &str) -> Result<(), ByokError> {
        let resp = self
            .http
            .get("https://api.openai.com/v1/models")
            .bearer_auth(raw_key)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| ByokError::Upstream(e.to_string()))?;

        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(ByokError::InvalidKey);
        }
        warn!(
            status = status.as_u16(),
            "openai validation returned non-auth error; treating as transient"
        );
        Err(ByokError::Upstream(format!(
            "openai returned {}",
            status.as_u16()
        )))
    }
}
