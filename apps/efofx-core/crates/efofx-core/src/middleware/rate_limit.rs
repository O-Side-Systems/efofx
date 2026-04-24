//! IP-keyed request throttling.
//!
//! The limiter is process-local and backed by [`governor`]. That is good
//! enough for the single-instance deployment Jeff's demo runs on; the
//! RUST-PORT-PLAN Open Questions call for a Valkey-backed impl in
//! Phase 4. This module keeps the concrete type small and private so a
//! shared-state backend can replace it without churning handlers.
//!
//! Keying:
//! * Per-route-group (branding, analytics-read, …) buckets live on
//!   separate [`IpRateLimiter`] instances — callers mount one per
//!   protected route group.
//! * Within a group, the key is the client IP, extracted from the
//!   `x-forwarded-for` leftmost entry when present, else from the
//!   `axum::extract::ConnectInfo<SocketAddr>` socket peer, else the
//!   string `"unknown"` (shared bucket — a safe fail-closed default on
//!   unproxied localhost traffic).

use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};

use efofx_openapi::{ApiError, ErrorCode};

/// Process-local, IP-keyed rate limiter for a single route group. Cheap
/// to clone; all state sits behind one [`Arc`].
#[derive(Clone)]
pub struct IpRateLimiter {
    inner: Arc<KeyedGovernor>,
}

/// Concrete `governor` type alias — keeps the cloneable [`IpRateLimiter`]
/// free of path-heavy generics.
type KeyedGovernor = governor::RateLimiter<
    String,
    governor::state::keyed::DefaultKeyedStateStore<String>,
    DefaultClock,
>;

impl IpRateLimiter {
    /// Create a limiter with a per-minute quota. Panics on `rpm == 0`
    /// because a zero-rate limiter can never admit a request —
    /// configuration must refuse that value, not silently deny every
    /// caller.
    pub fn per_minute(rpm: u32) -> Self {
        let quota = Quota::per_minute(NonZeroU32::new(rpm).expect("rate limit must be non-zero"));
        let inner: KeyedGovernor = RateLimiter::keyed(quota);
        Self {
            inner: Arc::new(inner),
        }
    }

    /// `true` when the key is within quota (and the limiter decrements a
    /// token on its behalf); `false` when the key is throttled.
    pub fn check(&self, key: &str) -> bool {
        self.inner.check_key(&key.to_string()).is_ok()
    }
}

// A `NotKeyed` single-bucket variant isn't needed yet — every widget
// route throttles per IP. The type below is kept dormant so the
// upgrade path is obvious when we do want it.
#[allow(dead_code)]
type SingleGovernor = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Axum middleware that consumes one token per request for the caller's
/// IP and returns `429` with the standard error envelope when the
/// bucket is empty.
pub async fn rate_limit(
    State(limiter): State<IpRateLimiter>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let key = extract_client_ip(&req, peer);
    if limiter.check(&key) {
        next.run(req).await
    } else {
        throttled_response()
    }
}

/// First leftmost `x-forwarded-for` entry, else the socket peer IP
/// (IP only, no port — two requests from the same host but different
/// ephemeral ports share a bucket), else `"unknown"`. The `unknown`
/// fallback keeps the limiter closed on misconfigured deployments.
fn extract_client_ip(req: &Request, peer: SocketAddr) -> String {
    if let Some(xff) = req.headers().get("x-forwarded-for") {
        if let Ok(s) = xff.to_str() {
            if let Some(first) = s.split(',').next() {
                let trimmed = first.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }
    peer.ip().to_string()
}

fn throttled_response() -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(ApiError::single(
            ErrorCode::RateLimitExceeded.as_str(),
            ErrorCode::RateLimitExceeded.default_message(),
        )),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_key_is_throttled_after_quota() {
        let rl = IpRateLimiter::per_minute(2);
        assert!(rl.check("1.2.3.4"));
        assert!(rl.check("1.2.3.4"));
        assert!(!rl.check("1.2.3.4"));
    }

    #[test]
    fn distinct_keys_have_distinct_buckets() {
        let rl = IpRateLimiter::per_minute(1);
        assert!(rl.check("1.2.3.4"));
        assert!(!rl.check("1.2.3.4"));
        assert!(rl.check("5.6.7.8"));
    }

    #[test]
    fn extract_uses_xff_first_entry() {
        let req = axum::http::Request::builder()
            .header("x-forwarded-for", "203.0.113.7, 10.0.0.1")
            .body(axum::body::Body::empty())
            .unwrap();
        let peer: SocketAddr = "127.0.0.1:80".parse().unwrap();
        assert_eq!(extract_client_ip(&req, peer), "203.0.113.7");
    }

    #[test]
    fn extract_falls_back_to_peer() {
        let req = axum::http::Request::builder()
            .body(axum::body::Body::empty())
            .unwrap();
        let peer: SocketAddr = "10.0.0.5:9999".parse().unwrap();
        assert_eq!(extract_client_ip(&req, peer), "10.0.0.5");
    }

    #[test]
    #[should_panic(expected = "rate limit must be non-zero")]
    fn zero_rpm_panics() {
        let _ = IpRateLimiter::per_minute(0);
    }
}
