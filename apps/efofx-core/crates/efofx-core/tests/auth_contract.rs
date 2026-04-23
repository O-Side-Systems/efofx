//! Phase 2A auth contract tests.
//!
//! Every rejection mode (missing / malformed / expired / wrong-aud /
//! wrong-iss / unknown-kid) is asserted to return the documented 401
//! envelope with the correct stable error code. These tests short-circuit
//! before the tenant resolver runs, so they don't need a live Mongo.
//!
//! Happy-path tenant provisioning coverage is deferred to an
//! infrastructure-backed suite (requires a test Mongo); the JWT
//! verification stack is fully exercised here.

mod common;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use common::TestHarness;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

/// Issue a GET against the harness's router and return (status, body_json).
async fn call(harness: &TestHarness, req: Request<Body>) -> (StatusCode, Value) {
    let resp = harness
        .router
        .clone()
        .oneshot(req)
        .await
        .expect("router responds");
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, body)
}

fn err_code(body: &Value) -> Option<&str> {
    body.get("errors")?.get(0)?.get("code")?.as_str()
}

#[tokio::test]
async fn health_endpoint_remains_unauthenticated() {
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let (status, _) = call(&harness, req).await;
    // Health probe will try to ping the fake Mongo handle; we only assert
    // it responds with *some* status, since the ping path isn't under test.
    assert!(
        status.is_success() || status.is_server_error(),
        "health returned unexpected status {status}"
    );
}

#[tokio::test]
async fn get_me_requires_auth() {
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("GET")
        .uri("/v1/me")
        .body(Body::empty())
        .unwrap();
    let (status, body) = call(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(err_code(&body), Some("auth.missing_token"));
}

#[tokio::test]
async fn get_me_rejects_malformed_bearer() {
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("GET")
        .uri("/v1/me")
        .header(header::AUTHORIZATION, "Basic abc")
        .body(Body::empty())
        .unwrap();
    let (status, body) = call(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(err_code(&body), Some("auth.missing_token"));
}

#[tokio::test]
async fn get_me_rejects_jwt_with_unknown_kid() {
    let harness = TestHarness::new().await;
    let claims = harness.valid_claims("user-unknown-kid");
    let token = harness.signer.sign_with_kid("stranger", &claims);
    let req = Request::builder()
        .method("GET")
        .uri("/v1/me")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let (status, body) = call(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(err_code(&body), Some("auth.invalid_token"));
}

#[tokio::test]
async fn get_me_rejects_expired_jwt() {
    let harness = TestHarness::new().await;
    let mut claims = harness.valid_claims("user-expired");
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    // Default jsonwebtoken leeway is 60s; push well past it so the
    // expiry check actually fires.
    claims["exp"] = serde_json::json!(now - 600);
    claims["iat"] = serde_json::json!(now - 3600);
    let token = harness.signer.sign(&claims);
    let req = Request::builder()
        .method("GET")
        .uri("/v1/me")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let (status, body) = call(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(err_code(&body), Some("auth.expired_token"));
}

#[tokio::test]
async fn get_me_rejects_wrong_audience() {
    let harness = TestHarness::new().await;
    let mut claims = harness.valid_claims("user-wrong-aud");
    claims["aud"] = serde_json::json!("not-authenticated");
    let token = harness.signer.sign(&claims);
    let req = Request::builder()
        .method("GET")
        .uri("/v1/me")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let (status, body) = call(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(err_code(&body), Some("auth.wrong_audience"));
}

#[tokio::test]
async fn get_me_rejects_wrong_issuer() {
    let harness = TestHarness::new().await;
    let mut claims = harness.valid_claims("user-wrong-iss");
    claims["iss"] = serde_json::json!("https://impostor.example.com/auth/v1");
    let token = harness.signer.sign(&claims);
    let req = Request::builder()
        .method("GET")
        .uri("/v1/me")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let (status, body) = call(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(err_code(&body), Some("auth.invalid_token"));
}

#[tokio::test]
async fn widget_api_key_rejects_missing_key() {
    let harness = TestHarness::new().await;
    // GET /v1/me accepts either; miss on both should still 401.
    let req = Request::builder()
        .method("GET")
        .uri("/v1/me/openai-key/status")
        .body(Body::empty())
        .unwrap();
    let (status, body) = call(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(err_code(&body), Some("auth.missing_token"));
}

#[tokio::test]
async fn widget_api_key_rejects_malformed_key() {
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("GET")
        .uri("/v1/me/openai-key/status")
        .header("x-api-key", "not-a-real-key")
        .body(Body::empty())
        .unwrap();
    let (status, body) = call(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(err_code(&body), Some("auth.api_key_invalid"));
}

#[tokio::test]
async fn patch_me_requires_supabase_jwt_not_api_key() {
    // PATCH /v1/me is JWT-only; a widget key must not authorize a write.
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("PATCH")
        .uri("/v1/me")
        .header("x-api-key", "sk_live_dead-beef-dead-beef-dead-beef-00_abc")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"company_name":"X"}"#))
        .unwrap();
    let (status, _) = call(&harness, req).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "PATCH must not accept widget API keys"
    );
}
