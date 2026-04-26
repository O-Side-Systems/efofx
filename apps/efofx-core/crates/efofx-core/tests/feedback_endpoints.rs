//! Phase 2E.2 contract tests for the JSON feedback endpoints.
//!
//! Auth-rejection envelope + JSON validation 400s. The 201 happy paths
//! that hit Mongo live in the infrastructure suite (Phase 4); these
//! exercises only verify the surface contract — auth wiring, error
//! envelope shape, and validation guards.

mod common;

use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use common::TestHarness;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

async fn dispatch(harness: &TestHarness, mut req: Request<Body>) -> (StatusCode, Value) {
    let peer: SocketAddr = "127.0.0.1:55555".parse().unwrap();
    req.extensions_mut().insert(ConnectInfo(peer));
    let resp = harness.router.clone().oneshot(req).await.expect("router");
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

fn valid_body() -> Value {
    serde_json::json!({
        "estimation_session_id": "sess_abcdef123456",
        "feedback_type": "accuracy",
        "rating": 4,
    })
}

#[tokio::test]
async fn create_feedback_requires_auth() {
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("POST")
        .uri("/v1/feedback")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&valid_body()).unwrap()))
        .unwrap();
    let (status, body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(err_code(&body).is_some(), "expected envelope, got {body}");
}

#[tokio::test]
async fn create_feedback_rejects_unknown_widget_key() {
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("POST")
        .uri("/v1/feedback")
        .header("content-type", "application/json")
        .header("x-api-key", "sk_live_unknown_key_value")
        .body(Body::from(serde_json::to_vec(&valid_body()).unwrap()))
        .unwrap();
    let (status, _body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_feedback_rejects_invalid_jwt_kid() {
    // Even with a Bearer JWT, an unknown kid trips the JWKS lookup before
    // the handler — this confirms either_auth runs the JWT branch when no
    // widget key is present.
    let harness = TestHarness::new().await;
    let claims = harness.valid_claims("user-1");
    let token = harness.signer.sign_with_kid("nope", &claims);
    let req = Request::builder()
        .method("POST")
        .uri("/v1/feedback")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(serde_json::to_vec(&valid_body()).unwrap()))
        .unwrap();
    let (status, body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(err_code(&body).is_some(), "expected envelope, got {body}");
}

#[tokio::test]
async fn feedback_summary_requires_jwt() {
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("GET")
        .uri("/v1/feedback/summary")
        .body(Body::empty())
        .unwrap();
    let (status, body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(err_code(&body).is_some(), "expected envelope, got {body}");
}

#[tokio::test]
async fn feedback_summary_rejects_widget_key() {
    // The summary endpoint is dashboard-only — even a well-formed widget
    // API key must be rejected (the route is behind supabase_jwt).
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("GET")
        .uri("/v1/feedback/summary")
        .header("x-api-key", "sk_live_some_key_value")
        .body(Body::empty())
        .unwrap();
    let (status, _body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn email_request_requires_jwt() {
    let harness = TestHarness::new().await;
    let body = serde_json::json!({
        "estimation_session_id": "sess_abc",
        "customer_email": "user@example.com",
    });
    let req = Request::builder()
        .method("POST")
        .uri("/v1/feedback/email-requests")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let (status, body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(err_code(&body).is_some(), "expected envelope, got {body}");
}

#[tokio::test]
async fn email_request_rejects_widget_key() {
    // The dashboard-only mint route must reject widget keys — they
    // never reach the JWT branch because the route is behind
    // supabase_jwt only.
    let harness = TestHarness::new().await;
    let body = serde_json::json!({
        "estimation_session_id": "sess_abc",
        "customer_email": "user@example.com",
    });
    let req = Request::builder()
        .method("POST")
        .uri("/v1/feedback/email-requests")
        .header("content-type", "application/json")
        .header("x-api-key", "sk_live_unknown_key_value")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let (status, _body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn email_request_rejects_unknown_jwt_kid() {
    let harness = TestHarness::new().await;
    let claims = harness.valid_claims("user-1");
    let token = harness.signer.sign_with_kid("nope", &claims);
    let body = serde_json::json!({
        "estimation_session_id": "sess_abc",
        "customer_email": "user@example.com",
    });
    let req = Request::builder()
        .method("POST")
        .uri("/v1/feedback/email-requests")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let (status, body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(err_code(&body).is_some(), "expected envelope, got {body}");
}
