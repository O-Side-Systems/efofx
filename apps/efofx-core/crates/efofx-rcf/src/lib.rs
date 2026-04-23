//! Reference Class Forecasting engine.
//!
//! Two layers:
//!
//! - [`scoring`] — pure functions (`extract_keywords`,
//!   `calculate_keyword_overlap`, `calculate_confidence_score`,
//!   `check_region_match`). No I/O.
//! - [`baseline`] — pure calculators (`calculate_baseline_estimate`,
//!   `apply_adjustments`) that turn a [`efofx_domain::ReferenceClass`]
//!   into a cost/timeline estimate with complexity+risk multipliers.
//! - [`RcfMatcher`] — cache-backed matcher that selects the best
//!   [`efofx_domain::ReferenceClass`] for a given
//!   `(description, category, region, tenant?)` tuple.
//!
//! Ported from `apps/efofx-estimate/app/services/rcf_engine.py` with
//! behavior-preserving changes only. Scoring weights, confidence threshold
//! (0.7), and cache TTL (5 min) match the FastAPI implementation.

pub mod baseline;
pub mod matcher;
pub mod scoring;

pub use baseline::{
    apply_adjustments, calculate_baseline_estimate, AdjustedEstimate, BaselineEstimate, Complexity,
    RiskLevel,
};
pub use matcher::{
    MatchError, MatchMetadata, RcfMatch, RcfMatcher, CONFIDENCE_THRESHOLD, MATCH_CACHE_TTL,
};
pub use scoring::{
    calculate_confidence_score, calculate_keyword_overlap, check_region_match, extract_keywords,
};
