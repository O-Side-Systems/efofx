//! Partner-routing tag derivation and URL templating (Phase 2F).
//!
//! Pure functions over `(EstimationSession, ScopingContext,
//! EstimationOutput, RoutingConfig)`. Two callers consume this:
//!
//! * The chat SSE pipeline (Phase 2F.2) — emits `routing_tags` on the
//!   `done` event so partners with a streaming integration can react
//!   without a follow-up call.
//! * The `POST /v1/integration/contractor-match` handler (Phase 2F.3)
//!   — same tag derivation plus the URL template substitution.
//!
//! Disabled tenants (`RoutingConfig.enabled = false`, or no
//! `settings.routing` document at all) get `Vec::new()` and `None`
//! everywhere — never an error. The wire shape stays stable so
//! downstream parsers don't need conditional branches.

use efofx_domain::{EstimationOutput, EstimationSession, ScopingContext};
use efofx_storage::RoutingConfig;

/// Cost-tier slug derived from `EstimationOutput.total_cost_p50` and
/// the tenant's configured breakpoints. Lower bound is inclusive: a
/// p50 exactly at `breakpoints[0]` lands in `Mid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostTier {
    Low,
    Mid,
    High,
}

impl CostTier {
    /// Wire string used in tag values and the `{cost_tier}` template
    /// placeholder.
    pub fn as_str(self) -> &'static str {
        match self {
            CostTier::Low => "low",
            CostTier::Mid => "mid",
            CostTier::High => "high",
        }
    }

    /// Bucket a p50 cost using the tenant's effective breakpoints.
    pub fn from_p50(p50_cost: f64, breakpoints: [u64; 2]) -> Self {
        // Negative or NaN never reach this path in production (the
        // LLM output schema constrains positive numbers), but guard
        // anyway so a malformed estimate doesn't panic the SSE
        // stream. NaN comparisons return false on both branches —
        // bucket as `Low` rather than crash.
        let p50 = if p50_cost.is_finite() && p50_cost > 0.0 {
            p50_cost as u64
        } else {
            0
        };
        if p50 >= breakpoints[1] {
            CostTier::High
        } else if p50 >= breakpoints[0] {
            CostTier::Mid
        } else {
            CostTier::Low
        }
    }
}

/// Derived routing data passed to both consumers. Keeping the
/// intermediate fields visible (rather than collapsing to just
/// `tags`) lets the contractor-match handler reuse the cost-tier
/// slug in the response without redoing the bucket math.
#[derive(Debug, Clone)]
pub struct RoutingData {
    pub tags: Vec<String>,
    pub region: Option<String>,
    pub project_type: Option<String>,
    pub cost_tier: CostTier,
    pub location: Option<String>,
}

/// Pure derivation. Disabled tenants get `tags: Vec::new()`; the
/// other fields are still populated so the contractor-match
/// response can show them in its `region` / `estimated_cost_tier`
/// fields.
///
/// Tag namespace is colon-prefixed so partners can filter cleanly:
///
/// * `region:<slug>` — region label, lowercased + spaces/dashes
///   collapsed to single dashes
/// * `type:<slug>` — project type after applying `tag_overrides`
/// * `tier:<slug>` — `low | mid | high`
/// * `location:<slug>` — location string lowercased + slugified
pub fn derive(
    session: &EstimationSession,
    scoping: &ScopingContext,
    output: &EstimationOutput,
    config: Option<&RoutingConfig>,
) -> RoutingData {
    let breakpoints = config
        .map(|c| c.effective_breakpoints())
        .unwrap_or([25_000, 250_000]);
    let cost_tier = CostTier::from_p50(output.total_cost_p50, breakpoints);

    let region_label = serde_json::to_value(session.region)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string));
    let region_slug = region_label.as_deref().map(slugify);

    let project_type_canonical = scoping.project_type.as_deref().map(slugify);
    let project_type = project_type_canonical
        .as_deref()
        .map(|canon| apply_override(canon, config));

    let location_slug = scoping.location.as_deref().map(slugify);

    let mut tags: Vec<String> = Vec::new();
    if config.map(|c| c.enabled).unwrap_or(false) {
        if let Some(slug) = region_slug.as_deref() {
            tags.push(format!("region:{slug}"));
        }
        if let Some(slug) = project_type.as_deref() {
            tags.push(format!("type:{slug}"));
        }
        tags.push(format!("tier:{}", cost_tier.as_str()));
        if let Some(slug) = location_slug.as_deref() {
            tags.push(format!("location:{slug}"));
        }
    }

    RoutingData {
        tags,
        region: region_label,
        project_type,
        cost_tier,
        location: scoping.location.clone(),
    }
}

