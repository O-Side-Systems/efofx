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

/// Variant of [`dispatch`] that returns the response head + body bytes
/// untouched — needed for HTML form responses where we want to assert
/// on `Content-Type` and substring presence rather than parsed JSON.
async fn dispatch_raw(
    harness: &TestHarness,
    mut req: Request<Body>,
) -> (StatusCode, String, Vec<u8>) {
    let peer: SocketAddr = "127.0.0.1:55555".parse().unwrap();
    req.extensions_mut().insert(ConnectInfo(peer));
    let resp = harness.router.clone().oneshot(req).await.expect("router");
    let status = resp.status();
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = resp
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    (status, content_type, bytes)
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

#[tokio::test]
async fn render_form_unknown_token_renders_expired_html() {
    // GET on the public form route never returns a non-200 status — the
    // page itself communicates Valid / Expired / Used / NotFound. This
    // also keeps email scanners (which preview the URL) from leaking
    // signal back to the sender.
    //
    // The fake Mongo handle in `TestHarness` errors on any query, which
    // the handler treats as "render the expired page with default
    // branding" — exactly the wire we want for an unknown token.
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("GET")
        .uri("/v1/feedback/forms/nonexistent-token-value")
        .body(Body::empty())
        .unwrap();
    let (status, content_type, bytes) = dispatch_raw(&harness, req).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        content_type.starts_with("text/html"),
        "unexpected content-type: {content_type}"
    );
    let body = String::from_utf8(bytes).expect("html body utf-8");
    assert!(
        body.contains("This link has expired"),
        "expected expired page, got: {body}"
    );
}

#[tokio::test]
async fn submit_form_rejects_missing_fields() {
    // axum's `Form<FeedbackSubmission>` extractor rejects a request
    // missing required fields with 422 before the handler runs. We rely
    // on that rather than re-validating each required field by hand.
    let harness = TestHarness::new().await;
    let req = Request::builder()
        .method("POST")
        .uri("/v1/feedback/forms/some-token")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from("rating=4")) // missing actual_cost, etc.
        .unwrap();
    let (status, _ct, _bytes) = dispatch_raw(&harness, req).await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
        "expected 4xx form rejection, got {status}"
    );
}

#[tokio::test]
async fn submit_form_rejects_bad_rating() {
    // Rating of 0 fails our explicit handler-side validator (the form
    // extractor accepts u8, so 0 is a valid u8). This proves the
    // validator runs and produces a JSON error envelope.
    let harness = TestHarness::new().await;
    let body = "rating=0&actual_cost=1000&actual_timeline_weeks=4&discrepancy_reason_primary=scope_changed";
    let req = Request::builder()
        .method("POST")
        .uri("/v1/feedback/forms/some-token")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap();
    let (status, body) = dispatch(&harness, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(err_code(&body).is_some(), "expected envelope, got {body}");
}
