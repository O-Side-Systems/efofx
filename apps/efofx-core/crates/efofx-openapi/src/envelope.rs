use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Consistent sync response envelope. Every non-streaming success responds
/// with this shape; streams use typed SSE events instead.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResponseMeta>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            data,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ResponseMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Error body returned for every non-2xx response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    pub errors: Vec<ApiErrorDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiErrorDetail {
    /// Stable machine-readable code, e.g. `auth.invalid_token`.
    pub code: String,
    /// Human-readable message safe to display to end users.
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl ApiError {
    pub fn single(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            errors: vec![ApiErrorDetail {
                code: code.into(),
                message: message.into(),
                meta: None,
            }],
        }
    }
}