/// Substitute `{tags}`, `{region}`, `{project_type}`, `{cost_tier}`
/// in the configured `directory_url_template`. Unknown placeholders
/// (e.g. `{location}` if a partner needs it later) pass through
/// untouched — additive forward compat.
///
/// Returns `None` when there is no template configured. Callers
/// surface that as a `directory_url: None` field — partners can
/// distinguish "no preferred destination" from "empty URL".
pub fn render_directory_url(data: &RoutingData, config: Option<&RoutingConfig>) -> Option<String> {
    let template = config.and_then(|c| c.directory_url_template.as_deref())?;
    let tags_joined = data.tags.join(",");
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start + 1..].find('}') else {
            // Unbalanced `{` — emit the remainder verbatim.
            out.push_str(&rest[start..]);
            rest = "";
            break;
        };
        let name = &rest[start + 1..start + 1 + end];
        let replacement: Option<&str> = match name {
            "tags" => Some(tags_joined.as_str()),
            "region" => data.region.as_deref(),
            "project_type" => data.project_type.as_deref(),
            "cost_tier" => Some(data.cost_tier.as_str()),
            _ => None,
        };
        match replacement {
            Some(value) => out.push_str(value),
            // Unknown placeholder — pass through as-is.
            None => out.push_str(&rest[start..start + end + 2]),
        }
        rest = &rest[start + end + 2..];
    }
    out.push_str(rest);
    Some(out)
}

