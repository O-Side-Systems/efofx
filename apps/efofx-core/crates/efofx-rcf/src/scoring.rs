//! Pure scoring primitives. No I/O, no allocations beyond the keyword set.
//!
//! Weights and stop-word list are ported from
//! `apps/efofx-estimate/app/services/rcf_engine.py` so the Rust
//! implementation produces identical confidence scores for the same inputs.

use std::collections::HashSet;

/// Stop-word set used by [`extract_keywords`]. Stored as a `&'static [&str]`
/// plus a helper `is_stop` — avoids allocating a `HashSet` on every call.
const STOP_WORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "has", "he", "in", "is", "it",
    "its", "of", "on", "that", "the", "to", "was", "will", "with", "we", "i", "you", "my", "our",
    "want",
];

fn is_stop(word: &str) -> bool {
    STOP_WORDS.contains(&word)
}

/// Lower-case the description, split on whitespace + common punctuation,
/// strip leading/trailing bracket-style chars, and drop stop words plus
/// single-character tokens.
///
/// Mirrors `rcf_engine.extract_keywords` byte-for-byte. Callers pass the
/// raw user-facing description; we don't preserve order (de-dup happens at
/// overlap-calc time).
pub fn extract_keywords(description: &str) -> Vec<String> {
    let normalized = description.to_lowercase().replace([',', '.', ';'], " ");

    let mut out = Vec::new();
    for raw in normalized.split_whitespace() {
        let trimmed = raw.trim_matches(|c: char| "()[]{}!?:;\"'".contains(c));
        if trimmed.chars().count() < 2 {
            continue;
        }
        if is_stop(trimmed) {
            continue;
        }
        out.push(trimmed.to_string());
    }
    out
}

/// Share of `desc_keywords` that also appear in `rc_keywords`. Empty
/// description → `0.0`. Matches the FastAPI formula: set intersection over
/// description size.
pub fn calculate_keyword_overlap(desc_keywords: &[String], rc_keywords: &[String]) -> f64 {
    if desc_keywords.is_empty() {
        return 0.0;
    }
    let desc_set: HashSet<&str> = desc_keywords.iter().map(|s| s.as_str()).collect();
    let rc_set: HashSet<&str> = rc_keywords.iter().map(|s| s.as_str()).collect();
    let matches = desc_set.intersection(&rc_set).count();
    matches as f64 / desc_set.len() as f64
}

/// Weighted confidence score clamped to `[0.0, 1.0]`.
///
/// Formula: `0.6 × keyword_overlap + 0.3 × category_match + 0.1 × region_match`.
/// Matches `rcf_engine.calculate_confidence_score`.
pub fn calculate_confidence_score(
    keyword_overlap: f64,
    category_match: bool,
    region_match: bool,
) -> f64 {
    let score = keyword_overlap * 0.6
        + if category_match { 1.0 } else { 0.0 } * 0.3
        + if region_match { 1.0 } else { 0.0 } * 0.1;
    score.clamp(0.0, 1.0)
}

