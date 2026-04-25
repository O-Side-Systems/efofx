//! Phase 2D.5 contract tests for the per-tenant widget CORS layer.
//!
//! Each test pre-seeds [`TestHarness::origin_cache`] to bypass the
//! branding-fetch staging round-trip. The browser-visible contract is:
//!
//! * preflight from a cached origin → 200 with the matching
//!   `Access-Control-Allow-Origin` reflected
//! * preflight from an uncached origin → no allow-origin header (the
//!   browser then fails the request); the underlying status is 200, since
//!   `tower_http::CorsLayer` answers preflight unconditionally and only
//!   the *header* is gated by the predicate
//! * actual GET from a cached origin → handler runs and the response
//!   carries `Access-Control-Allow-Origin`
//! * actual GET from an uncached origin → handler runs but the response
//!   carries no allow-origin header

mod common;

use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderValue, Method, Request, StatusCode};
use common::TestHarness;
use tower::ServiceExt;

const ALLOWED: &str = "https://allowed.example";
const FORBIDDEN: &str = "https://forbidden.example";

async fn dispatch(harness: &TestHarness, mut req: Request<Body>) -> axum::http::Response<Body> {
    let peer: SocketAddr = "127.0.0.1:55555".parse().unwrap();
    req.extensions_mut().insert(ConnectInfo(peer));
    harness.router.clone().oneshot(req).await.expect("router")
}

#[tokio::test]
async fn preflight_allowed_when_origin_seeded() {
    let harness = TestHarness::new().await;
    harness.origin_cache.insert_many([ALLOWED]);

    let req = Request::builder()
        .method(Method::OPTIONS)
        .uri("/v1/widget/leads")
        .header("origin", ALLOWED)
        .header("access-control-request-method", "POST")
        .header("access-control-request-headers", "content-type,x-api-key")
        .body(Body::empty())
        .unwrap();

    let resp = dispatch(&harness, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("access-control-allow-origin"),
        Some(&HeaderValue::from_static(ALLOWED)),
    );
}

#[tokio::test]
async fn preflight_origin_not_seeded_omits_allow_header() {
    let harness = TestHarness::new().await;
    // Cache empty on purpose — the predicate should refuse this origin.

    let req = Request::builder()
        .method(Method::OPTIONS)
        .uri("/v1/widget/leads")
        .header("origin", FORBIDDEN)
        .header("access-control-request-method", "POST")
        .header("access-control-request-headers", "content-type,x-api-key")
        .body(Body::empty())
        .unwrap();

    let resp = dispatch(&harness, req).await;
    // tower_http always answers preflight; the contract is that no
    // allow-origin header is sent, so the browser will fail the request.
    assert!(resp.headers().get("access-control-allow-origin").is_none());
}

#[tokio::test]
async fn actual_request_seeded_origin_reflects_header() {
    let harness = TestHarness::new().await;
    harness.origin_cache.insert_many([ALLOWED]);

    // No API key — auth will 401, but the CORS layer still attaches the
    // allow-origin header on the response since the predicate matched.
    let req = Request::builder()
        .method(Method::POST)
        .uri("/v1/widget/leads")
        .header("origin", ALLOWED)
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let resp = dispatch(&harness, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        resp.headers().get("access-control-allow-origin"),
        Some(&HeaderValue::from_static(ALLOWED)),
    );
}

#[tokio::test]
async fn actual_request_unseeded_origin_omits_allow_header() {
    let harness = TestHarness::new().await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/v1/widget/leads")
        .header("origin", FORBIDDEN)
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let resp = dispatch(&harness, req).await;
    // Auth still rejects (no API key), but the CORS contract for this
    // test is that the response carries no allow-origin header — the
    // browser drops the response before the app sees it.
    assert!(resp.headers().get("access-control-allow-origin").is_none());
}
