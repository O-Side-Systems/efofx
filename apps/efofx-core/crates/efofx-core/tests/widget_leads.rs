//! Phase 2D.6 contract tests for `POST /v1/widget/leads`.
//!
//! Auth-rejection envelope only — persistence + 201 happy path lives in
//! the infrastructure suite (Phase 4).

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

#[tokio::test]
async fn create_lead_requires_widget_api_key() {
    let harness = TestHarness::new().await;
    let body = serde_json::json!({
        "session_id": "00000000-0000-0000-0000-000000000000",
        "name": "Sam",
        "email": "sam@example.com",
        "phone": "5551234567",
    });
    let req = Request::builder()
        .method("POST")
        .uri("/v1/widget/leads")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let (status, body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(err_code(&body).is_some(), "expected envelope, got {body}");
}

#[tokio::test]
async fn create_lead_rejects_unknown_api_key() {
    let harness = TestHarness::new().await;
    let body = serde_json::json!({
        "session_id": "00000000-0000-0000-0000-000000000000",
        "name": "Sam",
        "email": "sam@example.com",
        "phone": "5551234567",
    });
    let req = Request::builder()
        .method("POST")
        .uri("/v1/widget/leads")
        .header("content-type", "application/json")
        .header("x-api-key", "sk_live_unknown_key_value")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let (status, _body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
