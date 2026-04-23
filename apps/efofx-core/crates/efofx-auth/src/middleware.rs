//! Axum middleware for Supabase JWT and widget API-key authentication.
//!
//! Each middleware calls a blessed resolver in
//! [`efofx_storage::auth`] to mint the shared [`TenantContext`]. Both
//! middlewares insert the context as an `axum::Extension`; handlers pull it
//! out with `Extension(ctx): Extension<TenantContext>`.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use efofx_config::SupabaseConfig;
use efofx_storage::auth::{ApiKeyAuth, ProvisionInput, TenantResolver};
use jsonwebtoken::{decode, decode_header, Algorithm, Validation};

use crate::{claims::SupabaseClaims, error::AuthError, jwks::JwksCache};

/// Bundle the state an auth middleware needs. Cloneable and cheap — all
/// inner fields are `Arc` handles or small config values.
#[derive(Clone)]
pub struct AuthState {
    pub jwks: JwksCache,
    pub supabase: Arc<SupabaseConfig>,
    pub resolver: TenantResolver,
    pub api_key: ApiKeyAuth,
}

/// Require a valid Supabase JWT. On success, injects `TenantContext` via
/// `Extension` and calls `next`. On failure, returns the standard auth
/// error envelope.
pub async fn supabase_jwt(
    State(state): State<AuthState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let token = extract_bearer(&req)?;
    let claims = verify_jwt(&state, &token).await?;
    let input = ProvisionInput {
        supabase_user_id: claims.sub,
        email: claims.email,
        company_name: claims.user_metadata.company_name,
    };
    let ctx = state.resolver.resolve_or_provision(input).await?;
    req.extensions_mut().insert(ctx);
    Ok(next.run(req).await)
}

/// Require a valid widget API key. On success, injects `TenantContext` via
/// `Extension` and calls `next`. On failure, returns the standard auth
/// error envelope.
pub async fn widget_api_key(
    State(state): State<AuthState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let raw = extract_api_key(&req)?;
    let ctx = state
        .resolver
        .resolve_by_api_key(&raw, &state.api_key)
        .await?;
    req.extensions_mut().insert(ctx);
    Ok(next.run(req).await)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn extract_bearer(req: &Request) -> Result<String, AuthError> {
    let header = req
        .headers()
        .get(header::AUTHORIZATION)
        .ok_or(AuthError::MissingToken)?
        .to_str()
        .map_err(|_| AuthError::MalformedHeader)?;
    let rest = header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .ok_or(AuthError::MalformedHeader)?;
    if rest.is_empty() {
        return Err(AuthError::MalformedHeader);
    }
    Ok(rest.to_owned())
}

fn extract_api_key(req: &Request) -> Result<String, AuthError> {
    // Prefer Authorization: Bearer sk_live_… when present; fall back to
    // X-Api-Key. The widget historically uses X-Api-Key; SDK callers often
    // use Bearer. Accept both.
    if let Some(h) = req.headers().get(header::AUTHORIZATION) {
        let s = h.to_str().map_err(|_| AuthError::MalformedHeader)?;
        if let Some(rest) = s
            .strip_prefix("Bearer ")
            .or_else(|| s.strip_prefix("bearer "))
        {
            if rest.starts_with("sk_live_") {
                return Ok(rest.to_owned());
            }
        }
    }
    if let Some(h) = req.headers().get("x-api-key") {
        let s = h.to_str().map_err(|_| AuthError::MalformedHeader)?;
        if !s.is_empty() {
            return Ok(s.to_owned());
        }
    }
    Err(AuthError::MissingToken)
}

async fn verify_jwt(state: &AuthState, token: &str) -> Result<SupabaseClaims, AuthError> {
    let header = decode_header(token).map_err(|_| AuthError::MalformedHeader)?;
    let kid = header.kid.ok_or(AuthError::MalformedHeader)?;
    let key = state.jwks.get(&kid).await.ok_or(AuthError::UnknownKid)?;

    // Supabase signs with RS256. Other algorithms are rejected so a
    // compromised upstream can't downgrade us to HS256.
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[state.supabase.audience.as_str()]);
    validation.set_issuer(&[state.supabase.issuer.as_str()]);
    validation.validate_exp = true;
    validation.validate_nbf = true;

    let token_data = decode::<SupabaseClaims>(token, &key, &validation).map_err(|e| {
        use jsonwebtoken::errors::ErrorKind;
        match e.kind() {
            ErrorKind::ExpiredSignature => AuthError::Expired,
            ErrorKind::InvalidAudience => AuthError::WrongAudience,
            _ => AuthError::InvalidToken(e),
        }
    })?;
    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;

    fn req_with_auth(h: &str) -> Request {
        HttpRequest::builder()
            .uri("/v1/me")
            .header(header::AUTHORIZATION, h)
            .body(Body::empty())
            .unwrap()
    }

    #[test]
    fn extract_bearer_rejects_missing_and_malformed() {
        // No header.
        let r: Request = HttpRequest::builder().uri("/").body(Body::empty()).unwrap();
        assert!(matches!(extract_bearer(&r), Err(AuthError::MissingToken)));

        // Wrong scheme.
        let r = req_with_auth("Basic abc");
        assert!(matches!(
            extract_bearer(&r),
            Err(AuthError::MalformedHeader)
        ));

        // Empty bearer.
        let r = req_with_auth("Bearer ");
        assert!(matches!(
            extract_bearer(&r),
            Err(AuthError::MalformedHeader)
        ));

        // OK — case-insensitive prefix.
        let r = req_with_auth("bearer abc.def.ghi");
        assert_eq!(extract_bearer(&r).unwrap(), "abc.def.ghi");
    }

    #[test]
    fn extract_api_key_accepts_both_headers() {
        let r = req_with_auth("Bearer sk_live_xxx");
        assert_eq!(extract_api_key(&r).unwrap(), "sk_live_xxx");

        let r: Request = HttpRequest::builder()
            .uri("/")
            .header("x-api-key", "sk_live_yyy")
            .body(Body::empty())
            .unwrap();
        assert_eq!(extract_api_key(&r).unwrap(), "sk_live_yyy");

        // JWT-shaped bearer is not a widget API key.
        let r = req_with_auth("Bearer eyJhbGciOiJIUzI1NiJ9.xxx.yyy");
        assert!(matches!(extract_api_key(&r), Err(AuthError::MissingToken)));
    }
}