fn slugify(input: &str) -> String {
    let lowered = input.to_ascii_lowercase();
    let mut out = String::with_capacity(lowered.len());
    let mut last_dash = true;
    for ch in lowered.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn apply_override(canonical: &str, config: Option<&RoutingConfig>) -> String {
    config
        .and_then(|c| c.tag_overrides.get(canonical))
        .cloned()
        .unwrap_or_else(|| canonical.to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use efofx_domain::{
        AdjustmentFactor, CostCategoryEstimate, EstimationOutput, EstimationSession,
        EstimationSessionId, EstimationStatus, Region, ScopingContext, TenantId,
    };
    use time::OffsetDateTime;

    use super::*;

    fn sample_session(region: Region) -> EstimationSession {
        let now = OffsetDateTime::now_utc();
        EstimationSession {
            id: EstimationSessionId::new(),
            tenant_id: TenantId(uuid::Uuid::new_v4()),
            status: EstimationStatus::Completed,
            description: "test".into(),
            region,
            reference_class: Some("residential_pool".into()),
            confidence_threshold: 70.0,
            prompt_version: Some("v1".into()),
            created_at: now,
            updated_at: now,
            completed_at: Some(now),
            expires_at: None,
        }
    }

    fn sample_output(p50: f64) -> EstimationOutput {
        EstimationOutput {
            total_cost_p50: p50,
            total_cost_p80: p50 * 1.2,
            timeline_weeks_p50: 8,
            timeline_weeks_p80: 10,
            cost_breakdown: vec![CostCategoryEstimate {
                category: "materials".into(),
                p50_cost: p50 * 0.4,
                p80_cost: p50 * 0.5,
                percentage_of_total: 0.4,
            }],
            adjustment_factors: vec![AdjustmentFactor {
                name: "x".into(),
                multiplier: 1.0,
                reason: "y".into(),
            }],
            confidence_score: 75.0,
            assumptions: vec![],
            summary: "z".into(),
        }
    }

    fn sample_scoping() -> ScopingContext {
        ScopingContext {
            project_type: Some("Pool".into()),
            project_size: Some("15x30".into()),
            location: Some("San Diego, CA".into()),
            timeline: Some("3 months".into()),
            special_conditions: None,
        }
    }

    #[test]
    fn cost_tier_buckets_at_breakpoints() {
        // Inclusive lower bound on `mid` and `high`.
        assert_eq!(
            CostTier::from_p50(24_999.0, [25_000, 250_000]),
            CostTier::Low
        );
        assert_eq!(
            CostTier::from_p50(25_000.0, [25_000, 250_000]),
            CostTier::Mid
        );
        assert_eq!(
            CostTier::from_p50(249_999.0, [25_000, 250_000]),
            CostTier::Mid
        );
        assert_eq!(
            CostTier::from_p50(250_000.0, [25_000, 250_000]),
            CostTier::High
        );
        // Custom breakpoints override.
        assert_eq!(
            CostTier::from_p50(15_000.0, [10_000, 100_000]),
            CostTier::Mid
        );
    }

    #[test]
    fn cost_tier_handles_garbage_inputs() {
        assert_eq!(
            CostTier::from_p50(f64::NAN, [25_000, 250_000]),
            CostTier::Low
        );
        assert_eq!(CostTier::from_p50(-100.0, [25_000, 250_000]), CostTier::Low);
        assert_eq!(
            CostTier::from_p50(f64::INFINITY, [25_000, 250_000]),
            CostTier::Low
        );
    }

    #[test]
    fn derive_disabled_tenant_returns_empty_tags_but_keeps_metadata() {
        let session = sample_session(Region::SoCalCoastal);
        let scoping = sample_scoping();
        let output = sample_output(50_000.0);
        let cfg = RoutingConfig {
            enabled: false,
            ..RoutingConfig::default()
        };

        let data = derive(&session, &scoping, &output, Some(&cfg));
        assert!(data.tags.is_empty());
        // Region + cost tier still computed so the contractor-match
        // handler can show them in the response even when routing is
        // off (partners get a deterministic "nothing to route"
        // response rather than empty everything).
        assert_eq!(data.region.as_deref(), Some("SoCal - Coastal"));
        assert_eq!(data.cost_tier, CostTier::Mid);
    }

    #[test]
    fn derive_no_config_treated_as_disabled() {
        let session = sample_session(Region::SoCalCoastal);
        let scoping = sample_scoping();
        let output = sample_output(50_000.0);

        let data = derive(&session, &scoping, &output, None);
        assert!(data.tags.is_empty());
        assert_eq!(data.cost_tier, CostTier::Mid);
    }

    #[test]
    fn derive_full_scoping_emits_all_four_tag_kinds() {
        let session = sample_session(Region::SoCalCoastal);
        let scoping = sample_scoping();
        let output = sample_output(300_000.0);
        let cfg = RoutingConfig {
            enabled: true,
            ..RoutingConfig::default()
        };

        let data = derive(&session, &scoping, &output, Some(&cfg));
        assert_eq!(
            data.tags,
            vec![
                "region:socal-coastal".to_string(),
                "type:pool".to_string(),
                "tier:high".to_string(),
                "location:san-diego-ca".to_string(),
            ]
        );
        assert_eq!(data.cost_tier, CostTier::High);
        assert_eq!(data.project_type.as_deref(), Some("pool"));
    }

    #[test]
    fn derive_applies_tag_overrides() {
        let session = sample_session(Region::NorCalBayArea);
        let scoping = sample_scoping();
        let output = sample_output(40_000.0);
        let mut overrides = BTreeMap::new();
        overrides.insert("pool".into(), "swimming-pool".into());
        let cfg = RoutingConfig {
            enabled: true,
            tag_overrides: overrides,
            ..RoutingConfig::default()
        };

        let data = derive(&session, &scoping, &output, Some(&cfg));
        assert_eq!(data.project_type.as_deref(), Some("swimming-pool"));
        assert!(
            data.tags.iter().any(|t| t == "type:swimming-pool"),
            "tag list missing override: {:?}",
            data.tags
        );
    }

    #[test]
    fn derive_skips_missing_optional_scoping_fields() {
        let session = sample_session(Region::NorCalBayArea);
        let scoping = ScopingContext {
            project_type: None,
            location: None,
            ..ScopingContext::default()
        };
        let output = sample_output(10_000.0);
        let cfg = RoutingConfig {
            enabled: true,
            ..RoutingConfig::default()
        };

        let data = derive(&session, &scoping, &output, Some(&cfg));
        // Region + tier always present when enabled; project_type and
        // location omitted because the scoping fields are missing.
        assert_eq!(
            data.tags,
            vec!["region:norcal-bay-area".to_string(), "tier:low".to_string(),]
        );
    }

    #[test]
    fn render_directory_url_substitutes_known_placeholders() {
        let data = RoutingData {
            tags: vec!["region:socal-coastal".into(), "type:pool".into()],
            region: Some("SoCal - Coastal".into()),
            project_type: Some("pool".into()),
            cost_tier: CostTier::Mid,
            location: Some("San Diego".into()),
        };
        let cfg = RoutingConfig {
            enabled: true,
            directory_url_template: Some(
                "https://partner.example/find?tags={tags}&t={cost_tier}&type={project_type}".into(),
            ),
            ..RoutingConfig::default()
        };
        let url = render_directory_url(&data, Some(&cfg)).unwrap();
        assert_eq!(
            url,
            "https://partner.example/find?tags=region:socal-coastal,type:pool&t=mid&type=pool"
        );
    }

    #[test]
    fn render_directory_url_passes_unknown_placeholders_through() {
        let data = RoutingData {
            tags: vec![],
            region: Some("X".into()),
            project_type: None,
            cost_tier: CostTier::Low,
            location: None,
        };
        let cfg = RoutingConfig {
            enabled: true,
            directory_url_template: Some("https://x/{region}/{unknown_field}".into()),
            ..RoutingConfig::default()
        };
        let url = render_directory_url(&data, Some(&cfg)).unwrap();
        assert_eq!(url, "https://x/X/{unknown_field}");
    }

    #[test]
    fn render_directory_url_preserves_multibyte_utf8() {
        let data = RoutingData {
            tags: vec!["type:pool".into()],
            region: Some("Köln".into()),
            project_type: None,
            cost_tier: CostTier::Low,
            location: None,
        };
        let cfg = RoutingConfig {
            enabled: true,
            directory_url_template: Some("https://partner.example/寿司/{region}?ä={tags}&{nope}".into()),
            ..RoutingConfig::default()
        };
        let url = render_directory_url(&data, Some(&cfg)).unwrap();
        assert_eq!(url, "https://partner.example/寿司/Köln?ä=type:pool&{nope}");
    }

    #[test]
    fn render_directory_url_handles_unbalanced_brace() {
        let data = RoutingData {
            tags: vec![],
            region: None,
            project_type: None,
            cost_tier: CostTier::Low,
            location: None,
        };
        let cfg = RoutingConfig {
            enabled: true,
            directory_url_template: Some("https://x/{tags".into()),
            ..RoutingConfig::default()
        };
        let url = render_directory_url(&data, Some(&cfg)).unwrap();
        assert_eq!(url, "https://x/{tags");
    }

    #[test]
    fn render_directory_url_returns_none_when_template_missing() {
        let data = RoutingData {
            tags: vec![],
            region: None,
            project_type: None,
            cost_tier: CostTier::Low,
            location: None,
        };
        let cfg = RoutingConfig {
            enabled: true,
            ..RoutingConfig::default()
        };
        assert!(render_directory_url(&data, Some(&cfg)).is_none());
        assert!(render_directory_url(&data, None).is_none());
    }
}
