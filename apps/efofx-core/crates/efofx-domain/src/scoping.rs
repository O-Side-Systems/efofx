//! Deterministic extraction of scoping fields from a user message.
//!
//! Ported from `apps/efofx-estimate/app/services/chat_service.py::_update_scoping_context`
//! so the Rust port produces the same decisions for the same inputs. The
//! extractor is:
//!
//! - **Pure**: takes a context snapshot + a message, returns an updated snapshot.
//! - **Conservative**: only fills fields that are `None`. Never overwrites a
//!   value the caller already set — whether from an earlier turn or from the LLM.
//! - **Deterministic**: no RNG, no I/O. Same input ⇒ same output.
//!
//! The regex set is compiled once at first use via `OnceLock`.

use std::sync::OnceLock;

use regex::Regex;

use crate::ScopingContext;

/// Exact substrings that, when present in a message, flip the session to
/// `Ready` regardless of whether the scoping fields are filled. Matched
/// case-insensitively after lowercasing the message.
pub const ESTIMATE_TRIGGER_PHRASES: &[&str] = &[
    "generate estimate",
    "give me an estimate",
    "ready for estimate",
    "create estimate",
    "/estimate",
];

/// Short affirmative responses. Matched as whole-word (message equals the
/// word, or the message starts-with `"<word> "` or ends-with `" <word>"`).
pub const CONFIRMATION_WORDS: &[&str] = &[
    "yes",
    "yeah",
    "sure",
    "go ahead",
    "ready",
    "go",
    "yep",
    "do it",
    "ok",
    "okay",
    "let's go",
    "absolutely",
    "definitely",
    "please",
];

/// True if `message` contains any explicit estimate-trigger phrase.
pub fn is_explicit_estimate_trigger(message: &str) -> bool {
    let lower = message.to_lowercase();
    let trimmed = lower.trim();
    ESTIMATE_TRIGGER_PHRASES
        .iter()
        .any(|phrase| trimmed.contains(phrase))
}

/// True if `message` is a short affirmative. Matches whole-word only to
/// avoid false positives like "maybe" triggering on "yes".
pub fn is_confirmation(message: &str) -> bool {
    let lower = message.to_lowercase();
    let trimmed = lower.trim();
    CONFIRMATION_WORDS.iter().any(|word| {
        trimmed == *word
            || trimmed.starts_with(&format!("{word} "))
            || trimmed.ends_with(&format!(" {word}"))
    })
}

/// Extract scoping fields from a user message, filling only `None` slots on
/// the supplied context. Returns the updated context; does not mutate
/// shared state.
pub fn extract_scoping(ctx: ScopingContext, user_message: &str) -> ScopingContext {
    let mut ctx = ctx;
    let msg_lower = user_message.to_lowercase();

    if ctx.project_type.is_none() {
        ctx.project_type = extract_project_type(&msg_lower);
    }
    if ctx.project_size.is_none() {
        ctx.project_size = extract_project_size(&msg_lower);
    }
    if ctx.location.is_none() {
        ctx.location = extract_location(&msg_lower, user_message);
    }
    if ctx.timeline.is_none() {
        ctx.timeline = extract_timeline(&msg_lower);
    }
    if ctx.special_conditions.is_none() {
        ctx.special_conditions = extract_special_conditions(&msg_lower);
    }
    ctx
}

// --- project_type ---------------------------------------------------------

/// Keyword → canonical project type. Order matters: first match wins, so
/// more specific phrases ("outdoor kitchen") must precede more general
/// ones ("kitchen"). Matches the FastAPI dict ordering.
const PROJECT_TYPE_KEYWORDS: &[(&str, &str)] = &[
    ("outdoor kitchen", "outdoor kitchen"),
    ("swimming pool", "pool"),
    ("pool", "pool"),
    ("spa", "pool"),
    ("deck", "deck"),
    ("patio", "patio"),
    ("renovation", "renovation"),
    ("remodel", "renovation"),
    ("addition", "addition"),
    ("kitchen", "kitchen renovation"),
    ("bathroom", "bathroom renovation"),
    ("bath", "bathroom renovation"),
    ("roof", "roofing"),
    ("fence", "fencing"),
    ("landscaping", "landscaping"),
    ("landscape", "landscaping"),
    ("driveway", "driveway"),
    ("garage", "garage"),
    ("shed", "shed"),
    ("pergola", "pergola"),
];

