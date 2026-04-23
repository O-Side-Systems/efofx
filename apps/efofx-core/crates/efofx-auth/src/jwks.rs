//! Supabase JWKS fetch and cache.
//!
//! Supabase publishes JWKS at `{url}/auth/v1/.well-known/jwks.json`. We
//! fetch at startup, spawn a refresh ticker at the configured interval, and
//! expose a keyed-by-kid lookup for the middleware.
//!
//! Concurrency: the cache is behind an `Arc<RwLock<HashMap<_>>>`. Reads are
//! contention-free in the hot path (middleware takes a read guard only to
//! clone a `DecodingKey`); the ticker takes a write guard at most once per
//! refresh interval.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use jsonwebtoken::DecodingKey;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::error::AuthError;

/// Supabase RSA JWKS entry. Supabase also publishes EC keys for some
/// deployments; we currently only support RSA. If `kty != "RSA"` the entry
/// is skipped on load and a warning is logged.
#[derive(Debug, Deserialize)]
struct JwkRsaEntry {
    kid: String,
    kty: String,
    /// RSA modulus (base64url).
    n: String,
    /// RSA public exponent (base64url).
    e: String,
    #[serde(default)]
    alg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JwksDocument {
    keys: Vec<JwkRsaEntry>,
}

/// Owns the JWKS map and knows how to refresh it.
#[derive(Clone)]
pub struct JwksCache {
    keys: Arc<RwLock<HashMap<String, DecodingKey>>>,
    jwks_url: String,
    http: reqwest::Client,
    refresh_interval: Duration,
}

impl JwksCache {
    /// Build a cache, do the first fetch synchronously, and return the
    /// loaded cache. Call [`JwksCache::spawn_refresh`] to start the
    /// background refresher.
    pub async fn bootstrap(
        supabase_url: &str,
        refresh_interval: Duration,
    ) -> Result<Self, AuthError> {
        let jwks_url = format!(
            "{}/auth/v1/.well-known/jwks.json",
            supabase_url.trim_end_matches('/')
        );
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| AuthError::JwksUnavailable(e.to_string()))?;
        let cache = Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            jwks_url,
            http,
            refresh_interval,
        };
        cache.refresh_once().await?;
        Ok(cache)
    }

    /// Spawn a tokio task that refreshes the JWKS every
    /// `refresh_interval`. Errors during refresh are logged; the cached
    /// keys from the previous successful fetch remain valid.
    pub fn spawn_refresh(&self) {
        let inner = self.clone();
        let interval = inner.refresh_interval;
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            // First tick fires immediately; skip it since bootstrap already
            // did the initial fetch.
            ticker.tick().await;
            loop {
                ticker.tick().await;
                if let Err(e) = inner.refresh_once().await {
                    warn!(error=%e, "jwks refresh failed; keeping previous keys");
                }
            }
        });
    }

    /// Get a clone of the decoding key for `kid`, if present.
    pub async fn get(&self, kid: &str) -> Option<DecodingKey> {
        self.keys.read().await.get(kid).cloned()
    }

    async fn refresh_once(&self) -> Result<(), AuthError> {
        let resp = self
            .http
            .get(&self.jwks_url)
            .send()
            .await
            .map_err(|e| AuthError::JwksUnavailable(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(AuthError::JwksUnavailable(format!(
                "jwks endpoint returned {}",
                resp.status()
            )));
        }
        let doc: JwksDocument = resp
            .json()
            .await
            .map_err(|e| AuthError::JwksUnavailable(e.to_string()))?;

        let mut new_map = HashMap::with_capacity(doc.keys.len());
        for entry in doc.keys {
            if entry.kty != "RSA" {
                warn!(kid=%entry.kid, kty=%entry.kty, "skipping non-RSA JWKS entry");
                continue;
            }
            match DecodingKey::from_rsa_components(&entry.n, &entry.e) {
                Ok(key) => {
                    new_map.insert(entry.kid, key);
                }
                Err(e) => {
                    warn!(error=%e, alg=?entry.alg, "skipping malformed JWKS entry");
                }
            }
        }

        if new_map.is_empty() {
            return Err(AuthError::JwksUnavailable(
                "jwks document contained no usable RSA keys".into(),
            ));
        }

        let count = new_map.len();
        *self.keys.write().await = new_map;
        info!(keys = count, url = %self.jwks_url, "jwks refreshed");
        Ok(())
    }
}
