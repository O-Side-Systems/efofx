//! Auth-middleware failure modes. Each variant maps to one
//! [`efofx_openapi::ErrorCode`] and a fixed HTTP status so callers see a
//! consistent envelope regardless of which surface rejected them.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use efofx_openapi::{ApiError, ErrorCode};

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("authorization header missing")]
    MissingToken,
    #[error("bearer prefix malformed")]
    MalformedHeader,
    #[error("token has no matching kid in JWKS")]
    UnknownKid,
    #[error("token signature or claims rejected: {0}")]
    InvalidToken(#[source] jsonwebtoken::errors::Error),
    #[error("token expired")]
    Expired,
    #[error("token audience mismatch")]
    WrongAudience,
    #[error("api key malformed")]
    ApiKeyMalformed,
    #[error("api key not found or incorrect")]
    ApiKeyInvalid,
    #[error("downstream lookup failed: {0}")]
    Storage(#[source] efofx_storage::StorageError),
    #[error("jwks fetch failed: {0}")]
    JwksUnavailable(String),
    #[error("auth dependency missing from request: {0}")]
    MissingDependency(&'static str),
}

impl AuthError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::MissingToken | Self::MalformedHeader | Self::MissingDependency(_) => {
                ErrorCode::AuthMissingToken
            }
            Self::UnknownKid | Self::InvalidToken(_) | Self::JwksUnavailable(_) => {
                ErrorCode::AuthInvalidToken
            }
            Self::Expired => ErrorCode::AuthExpiredToken,
            Self::WrongAudience => ErrorCode::AuthWrongAudience,
            Self::ApiKeyMalformed | Self::ApiKeyInvalid => ErrorCode::AuthApiKeyInvalid,
            // Storage failure during resolution is a 500 case but we surface
            // it as auth.invalid_token on the wire so we don't leak internals
            // to unauthenticated callers. The `tracing::error` log in the
            // middleware is the operator's signal.
            Self::Storage(_) => ErrorCode::AuthInvalidToken,
        }
    }

    fn status(&self) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let code = self.code();
        let status = self.status();
        (
            status,
            Json(ApiError::single(code.as_str(), code.default_message())),
        )
            .into_response()
    }
}

impl From<efofx_storage::StorageError> for AuthError {
    fn from(value: efofx_storage::StorageError) -> Self {
        Self::Storage(value)
    }
}
