//! Cache-backed reference-class matcher.
//!
//! `RcfMatcher` wraps a [`ReferenceRepo`] and a [`Cache`] backend and
//! implements the confidence-gated matching semantics from
//! `app/services/rcf_engine.py::find_matching_reference_class`:
//!
//! 1. Hash the `(description, category, region, tenant_id)` tuple and
//!    return the cached result if present.
//! 2. Extract keywords; fail fast on empty description.
//! 3. Query reference classes by category (honoring tenant visibility).
//! 4. Score each candidate, preferring tenant-specific rows on ties.
//! 5. Reject matches below the 0.7 confidence threshold.
//! 6. Cache the winning result with a 5-minute TTL.
//!
//! The cache key is a SHA-256 of canonical JSON — deterministic across
//! invocations; FastAPI uses `{description}:{category}:{region}:{tenant}`
//! as a plain string key. The Rust port switches to hashed keys because
//! the cache trait is generic over backends (a future Valkey impl will
//! benefit from fixed-length keys).

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use efofx_cache::{cache_key_from_canonical_json, Cache};
use efofx_domain::ReferenceClass;
use efofx_storage::{decode_reference_class, ReferenceRepo, StorageError, TenantContext};
use mongodb::bson::Document;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};
use utoipa::ToSchema;

use crate::scoring::{
    calculate_confidence_score, calculate_keyword_overlap, check_region_match, extract_keywords,
};

/// Minimum confidence required for a match to count. Below this, the
/// caller is expected to ask for more detail.
pub const CONFIDENCE_THRESHOLD: f64 = 0.7;

/// TTL for cached matches. Mirrors the FastAPI `_CACHE_TTL_SECONDS = 300`.
pub const MATCH_CACHE_TTL: Duration = Duration::from_secs(300);

/// Errors surfaced by [`RcfMatcher::find_matching_reference_class`].
#[derive(Debug, Error)]
pub enum MatchError {
    #[error("please provide more details in your project description")]
    EmptyKeywords,
    #[error("no reference classes available for category '{0}'")]
    NoClassesForCategory(String),
    #[error("could not find a confident match (confidence: {confidence:.2}); please provide more specific details")]
    LowConfidence { confidence: f64 },
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("reference class decode failed: {0}")]
    Decode(String),
}

/// Result of a successful match. Serializable so the wrapping service can
/// cache the JSON directly.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RcfMatch {
    pub reference_class: ReferenceClass,
    pub confidence: f64,
    pub match_metadata: MatchMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MatchMetadata {
    pub description_keywords: Vec<String>,
    pub matched_keywords: Vec<String>,
    pub is_tenant_specific: bool,
    /// Wall-clock elapsed in the match call, rounded to 2 decimals.
    pub processing_time_ms: f64,
}

/// Internal key payload so the cache digest captures every input that can
/// change the result. `cache_key_from_canonical_json` hashes this via serde.
#[derive(Serialize)]
struct CacheKey<'a> {
    description: &'a str,
    category: &'a str,
    region: &'a str,
    tenant_id: Option<&'a str>,
}

/// Cache-backed matcher. Cheap to clone.
#[derive(Clone)]
pub struct RcfMatcher {
    repo: ReferenceRepo,
    cache: Arc<dyn Cache>,
}

impl RcfMatcher {
    pub fn new(repo: ReferenceRepo, cache: Arc<dyn Cache>) -> Self {
        Self { repo, cache }
    }

