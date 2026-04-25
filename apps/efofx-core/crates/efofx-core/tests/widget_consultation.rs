//! Phase 2D.3 contract tests for `POST /v1/widget/consultations`.
//!
//! The middleware short-circuits before the handler runs when the widget
//! API key header is missing or malformed, so these tests can exercise
//! the documented rejection envelope without a live Mongo. Persistence /
//! email-side coverage lands in the infrastructure suite (Phase 4).

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::TestHarness;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

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
async fn create_consultation_requires_widget_api_key() {
    let harness = TestHarness::new().await;
    let body = serde_json::json!({
        "session_id": "00000000-0000-0000-0000-000000000000",
        "name": "Sam",
        "email": "sam@example.com",
        "phone": "5551234567",
        "message": "Could you call me about a roof replacement?",
    });
    let req = Request::builder()
        .method("POST")
        .uri("/v1/widget/consultations")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let (status, body) = call(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // The exact code is owned by the api-key middleware; we only assert
    // it produced the documented envelope shape.
    assert!(
        err_code(&body).is_some(),
        "expected error envelope, got {body}"
    );
}

#[tokio::test]
async fn create_consultation_rejects_malformed_api_key_header() {
    let harness = TestHarness::new().await;
    let body = serde_json::json!({
        "session_id": "00000000-0000-0000-0000-000000000000",
        "name": "Sam",
        "email": "sam@example.com",
        "phone": "5551234567",
        "message": "ping",
    });
    let req = Request::builder()
        .method("POST")
        .uri("/v1/widget/consultations")
        .header("content-type", "application/json")
        .header("x-api-key", "not-a-real-key")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let (status, body) = call(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(err_code(&body).is_some());
}
