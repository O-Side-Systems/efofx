//! Per-tenant CORS for the widget API-key surface.
//!
//! ## Why a custom origin cache instead of `CorsLayer::allow_origin(list)`
//!
//! `tower_http`'s built-in origin lists are baked at construction time. We
//! need the allowed-origin set to *grow at runtime* as new tenants come
//! online without restarting the server. The cache is shared across all
//! widget routes; the CorsLayer's `AllowOrigin::predicate` reads from it
//! per request.
//!
//! ## Seeding
//!
//! Two paths populate the cache:
//!
//! 1. `GET /v1/widget/branding/{prefix}` — public, cheap. The
//!    [`TenantRepo::fetch_branding_by_prefix`] call already returns the
//!    tenant's `allowed_origins`, so the branding handler folds them in
//!    on a successful lookup.
//! 2. [`seed_widget_origins`] — runs *after* `widget_api_key` auth on the
//!    widget API surface. Defense-in-depth: in steady state every widget
//!    fetches branding before posting, so this seeding rarely matters,
//!    but it keeps the cache populated even if a deployment is reset
//!    while widgets are mid-flight.
//!
//! ## Cold start
//!
//! Before any branding fetch, the cache is empty and a preflight from a
//! new origin will be rejected. This is intentional: an unconfigured
//! tenant cannot accept any cross-origin write. The widget bootstraps
//! with the branding GET, which is a "simple request" (no preflight),
//! seeding the cache before any widget-API-key call needs it.
//!
//! ## Eviction
//!
//! Not implemented. A revoked origin remains allowed until the process
//! restarts. Out of scope for 2D.5; revisit alongside the
//! `PATCH /v1/me` settings endpoint.

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use axum::{
    extract::{Request, State},
    http::HeaderValue,
    middleware::Next,
    response::Response,
};
use tower_http::cors::{AllowMethods, AllowOrigin, CorsLayer};

use efofx_storage::{TenantContext, TenantRepo};

/// Process-local set of allowed origins, keyed by exact-string match
/// against the request's `Origin` header. Cheap to clone (one [`Arc`]).
#[derive(Clone, Default)]
pub struct OriginCache {
    inner: Arc<RwLock<HashSet<String>>>,
}

impl OriginCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert each origin, ignoring duplicates and empty strings. The
    /// caller does not need to deduplicate or filter beforehand.
    pub fn insert_many<I, S>(&self, origins: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut guard = self.inner.write().expect("origin cache poisoned");
        for o in origins {
            let s = o.as_ref().trim();
            if !s.is_empty() {
                guard.insert(s.to_string());
            }
        }
    }

    /// `true` when the origin (exact match, case-sensitive) is in the
    /// cache. Cheap read — RwLock contention is bounded by the predicate
    /// being called once per request.
    pub fn contains(&self, origin: &str) -> bool {
        self.inner
            .read()
            .expect("origin cache poisoned")
            .contains(origin)
    }

    /// Snapshot the current set. Test-only — production code never needs
    /// to enumerate origins.
    #[doc(hidden)]
    pub fn snapshot(&self) -> Vec<String> {
        self.inner
            .read()
            .expect("origin cache poisoned")
            .iter()
            .cloned()
            .collect()
    }
}

/// Build the CORS layer for the widget-API-key surface. The
/// `AllowOrigin::predicate` is the only way to consult a runtime-mutable
/// allowlist — fixed lists are baked in at layer construction.
///
/// Methods and headers are explicit rather than `Any` so a future move
/// to credentialed requests doesn't silently broaden the surface.
pub fn build_widget_cors_layer(cache: OriginCache) -> CorsLayer {
    let predicate =
        AllowOrigin::predicate(move |origin: &HeaderValue, _parts| match origin.to_str() {
            Ok(s) => cache.contains(s),
            Err(_) => false,
        });

    CorsLayer::new()
        .allow_origin(predicate)
        .allow_methods(AllowMethods::list([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::OPTIONS,
        ]))
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            // The widget identifies itself with `x-api-key`; without it
            // in the allowlist, the browser strips the header on
            // preflighted requests and auth would 401.
            axum::http::HeaderName::from_static("x-api-key"),
        ])
        .max_age(std::time::Duration::from_secs(600))
}

/// State carried by [`seed_widget_origins`]. Tiny struct so axum's
/// `from_fn_with_state` can ferry both repo and cache without us
/// inventing a wrapper at the call site.
#[derive(Clone)]
pub struct SeedOriginsState {
    pub tenants: TenantRepo,
    pub cache: OriginCache,
}

/// Defense-in-depth seeding: after `widget_api_key` resolves the tenant,
/// fold that tenant's allowed_origins into the cache. Skipped when the
/// request's `Origin` is already cached (the common case once a tenant
/// has had at least one branding hit), so this only pays for a Mongo
/// read on cold tenants.
///
/// Failures here never fail the request — they just leave the cache
/// unseeded for this tenant. The CORS layer's predicate already ran
/// before this middleware, so the response either already has the
/// `Access-Control-Allow-Origin` header (predicate said yes) or doesn't
/// (predicate said no). This middleware affects *future* requests only.
pub async fn seed_widget_origins(
    State(state): State<SeedOriginsState>,
    req: Request,
    next: Next,
) -> Response {
    if let Some(origin_str) = req
        .headers()
        .get(axum::http::header::ORIGIN)
        .and_then(|v| v.to_str().ok())
    {
        if !state.cache.contains(origin_str) {
            // Cache miss. Look up settings; on success, fold them in.
            if let Some(ctx) = req.extensions().get::<TenantContext>().cloned() {
                match state.tenants.fetch_allowed_origins(&ctx).await {
                    Ok(origins) => state.cache.insert_many(origins),
                    Err(err) => {
                        tracing::warn!(
                            error = %err,
                            "tenant_cors: fetch_allowed_origins failed; cache not seeded"
                        );
                    }
                }
            }
        }
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_then_contains() {
        let c = OriginCache::new();
        c.insert_many(["https://a.example", "https://b.example"]);
        assert!(c.contains("https://a.example"));
        assert!(c.contains("https://b.example"));
        assert!(!c.contains("https://other.example"));
    }

    #[test]
    fn empty_strings_ignored() {
        let c = OriginCache::new();
        c.insert_many(["", "  ", "https://ok.example"]);
        let snap = c.snapshot();
        assert_eq!(snap.len(), 1);
        assert!(c.contains("https://ok.example"));
    }

    #[test]
    fn duplicates_collapse() {
        let c = OriginCache::new();
        c.insert_many(["https://x.example", "https://x.example"]);
        assert_eq!(c.snapshot().len(), 1);
    }
}