    /// Find the best-matching reference class. Returns `Err` on any
    /// failure — low-confidence, missing category, empty description,
    /// storage error.
    pub async fn find_matching_reference_class(
        &self,
        description: &str,
        category: &str,
        region: &str,
        ctx: Option<&TenantContext>,
    ) -> Result<RcfMatch, MatchError> {
        let start = std::time::Instant::now();
        let tenant_id_str = ctx.map(|c| c.tenant_id().to_string());

        // --- cache lookup ----------------------------------------------
        let key = cache_key_from_canonical_json(&CacheKey {
            description,
            category,
            region,
            tenant_id: tenant_id_str.as_deref(),
        });
        if let Some(raw) = self.cache.get(&key).await {
            match serde_json::from_str::<RcfMatch>(&raw) {
                Ok(m) => {
                    debug!(key = %key, "rcf cache hit");
                    return Ok(m);
                }
                Err(e) => {
                    // Corrupt/out-of-version entry — drop it and re-compute.
                    warn!(error = %e, "rcf cache entry rejected; recomputing");
                    let _ = self.cache.invalidate(&key).await;
                }
            }
        }

        // --- extract keywords ------------------------------------------
        let desc_keywords = extract_keywords(description);
        if desc_keywords.is_empty() {
            return Err(MatchError::EmptyKeywords);
        }

        // --- fetch candidates ------------------------------------------
        let candidates: Vec<Document> = self
            .repo
            .list_reference_class_docs_for_category(ctx, category)
            .await?;
        if candidates.is_empty() {
            return Err(MatchError::NoClassesForCategory(category.to_string()));
        }

        // --- score ------------------------------------------------------
        let tenant_id = ctx.map(|c| c.tenant_id().to_string());
        let scored = score_candidates(&candidates, &desc_keywords, category, region, &tenant_id)?;
        let (best_doc, best_confidence, is_tenant_specific, matched_keywords) = scored;

        let elapsed_ms = start.elapsed().as_secs_f64() * 1_000.0;
        info!(
            category,
            region,
            confidence = best_confidence,
            tenant_specific = is_tenant_specific,
            elapsed_ms,
            "rcf match attempt"
        );

        if best_confidence < CONFIDENCE_THRESHOLD {
            warn!(
                confidence = best_confidence,
                "rcf low-confidence match — rejecting"
            );
            return Err(MatchError::LowConfidence {
                confidence: best_confidence,
            });
        }

        let reference_class = decode_reference_class(best_doc).map_err(MatchError::Decode)?;

        let m = RcfMatch {
            reference_class,
            confidence: round3(best_confidence),
            match_metadata: MatchMetadata {
                description_keywords: desc_keywords,
                matched_keywords,
                is_tenant_specific,
                processing_time_ms: round2(elapsed_ms),
            },
        };

        // --- cache write ------------------------------------------------
        if let Ok(encoded) = serde_json::to_string(&m) {
            self.cache.set(&key, encoded, MATCH_CACHE_TTL).await;
        }
        Ok(m)
    }
}

/// Score every candidate; return the best by (confidence desc, then
/// tenant-specific preferred).
#[allow(clippy::type_complexity)]
fn score_candidates(
    docs: &[Document],
    desc_keywords: &[String],
    category: &str,
    region: &str,
    tenant_id: &Option<String>,
) -> Result<(Document, f64, bool, Vec<String>), MatchError> {
    let mut scored: Vec<(Document, f64, bool, Vec<String>)> = Vec::with_capacity(docs.len());
    for doc in docs {
        let doc_category = doc.get_str("category").unwrap_or("");
        let category_match = doc_category.eq_ignore_ascii_case(category);

        let rc_regions = doc
            .get_array("regions")
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let region_match = check_region_match(region, &rc_regions);

        let rc_keywords = doc
            .get_array("keywords")
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let overlap = calculate_keyword_overlap(desc_keywords, &rc_keywords);
        let confidence = calculate_confidence_score(overlap, category_match, region_match);

        let is_tenant_specific = match (tenant_id, doc.get("tenant_id")) {
            (Some(tid), Some(mongodb::bson::Bson::String(doc_tid))) => tid == doc_tid,
            _ => false,
        };

        // Intersection of description_keywords and rc_keywords.
        let matched: Vec<String> = {
            let rc_set: std::collections::HashSet<&str> =
                rc_keywords.iter().map(|s| s.as_str()).collect();
            desc_keywords
                .iter()
                .filter(|k| rc_set.contains(k.as_str()))
                .cloned()
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect()
        };

        scored.push((doc.clone(), confidence, is_tenant_specific, matched));
    }

    // Sort by (confidence desc, tenant_specific desc).
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.2.cmp(&a.2))
    });

    scored
        .into_iter()
        .next()
        .ok_or_else(|| MatchError::NoClassesForCategory(category.to_string()))
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
fn round3(v: f64) -> f64 {
    (v * 1_000.0).round() / 1_000.0
}

