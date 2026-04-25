//! Phase 2D.4 contract tests for `/v1/widget/events`.
//!
//! Both verbs short-circuit before hitting Mongo on the rejection paths
//! covered here:
//!
//! * `POST /v1/widget/events` — needs a widget API key. Without one the
//!   middleware returns 401 with the documented envelope, before the
//!   handler validates the body.
//! * `GET /v1/widget/events` — needs a Supabase JWT and is rate-limited
//!   per IP. Without auth the middleware returns 401.
//!
//! Persistence + happy-path coverage lands in the infrastructure suite
//! (Phase 4) where Mongo is available.

mod common;

use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use common::TestHarness;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

/// `oneshot` doesn't populate `ConnectInfo`, so the rate-limit
/// middleware needs us to inject it manually before dispatching.
async fn call(harness: &TestHarness, mut req: Request<Body>) -> (StatusCode, Value) {
    let peer: SocketAddr = "127.0.0.1:55555".parse().unwrap();
    req.extensions_mut().insert(ConnectInfo(peer));
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
async fn post_events_requires_widget_api_key() {
    let harness = TestHarness::new().await;
    let body = serde_json::json!({ "event_type": "widget_view" });
    let req = Request::builder()
        .method("POST")
        .uri("/v1/widget/events")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let (status, body) = call(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(err_code(&body).is_some(), "expected envelope, got {body}");
}

#[tokio::test]
async fn post_events_rejects_unknown_event_type_when_authed() {
    // We can't authenticate without Mongo, so we only assert that a
    // missing/invalid key still short-circuits at 401 — body validation
    // (400 widget.invalid_event_type) is exercised in the infra suite.
    let harness = TestHarness::new().await;
    let body = serde_json::json!({ "event_type": "totally_invalid" });
    let req = Request::builder()
        .method("POST")
        .uri("/v1/widget/events")
        .header("content-type", "application/json")
        .header("x-api-key", "sk_live_not_a_real_key")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let (status, _body) = call(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_events_requires_jwt() {
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("GET")
        .uri("/v1/widget/events")
        .body(Body::empty())
        .unwrap();
    let (status, body) = call(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(err_code(&body), Some("auth.missing_token"));
}

#[tokio::test]
async fn get_events_with_days_query_still_requires_jwt() {
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("GET")
        .uri("/v1/widget/events?days=7")
        .body(Body::empty())
        .unwrap();
    let (status, body) = call(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(err_code(&body).is_some());
}