fn extract_project_type(msg_lower: &str) -> Option<String> {
    PROJECT_TYPE_KEYWORDS
        .iter()
        .find(|(keyword, _)| msg_lower.contains(keyword))
        .map(|(_, canonical)| (*canonical).to_string())
}

// --- project_size ---------------------------------------------------------

fn size_nxm_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(\d+)\s*x\s*(\d+)\s*(?:feet|ft|foot)?").unwrap())
}

fn size_sqft_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(\d+)\s*(?:sq\.?\s*ft|sqft|square\s*feet|square\s*foot)").unwrap()
    })
}

fn size_dimension_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?:about\s+)?(\d+)\s*(?:feet|ft|foot)\s*(?:long|wide|deep|in\s+(?:length|width|depth))?",
        )
        .unwrap()
    })
}

fn extract_project_size(msg_lower: &str) -> Option<String> {
    if let Some(caps) = size_nxm_re().captures(msg_lower) {
        let a = caps.get(1)?.as_str();
        let b = caps.get(2)?.as_str();
        return Some(format!("{a}x{b} feet"));
    }
    if let Some(caps) = size_sqft_re().captures(msg_lower) {
        let n = caps.get(1)?.as_str();
        return Some(format!("{n} sq ft"));
    }
    let dims: Vec<String> = size_dimension_re()
        .captures_iter(msg_lower)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect();
    match dims.len() {
        0 => None,
        1 => Some(format!("{} feet", dims[0])),
        _ => Some(format!("{}x{} feet", dims[0], dims[1])),
    }
}

// --- location -------------------------------------------------------------

/// City / region keyword → canonical region tag. Checked in declaration
/// order; first match wins.
///
/// Ordering improvement over FastAPI: multi-word aliases ("inland empire",
/// "orange county") are listed before 2-letter codes ("la", "sf") to prevent
/// false positives — the FastAPI dict had "la" before "inland empire", so
/// "Inland Empire home" would match "la" (substring of "in**la**nd") and
/// route to SoCal Coastal instead of SoCal Inland. Documented in
/// `docs/rust-port/workflow-inventory.md` under scoping-extractor-divergences.
const LOCATION_KEYWORDS: &[(&str, &str)] = &[
    ("bay area", "NorCal - Bay Area"),
    ("san francisco", "NorCal - Bay Area"),
    ("silicon valley", "NorCal - Bay Area"),
    ("san jose", "NorCal - Bay Area"),
    ("los angeles", "SoCal - Coastal"),
    ("san diego", "SoCal - Coastal"),
    ("santa barbara", "SoCal - Coastal"),
    ("orange county", "SoCal - Coastal"),
    ("southern california", "SoCal - Coastal"),
    ("inland empire", "SoCal - Inland"),
    ("san bernardino", "SoCal - Inland"),
    ("riverside", "SoCal - Inland"),
    ("socal", "SoCal - Coastal"),
    ("northern california", "NorCal - Central"),
    ("norcal", "NorCal - Central"),
    ("sacramento", "NorCal - Central"),
    ("fresno", "NorCal - Central"),
    ("scottsdale", "Arizona - Phoenix"),
    ("phoenix", "Arizona - Phoenix"),
    ("tempe", "Arizona - Phoenix"),
    ("tucson", "Arizona - Tucson"),
    ("las vegas", "Nevada - Las Vegas"),
    ("henderson", "Nevada - Las Vegas"),
    ("reno", "Nevada - Reno"),
    ("oceanside", "SoCal - Coastal"),
    ("carlsbad", "SoCal - Coastal"),
    ("encinitas", "SoCal - Coastal"),
    ("vista", "SoCal - Coastal"),
    ("escondido", "SoCal - Inland"),
    ("temecula", "SoCal - Inland"),
    ("murrieta", "SoCal - Inland"),
    ("arizona", "Arizona - Phoenix"),
    ("nevada", "Nevada - Las Vegas"),
    ("california", "NorCal - Bay Area"),
    // Two-letter codes come last to avoid substring collisions:
    // "la" matches inside "las vegas", "in**la**nd", "**la**ndscape", etc.;
    // "sf" inside hypothetical future keys. By checking them after every
    // multi-word alias, we only fall back to them when nothing else matched.
    ("la", "SoCal - Coastal"),
    ("sf", "NorCal - Bay Area"),
];