// Keep `BTreeMap` reachable in case future callers want a typed view;
// silence dead-code for the demo path.
#[allow(dead_code)]
fn _types_in_scope(_m: BTreeMap<String, f64>) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_stable_for_same_inputs() {
        let a = cache_key_from_canonical_json(&CacheKey {
            description: "pool project",
            category: "construction",
            region: "us-ca",
            tenant_id: Some("tenant1"),
        });
        let b = cache_key_from_canonical_json(&CacheKey {
            description: "pool project",
            category: "construction",
            region: "us-ca",
            tenant_id: Some("tenant1"),
        });
        assert_eq!(a, b);
    }

    #[test]
    fn cache_key_differs_with_tenant() {
        let anon = cache_key_from_canonical_json(&CacheKey {
            description: "pool project",
            category: "construction",
            region: "us-ca",
            tenant_id: None,
        });
        let tenant = cache_key_from_canonical_json(&CacheKey {
            description: "pool project",
            category: "construction",
            region: "us-ca",
            tenant_id: Some("tenant1"),
        });
        assert_ne!(anon, tenant);
    }

    // Scoring behavior is exercised against raw Documents — the DB-free
    // cases give us confidence without needing a running Mongo.
    use mongodb::bson::{doc, Bson};

    fn class_doc(
        name: &str,
        tenant: Option<&str>,
        keywords: &[&str],
        regions: &[&str],
    ) -> Document {
        let mut d = doc! {
            "name": name,
            "category": "construction",
            "subcategory": "pool",
            "description": "test class",
            "keywords": keywords.iter().map(|s| Bson::String((*s).into())).collect::<Vec<_>>(),
            "regions": regions.iter().map(|s| Bson::String((*s).into())).collect::<Vec<_>>(),
            "cost_distribution": { "p50": 1.0, "p80": 2.0, "p95": 3.0, "currency": "USD" },
            "timeline_distribution": { "p50_days": 1_i32, "p80_days": 2_i32, "p95_days": 3_i32 },
            "cost_breakdown_template": { "materials": 1.0 },
            "is_synthetic": true,
            "validation_source": "test",
            "created_at": mongodb::bson::DateTime::now(),
        };
        match tenant {
            Some(t) => d.insert("tenant_id", t),
            None => d.insert("tenant_id", Bson::Null),
        };
        d
    }

    #[test]
    fn score_picks_highest_confidence_match() {
        let docs = vec![
            class_doc(
                "pool",
                None,
                &["pool", "swimming", "residential"],
                &["us-ca-south"],
            ),
            class_doc("kitchen", None, &["kitchen", "remodel"], &["us-ca-south"]),
        ];
        let desc = vec!["pool".to_string(), "swimming".to_string()];
        let (best, conf, tenant, matched) =
            score_candidates(&docs, &desc, "construction", "us-ca-south", &None).unwrap();
        assert_eq!(best.get_str("name").unwrap(), "pool");
        assert!(conf >= 0.7, "expected >=0.7, got {conf}");
        assert!(!tenant);
        assert!(matched.contains(&"pool".to_string()));
        assert!(matched.contains(&"swimming".to_string()));
    }

    #[test]
    fn score_prefers_tenant_specific_on_ties() {
        let docs = vec![
            class_doc(
                "platform_pool",
                None,
                &["pool", "swimming"],
                &["us-ca-south"],
            ),
            class_doc(
                "tenant_pool",
                Some("tenant-abc"),
                &["pool", "swimming"],
                &["us-ca-south"],
            ),
        ];
        let desc = vec!["pool".to_string(), "swimming".to_string()];
        let (best, _conf, is_tenant, _matched) = score_candidates(
            &docs,
            &desc,
            "construction",
            "us-ca-south",
            &Some("tenant-abc".to_string()),
        )
        .unwrap();
        assert_eq!(best.get_str("name").unwrap(), "tenant_pool");
        assert!(is_tenant);
    }

    #[test]
    fn score_region_mismatch_lowers_confidence() {
        let docs = vec![class_doc(
            "pool",
            None,
            &["pool", "swimming"],
            &["us-ca-south"],
        )];
        let desc = vec!["pool".to_string(), "swimming".to_string()];

        let (_, conf_ca, _, _) =
            score_candidates(&docs, &desc, "construction", "us-ca-south", &None).unwrap();
        let (_, conf_tx, _, _) =
            score_candidates(&docs, &desc, "construction", "us-tx-houston", &None).unwrap();
        assert!(conf_ca > conf_tx);
    }
}
