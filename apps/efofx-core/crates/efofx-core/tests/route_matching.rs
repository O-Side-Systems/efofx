//! Route-matching regression tests.
//!
//! axum 0.7 (matchit 0.7) only understands `:param` placeholders; the
//! `{param}` syntax from axum 0.8 registers as a *literal* segment, so a
//! route written that way silently never matches real traffic. The chat
//! and leads routers shipped with `{param}` paths during the port — these
//! tests pin every parameterised route to "matches" by asserting an
//! unauthenticated request reaches the auth layer (401) or handler
//! instead of falling through to the router's 404 fallback.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::TestHarness;
use tower::ServiceExt;

const SESSION_ID: &str = "0f8fad5b-d9cb-469f-a165-70867728950e";

async fn status_of(harness: &TestHarness, method: &str, uri: &str) -> StatusCode {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    harness
        .router
        .clone()
        .oneshot(req)
        .await
        .expect("router responds")
        .status()
}

#[tokio::test]
async fn chat_get_session_route_matches() {
    let harness = TestHarness::new().await;
    let uri = format!("/v1/chat/sessions/{SESSION_ID}");
    let status = status_of(&harness, "GET", &uri).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "route must match (401), not 404"
    );
}

#[tokio::test]
async fn chat_append_message_route_matches() {
    let harness = TestHarness::new().await;
    let uri = format!("/v1/chat/sessions/{SESSION_ID}/messages");
    let status = status_of(&harness, "POST", &uri).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "route must match (401), not 404"
    );
}

#[tokio::test]
async fn chat_generate_estimate_route_matches() {
    let harness = TestHarness::new().await;
    let uri = format!("/v1/chat/sessions/{SESSION_ID}:generate-estimate");
    let status = status_of(&harness, "POST", &uri).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "route must match (401), not 404"
    );
}

#[tokio::test]
async fn estimates_route_matches() {
    let harness = TestHarness::new().await;
    let uri = format!("/v1/estimates/{SESSION_ID}");
    let status = status_of(&harness, "GET", &uri).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "route must match (401), not 404"
    );
}

#[tokio::test]
async fn leads_item_route_matches() {
    let harness = TestHarness::new().await;
    let uri = format!("/v1/leads/{SESSION_ID}");
    let status = status_of(&harness, "GET", &uri).await;
    // Leads are Phase-4 stubs with no auth layer yet; reaching the 501
    // handler proves the parameterised path matches.
    assert_eq!(
        status,
        StatusCode::NOT_IMPLEMENTED,
        "route must match (501), not 404"
    );
}

#[tokio::test]
async fn unknown_route_still_falls_through_to_404() {
    let harness = TestHarness::new().await;
    let status = status_of(&harness, "GET", "/v1/chat/definitely-not-a-route").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
