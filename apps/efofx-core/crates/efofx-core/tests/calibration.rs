//! Phase 2E.5 contract tests for the calibration endpoints.
//!
//! Auth-rejection envelope + query-validation 400s. Aggregation
//! happy-paths require a real Mongo and live in the infrastructure
//! suite (Phase 4).

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
async fn metrics_requires_jwt() {
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("GET")
        .uri("/v1/calibration/metrics")
        .body(Body::empty())
        .unwrap();
    let (status, body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(err_code(&body).is_some(), "expected envelope, got {body}");
}

#[tokio::test]
async fn metrics_rejects_widget_key() {
    // Calibration is dashboard-only — widget keys never resolve here.
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("GET")
        .uri("/v1/calibration/metrics")
        .header("x-api-key", "sk_live_unknown_key_value")
        .body(Body::empty())
        .unwrap();
    let (status, _body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn metrics_rejects_unknown_jwt_kid() {
    let harness = TestHarness::new().await;
    let claims = harness.valid_claims("user-1");
    let token = harness.signer.sign_with_kid("nope", &claims);
    let req = Request::builder()
        .method("GET")
        .uri("/v1/calibration/metrics")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let (status, body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(err_code(&body).is_some(), "expected envelope, got {body}");
}

#[tokio::test]
async fn trend_requires_jwt() {
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("GET")
        .uri("/v1/calibration/trend")
        .body(Body::empty())
        .unwrap();
    let (status, body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(err_code(&body).is_some(), "expected envelope, got {body}");
}

#[tokio::test]
async fn trend_rejects_widget_key() {
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("GET")
        .uri("/v1/calibration/trend")
        .header("x-api-key", "sk_live_unknown_key_value")
        .body(Body::empty())
        .unwrap();
    let (status, _body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ----------------------------------------------------------------------
// Query-validation 400s — these need to land *before* tenant resolution
// so they fire even with the fake Mongo handle. The auth layer runs
// first, so we don't have a path to the validator that doesn't
// involve a live tenant. The contract is therefore documented here:
// when invoked authenticated, an out-of-range query parameter returns
// 400 before any DB call. Live validation runs in Phase 4.
//
// The closest thing we *can* test cheaply is that the auth layer
// itself returns 401 before query validation runs — i.e. the
// validation layer does not leak shape information to unauthenticated
// callers.
// ----------------------------------------------------------------------

#[tokio::test]
async fn metrics_validation_does_not_leak_to_unauth() {
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("GET")
        .uri("/v1/calibration/metrics?date_range=junk")
        .body(Body::empty())
        .unwrap();
    let (status, body) = dispatch(&harness, req).await;
    // Critically: 401 here, not 400. The auth layer must run before
    // the query validator; otherwise unauthenticated probes could
    // map out our schema.
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(err_code(&body).is_some(), "expected envelope, got {body}");
}

#[tokio::test]
async fn trend_validation_does_not_leak_to_unauth() {
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("GET")
        .uri("/v1/calibration/trend?months=999")
        .body(Body::empty())
        .unwrap();
    let (status, body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(err_code(&body).is_some(), "expected envelope, got {body}");
}