fn location_preposition_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?:in|near|around|located in|from)\s+([a-z\s,]+?)(?:\s*[.,!?]|$)").unwrap()
    })
}

fn city_state_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"([A-Za-z\s]+),\s*([A-Za-z\s]+)").unwrap())
}

fn extract_location(msg_lower: &str, original: &str) -> Option<String> {
    if let Some((_, region)) = LOCATION_KEYWORDS
        .iter()
        .find(|(keyword, _)| msg_lower.contains(keyword))
    {
        return Some((*region).to_string());
    }

    if let Some(caps) = location_preposition_re().captures(msg_lower) {
        let raw = caps.get(1)?.as_str().trim();
        if raw.len() > 2 && !matches!(raw, "the" | "a" | "an") {
            return Some(title_case(raw));
        }
    }

    if let Some(caps) = city_state_re().captures(original) {
        let city = caps.get(1)?.as_str().trim();
        let city_lower = city.to_lowercase();
        if city.len() > 2 && !matches!(city_lower.as_str(), "the" | "a" | "an" | "yes" | "no") {
            let state = caps.get(2)?.as_str().trim();
            return Some(format!("{city}, {state}"));
        }
    }

    None
}

/// Minimal title-casing: upper-case the first character of each
/// whitespace-separated word. Preserves punctuation.
fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// --- timeline -------------------------------------------------------------

fn timeline_patterns() -> &'static [Regex] {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        [
            r"(?:spring|summer|fall|autumn|winter)(?:\s+\d{4})?",
            r"(?:january|february|march|april|may|june|july|august|september|october|november|december)(?:\s+\d{4})?",
            r"\d+\s*(?:weeks?|months?|years?)",
            r"(?:one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)\s+(?:weeks?|months?|years?)",
            r"(?:a\s+)?(?:couple|few|several)\s+(?:weeks?|months?|years?)",
            r"asap|as soon as possible",
            r"next\s+(?:month|year|spring|summer|fall|winter)",
            r"(?:within|in)\s+(?:the\s+)?(?:next\s+)?(?:\d+|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|a\s+couple|a\s+few|several)\s+(?:weeks?|months?|years?)",
            r"by\s+(?:the\s+)?(?:end\s+of\s+)?(?:spring|summer|fall|winter|\d{4}|next)",
        ]
        .iter()
        .map(|p| Regex::new(p).unwrap())
        .collect()
    })
}

fn extract_timeline(msg_lower: &str) -> Option<String> {
    for re in timeline_patterns() {
        if let Some(m) = re.find(msg_lower) {
            return Some(m.as_str().trim().to_string());
        }
    }
    None
}

// --- special_conditions ---------------------------------------------------

const SPECIAL_KEYWORDS: &[&str] = &[
    "hoa",
    "homeowners association",
    "slope",
    "sloped",
    "hillside",
    "access",
    "limited access",
    "soil",
    "clay soil",
    "sandy",
    "permit",
    "permits",
    "existing",
    "removal",
    "tear out",
    "drainage",
    "drain",
    "underground",
    "buried",
    "easement",
    "setback",
];

