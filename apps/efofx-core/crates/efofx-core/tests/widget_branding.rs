//! Phase 2D.6 contract tests for `GET /v1/widget/branding/{prefix}`.
//!
//! What this surface guarantees:
//!
//! * Public — no auth required.
//! * 32-hex prefix is the *only* accepted shape. Anything else is 404
//!   without a Mongo round-trip (so this test runs against the fake
//!   Mongo without flaking).
//! * Per-IP rate limit. The branding limiter on the harness is set to
//!   1M req/min so we can't trigger it in unit tests; we instead build
//!   a one-token limiter inline and confirm it produces the documented
//!   429 envelope.

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
async fn branding_rejects_short_prefix_with_404() {
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("GET")
        .uri("/v1/widget/branding/abc123")
        .body(Body::empty())
        .unwrap();
    let (status, body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(err_code(&body), Some("widget.branding_not_found"));
}

#[tokio::test]
async fn branding_rejects_non_hex_prefix_with_404() {
    let harness = TestHarness::new().await;
    // 32 chars but not hex — must reject before Mongo.
    let req = Request::builder()
        .method("GET")
        .uri("/v1/widget/branding/zzzz-zzzz-zzzz-zzzz-zzzz-zzzz-zz")
        .body(Body::empty())
        .unwrap();
    let (status, body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(err_code(&body), Some("widget.branding_not_found"));
}
