//! A tiny key-value cache abstraction with TTL semantics.
//!
//! # Why a trait
//!
//! Two call sites need identical behavior but different backends:
//!
//! - **RCF match cache** — in-process, 5-minute TTL, keyed by
//!   `{description}:{category}:{region}:{tenant}`. Cheap hits dominate; a
//!   shared backend would add latency.
//! - **Estimation response cache** — deployment-wide, keyed by prompt hash,
//!   so multiple server instances reuse expensive LLM calls. This path
//!   wants Valkey.
//!
//! Both sit behind [`Cache`]; callers pick the backend at wiring time. Phase
//! 2C ships only the in-memory backend ([`InMemoryCache`]); the Valkey impl
//! lands with Phase 4's deployment decisions.
//!
//! # Key hashing
//!
//! [`cache_key_from_canonical_json`] is a small utility for building a
//! stable hash from structured input (e.g. the OpenAI request messages and
//! model name). It mirrors `ValkeyCache.make_input_hash` from FastAPI so
//! the two backends can share a key-space if they ever co-exist.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use tokio::time::Instant;

/// A minimal TTL cache. Values are opaque `String`s — callers serialize
/// structured data to JSON before `set`, and parse it back on `get`.
#[async_trait]
pub trait Cache: Send + Sync {
    /// Return the value for `key` if present and not past its TTL.
    async fn get(&self, key: &str) -> Option<String>;

    /// Store `value` under `key` for at most `ttl`. Overwrites any existing
    /// entry.
    async fn set(&self, key: &str, value: String, ttl: Duration);

    /// Remove a single key. Returns `true` if a value was present.
    async fn invalidate(&self, key: &str) -> bool;

    /// Clear every entry. Primarily for tests.
    async fn clear(&self);
}

#[derive(Debug, Clone)]
struct Entry {
    value: String,
    expires_at: Instant,
}

/// In-process TTL cache backed by a `Mutex<HashMap>`. Suitable for
/// per-instance caches where a global view isn't needed (e.g. the RCF
/// match cache).
#[derive(Debug, Clone, Default)]
pub struct InMemoryCache {
    inner: Arc<Mutex<HashMap<String, Entry>>>,
}

impl InMemoryCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of entries currently stored. Counts expired entries that
    /// haven't yet been lazily evicted — tests should clear first.
    pub async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.is_empty()
    }
}

#[async_trait]
impl Cache for InMemoryCache {
    async fn get(&self, key: &str) -> Option<String> {
        let mut guard = self.inner.lock().await;
        if let Some(entry) = guard.get(key) {
            if Instant::now() < entry.expires_at {
                return Some(entry.value.clone());
            }
            // Lazy eviction.
            guard.remove(key);
        }
        None
    }

    async fn set(&self, key: &str, value: String, ttl: Duration) {
        let entry = Entry {
            value,
            expires_at: Instant::now() + ttl,
        };
        self.inner.lock().await.insert(key.to_string(), entry);
    }

    async fn invalidate(&self, key: &str) -> bool {
        self.inner.lock().await.remove(key).is_some()
    }

    async fn clear(&self) {
        self.inner.lock().await.clear();
    }
}

/// A no-op cache useful as a default when caching should be disabled (tests,
/// or deployments that choose not to spend memory on the RCF cache). Every
/// `get` misses; `set` is a no-op.
#[derive(Debug, Clone, Default)]
pub struct NoopCache;

#[async_trait]
impl Cache for NoopCache {
    async fn get(&self, _key: &str) -> Option<String> {
        None
    }
    async fn set(&self, _key: &str, _value: String, _ttl: Duration) {}
    async fn invalidate(&self, _key: &str) -> bool {
        false
    }
    async fn clear(&self) {}
}