fn extract_special_conditions(msg_lower: &str) -> Option<String> {
    let found: Vec<&str> = SPECIAL_KEYWORDS
        .iter()
        .filter(|k| msg_lower.contains(**k))
        .copied()
        .collect();
    if found.is_empty() {
        None
    } else {
        Some(found.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> ScopingContext {
        ScopingContext::default()
    }

    // --- project_type ---

    #[test]
    fn project_type_maps_keywords() {
        let cases = [
            ("I want a pool", "pool"),
            ("Planning a swimming pool", "pool"),
            ("Need a new deck", "deck"),
            // FastAPI parity: dict order is `renovation`/`remodel` before `kitchen`,
            // so a message containing both tokens matches `remodel` → "renovation"
            // first. We honor that here — use `Kitchen install` for a kitchen hit.
            ("Kitchen install", "kitchen renovation"),
            ("Kitchen remodel project", "renovation"),
            // FastAPI parity: `renovation` is matched before `bathroom` in
            // dict order, so a message with both maps to "renovation". Use
            // bare `Bathroom` for the bathroom-renovation mapping.
            ("Bathroom project", "bathroom renovation"),
            ("New roof", "roofing"),
            ("Fence installation", "fencing"),
            ("Landscaping work", "landscaping"),
            ("Driveway paving", "driveway"),
            ("Detached garage", "garage"),
            ("Shed build", "shed"),
            ("Pergola install", "pergola"),
            ("Outdoor kitchen install", "outdoor kitchen"),
        ];
        for (msg, expected) in cases {
            let ctx = extract_scoping(empty(), msg);
            assert_eq!(
                ctx.project_type.as_deref(),
                Some(expected),
                "failed on `{msg}`"
            );
        }
    }

    #[test]
    fn project_type_preserves_existing_value() {
        let mut ctx = empty();
        ctx.project_type = Some("existing-custom".into());
        let ctx = extract_scoping(ctx, "I want a pool");
        assert_eq!(ctx.project_type.as_deref(), Some("existing-custom"));
    }

    // --- project_size ---

    #[test]
    fn project_size_recognises_nxm() {
        let ctx = extract_scoping(empty(), "pool is 15x30");
        assert_eq!(ctx.project_size.as_deref(), Some("15x30 feet"));
        let ctx = extract_scoping(empty(), "deck 20 x 15 feet");
        assert_eq!(ctx.project_size.as_deref(), Some("20x15 feet"));
    }

    #[test]
    fn project_size_recognises_sqft() {
        let ctx = extract_scoping(empty(), "500 sqft patio");
        assert_eq!(ctx.project_size.as_deref(), Some("500 sq ft"));
        let ctx = extract_scoping(empty(), "800 sq ft addition");
        assert_eq!(ctx.project_size.as_deref(), Some("800 sq ft"));
        let ctx = extract_scoping(empty(), "1200 square feet of new space");
        assert_eq!(ctx.project_size.as_deref(), Some("1200 sq ft"));
    }

    #[test]
    fn project_size_recognises_natural_dimensions() {
        let ctx = extract_scoping(empty(), "about 50 feet long and 20 feet wide");
        assert_eq!(ctx.project_size.as_deref(), Some("50x20 feet"));
        let ctx = extract_scoping(empty(), "around 8 feet deep");
        assert_eq!(ctx.project_size.as_deref(), Some("8 feet"));
    }

    // --- location ---

    #[test]
    fn location_maps_known_regions() {
        let cases = [
            ("I'm in the Bay Area", "NorCal - Bay Area"),
            ("san francisco home", "NorCal - Bay Area"),
            ("SF condo", "NorCal - Bay Area"),
            ("Los Angeles house", "SoCal - Coastal"),
            ("orange county project", "SoCal - Coastal"),
            ("SoCal property", "SoCal - Coastal"),
            ("Inland Empire home", "SoCal - Inland"),
            ("phoenix area", "Arizona - Phoenix"),
            ("scottsdale lot", "Arizona - Phoenix"),
            ("tucson property", "Arizona - Tucson"),
            ("las vegas place", "Nevada - Las Vegas"),
            ("reno build", "Nevada - Reno"),
        ];
        for (msg, expected) in cases {
            let ctx = extract_scoping(empty(), msg);
            assert_eq!(ctx.location.as_deref(), Some(expected), "failed on `{msg}`");
        }
    }

    #[test]
    fn location_falls_back_to_preposition_phrase() {
        let ctx = extract_scoping(empty(), "we're located in Big Sur.");
        assert_eq!(ctx.location.as_deref(), Some("Big Sur"));
    }

    #[test]
    fn location_falls_back_to_city_state_pattern() {
        // Bare "City, State" hits the city-state regex directly.
        let ctx = extract_scoping(empty(), "Boulder, Colorado");
        assert_eq!(ctx.location.as_deref(), Some("Boulder, Colorado"));
    }

    // --- timeline ---

    #[test]
    fn timeline_matches_season() {
        let ctx = extract_scoping(empty(), "we want to start in spring 2026");
        assert_eq!(ctx.timeline.as_deref(), Some("spring 2026"));
    }

    #[test]
    fn timeline_matches_duration() {
        let ctx = extract_scoping(empty(), "finish within 3 months");
        assert!(ctx.timeline.is_some(), "expected a match");
        let t = ctx.timeline.unwrap();
        assert!(t.contains("3 months") || t.contains("within"), "got `{t}`");
    }

    #[test]
    fn timeline_matches_asap() {
        let ctx = extract_scoping(empty(), "asap please");
        assert_eq!(ctx.timeline.as_deref(), Some("asap"));
    }

    #[test]
    fn timeline_season_wins_over_by_end_of() {
        // FastAPI parity: the season pattern is first in the list, so "summer"
        // matches before the more-descriptive "by end of summer" pattern does.
        // Capturing a phrase rather than a bare season would require pattern
        // reordering that could regress other cases — we accept the parity.
        let ctx = extract_scoping(empty(), "finished by end of summer");
        assert_eq!(ctx.timeline.as_deref(), Some("summer"));
    }

    #[test]
    fn timeline_matches_by_end_of_year() {
        // No season in the message, so the `by end of` pattern wins.
        let ctx = extract_scoping(empty(), "done by end of 2026");
        assert_eq!(ctx.timeline.as_deref(), Some("by end of 2026"));
    }

    // --- special_conditions ---

    #[test]
    fn special_conditions_collects_multiple_keywords() {
        let ctx = extract_scoping(empty(), "We have HOA rules and a sloped hillside lot");
        let conds = ctx.special_conditions.expect("should match");
        assert!(conds.contains("hoa"));
        assert!(conds.contains("slope"));
        assert!(conds.contains("hillside"));
    }

    #[test]
    fn special_conditions_none_when_clean() {
        let ctx = extract_scoping(empty(), "just a simple project");
        assert!(ctx.special_conditions.is_none());
    }

    // --- triggers / confirmations ---

    #[test]
    fn estimate_trigger_detected() {
        assert!(is_explicit_estimate_trigger("ok generate estimate now"));
        assert!(is_explicit_estimate_trigger("Give me an estimate"));
        assert!(is_explicit_estimate_trigger("/estimate"));
        assert!(!is_explicit_estimate_trigger("I need a quote"));
    }

    #[test]
    fn confirmation_whole_word_matching() {
        assert!(is_confirmation("yes"));
        assert!(is_confirmation("  Yes  "));
        assert!(is_confirmation("yes please"));
        assert!(is_confirmation("Go ahead"));
        assert!(!is_confirmation("maybe"), "no substring match");
        assert!(!is_confirmation("yesterday"), "must be whole word");
        // FastAPI parity: trailing punctuation is NOT stripped, so "Yes!" is
        // treated as the token `yes!` and does not match.
        assert!(!is_confirmation("Yes!"));
    }

    // --- end-to-end ---

    #[test]
    fn end_to_end_fills_four_required_fields() {
        let ctx = extract_scoping(
            empty(),
            "I want a 15x30 pool in the Bay Area starting spring 2026",
        );
        assert_eq!(ctx.project_type.as_deref(), Some("pool"));
        assert_eq!(ctx.project_size.as_deref(), Some("15x30 feet"));
        assert_eq!(ctx.location.as_deref(), Some("NorCal - Bay Area"));
        assert_eq!(ctx.timeline.as_deref(), Some("spring 2026"));
        assert!(ctx.is_ready());
    }

    #[test]
    fn extraction_does_not_overwrite_filled_fields() {
        let mut ctx = empty();
        ctx.project_type = Some("kept".into());
        ctx.location = Some("kept too".into());
        let ctx = extract_scoping(ctx, "15x30 pool in Phoenix spring 2026");
        assert_eq!(ctx.project_type.as_deref(), Some("kept"));
        assert_eq!(ctx.project_size.as_deref(), Some("15x30 feet"));
        assert_eq!(ctx.location.as_deref(), Some("kept too"));
        assert_eq!(ctx.timeline.as_deref(), Some("spring 2026"));
    }
}