/// True when `region` matches any entry in `rc_regions`, case-insensitively.
pub fn check_region_match(region: &str, rc_regions: &[String]) -> bool {
    let needle = region.to_lowercase();
    rc_regions.iter().any(|r| r.to_lowercase() == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- extract_keywords -------------------------------------------------

    #[test]
    fn extract_keywords_basic() {
        let k = extract_keywords("I want to build a swimming pool in my backyard");
        for needed in ["build", "swimming", "pool", "backyard"] {
            assert!(k.iter().any(|w| w == needed), "missing `{needed}` in {k:?}");
        }
        for stop in ["i", "want", "to", "a", "in", "my"] {
            assert!(!k.iter().any(|w| w == stop), "stop `{stop}` leaked");
        }
    }

    #[test]
    fn extract_keywords_with_punctuation() {
        let k = extract_keywords("Need a pool, spa, and decking. Budget: $50,000!");
        assert!(k.iter().any(|w| w == "pool"));
        assert!(k.iter().any(|w| w == "spa"));
        assert!(k.iter().any(|w| w == "decking"));
        assert!(k.iter().any(|w| w == "budget"));
    }

    #[test]
    fn extract_keywords_case_insensitive() {
        let k = extract_keywords("POOL Swimming BACKYARD");
        assert!(k.iter().any(|w| w == "pool"));
        assert!(k.iter().any(|w| w == "swimming"));
        assert!(k.iter().any(|w| w == "backyard"));
        assert!(!k.iter().any(|w| w == "POOL"));
    }

    #[test]
    fn extract_keywords_empty() {
        assert!(extract_keywords("").is_empty());
    }

    #[test]
    fn extract_keywords_filters_short_words() {
        let k = extract_keywords("I a pool in LA");
        assert!(!k.iter().any(|w| w == "i"));
        assert!(!k.iter().any(|w| w == "a"));
        assert!(k.iter().any(|w| w == "la"));
    }

    // --- calculate_keyword_overlap ---------------------------------------

    #[test]
    fn overlap_perfect_match() {
        let desc = vec!["pool".into(), "swimming".into(), "backyard".into()];
        let rc = vec!["pool".into(), "swimming".into(), "backyard".into()];
        assert_eq!(calculate_keyword_overlap(&desc, &rc), 1.0);
    }

    #[test]
    fn overlap_partial_match() {
        let desc = vec![
            "pool".into(),
            "swimming".into(),
            "backyard".into(),
            "concrete".into(),
        ];
        let rc = vec!["pool".into(), "swimming".into()];
        assert_eq!(calculate_keyword_overlap(&desc, &rc), 0.5);
    }

    #[test]
    fn overlap_no_match() {
        let desc = vec!["kitchen".into(), "renovation".into()];
        let rc = vec!["pool".into(), "swimming".into()];
        assert_eq!(calculate_keyword_overlap(&desc, &rc), 0.0);
    }

    #[test]
    fn overlap_empty_description() {
        let desc: Vec<String> = vec![];
        let rc = vec!["pool".into()];
        assert_eq!(calculate_keyword_overlap(&desc, &rc), 0.0);
    }

    // --- calculate_confidence_score --------------------------------------

    #[test]
    fn confidence_perfect_match() {
        assert!((calculate_confidence_score(1.0, true, true) - 1.0).abs() < 0.001);
    }

    #[test]
    fn confidence_keyword_only() {
        assert_eq!(calculate_confidence_score(1.0, false, false), 0.6);
    }

    #[test]
    fn confidence_category_only() {
        let s = calculate_confidence_score(0.0, true, false);
        assert!((s - 0.3).abs() < 1e-9, "got {s}");
    }

    #[test]
    fn confidence_partial() {
        let s = calculate_confidence_score(0.5, true, false);
        assert!((s - 0.6).abs() < 1e-9, "got {s}");
    }

    #[test]
    fn confidence_weighted() {
        // 0.8 × 0.6 + 0.3 + 0.1 = 0.88
        assert!((calculate_confidence_score(0.8, true, true) - 0.88).abs() < 0.001);
    }

    #[test]
    fn confidence_clamps_to_0_1() {
        assert_eq!(calculate_confidence_score(0.0, false, false), 0.0);
        assert!(calculate_confidence_score(5.0, true, true) <= 1.0);
    }

    // --- check_region_match ----------------------------------------------

    #[test]
    fn region_match_exact() {
        assert!(check_region_match(
            "us-ca-south",
            &["us-ca-south".into(), "us-ca-north".into()]
        ));
    }

    #[test]
    fn region_match_case_insensitive() {
        assert!(check_region_match("US-CA-SOUTH", &["us-ca-south".into()]));
        assert!(check_region_match("us-ca-south", &["US-CA-SOUTH".into()]));
    }

    #[test]
    fn region_no_match() {
        assert!(!check_region_match(
            "us-tx-houston",
            &["us-ca-south".into(), "us-ca-north".into()]
        ));
    }

    #[test]
    fn region_empty_list() {
        assert!(!check_region_match("us-ca-south", &[]));
    }
}