/// Stable SHA-256 hash of the canonical JSON representation of `value`.
///
/// Mirrors `ValkeyCache.make_input_hash` — the FastAPI code JSON-encodes
/// the `(messages, model)` tuple with `sort_keys=True` and hashes the
/// result. Using canonical JSON here lets the Rust and Python code share
/// cache keys byte-for-byte if that ever matters.
pub fn cache_key_from_canonical_json<T: serde::Serialize>(value: &T) -> String {
    // `serde_json::to_vec` is already key-sorted when the input is a `Value`
    // with BTreeMap; for plain structs the field order is the declaration
    // order, which is deterministic across runs of the same binary. Good
    // enough for an in-process cache. If/when the Valkey backend lands with
    // cross-language sharing, add a proper canonical-JSON encoder there.
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let digest = Sha256::digest(&bytes);
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{advance, Duration as TokioDuration};

    #[tokio::test]
    async fn get_returns_none_before_set() {
        let c = InMemoryCache::new();
        assert!(c.get("missing").await.is_none());
    }

    #[tokio::test]
    async fn set_then_get_returns_value() {
        let c = InMemoryCache::new();
        c.set("k", "v".into(), Duration::from_secs(60)).await;
        assert_eq!(c.get("k").await.as_deref(), Some("v"));
    }

    #[tokio::test]
    async fn set_overwrites_prior_value() {
        let c = InMemoryCache::new();
        c.set("k", "v1".into(), Duration::from_secs(60)).await;
        c.set("k", "v2".into(), Duration::from_secs(60)).await;
        assert_eq!(c.get("k").await.as_deref(), Some("v2"));
    }

    #[tokio::test(start_paused = true)]
    async fn entry_expires_after_ttl() {
        let c = InMemoryCache::new();
        c.set("k", "v".into(), Duration::from_secs(5)).await;
        advance(TokioDuration::from_secs(3)).await;
        assert_eq!(c.get("k").await.as_deref(), Some("v"));
        advance(TokioDuration::from_secs(3)).await;
        assert!(c.get("k").await.is_none(), "should be expired");
    }

    #[tokio::test]
    async fn invalidate_removes_entry() {
        let c = InMemoryCache::new();
        c.set("k", "v".into(), Duration::from_secs(60)).await;
        assert!(c.invalidate("k").await);
        assert!(c.get("k").await.is_none());
        assert!(!c.invalidate("k").await, "second call reports absence");
    }

    #[tokio::test]
    async fn clear_empties_cache() {
        let c = InMemoryCache::new();
        c.set("a", "1".into(), Duration::from_secs(60)).await;
        c.set("b", "2".into(), Duration::from_secs(60)).await;
        c.clear().await;
        assert!(c.is_empty().await);
    }

    #[tokio::test]
    async fn noop_cache_always_misses() {
        let c = NoopCache;
        c.set("k", "v".into(), Duration::from_secs(60)).await;
        assert!(c.get("k").await.is_none());
        assert!(!c.invalidate("k").await);
    }

    #[test]
    fn canonical_json_key_is_deterministic() {
        let k1 = cache_key_from_canonical_json(&("gpt-4o-mini", vec!["hi", "there"]));
        let k2 = cache_key_from_canonical_json(&("gpt-4o-mini", vec!["hi", "there"]));
        assert_eq!(k1, k2);
        assert_eq!(k1.len(), 64, "sha-256 hex length");
    }

    #[test]
    fn canonical_json_key_differs_on_input() {
        let k1 = cache_key_from_canonical_json(&("gpt-4o-mini", vec!["hi"]));
        let k2 = cache_key_from_canonical_json(&("gpt-4o-mini", vec!["hello"]));
        assert_ne!(k1, k2);
    }

    #[tokio::test]
    async fn cache_is_trait_object_safe() {
        // Compile-time: the trait must be dyn-compatible so callers can hold
        // `Arc<dyn Cache>` in AppState.
        let c: Arc<dyn Cache> = Arc::new(InMemoryCache::new());
        c.set("k", "v".into(), Duration::from_secs(1)).await;
        assert_eq!(c.get("k").await.as_deref(), Some("v"));
    }
}
