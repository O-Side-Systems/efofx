//! Smoke-tests that the JSON files actually shipped under
//! `apps/efofx-core/config/prompts/` load cleanly. If a prompt file is
//! malformed, this test fails the build — the server would otherwise only
//! discover the issue at startup.

use efofx_prompts::PromptRegistry;
use std::path::PathBuf;

fn prompts_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR points at `crates/efofx-prompts/`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/ parent")
        .parent()
        .expect("workspace root")
        .join("config")
        .join("prompts")
}

#[test]
fn shipped_prompts_load() {
    let dir = prompts_dir();
    let reg = PromptRegistry::load_from_dir(&dir)
        .unwrap_or_else(|e| panic!("shipped prompts must load from {}: {e}", dir.display()));

    // The three prompts this port carries.
    let scoping = reg.latest("scoping").expect("scoping prompt loaded");
    assert_eq!(scoping.version, "1.0.0");

    let estimation = reg.latest("estimation").expect("estimation prompt loaded");
    assert_eq!(estimation.version, "1.0.0");

    let narrative = reg.latest("narrative").expect("narrative prompt loaded");
    assert_eq!(narrative.version, "1.1.0");
}

#[test]
fn scoping_user_template_renders_with_phase_2b_vars() {
    let reg = PromptRegistry::load_from_dir(prompts_dir()).unwrap();
    let scoping = reg.latest("scoping").unwrap();
    // These are the three placeholders ChatService will supply.
    let rendered = scoping
        .render_user(&[
            ("conversation_history", "User: Hi\nAssistant: Hello"),
            ("user_message", "I want to build a pool"),
            ("scoping_context", "{\"project_type\":\"pool\"}"),
        ])
        .expect("scoping prompt renders with 2B placeholders");
    assert!(rendered.contains("pool"));
    assert!(rendered.contains("Hi"));
}

#[test]
fn narrative_template_renders_with_preformatted_numerics() {
    let reg = PromptRegistry::load_from_dir(prompts_dir()).unwrap();
    let narrative = reg.latest("narrative").unwrap();
    // Placeholders use plain names; caller pre-formats numbers.
    let rendered = narrative
        .render_user(&[
            ("project_description", "A 15x30 pool"),
            ("total_cost_p50", "$42,000"),
            ("total_cost_p80", "$58,000"),
            ("timeline_weeks_p50", "6"),
            ("timeline_weeks_p80", "10"),
            ("cost_breakdown", "- Excavation: $8,000"),
            ("adjustment_factors", "- Region: SoCal"),
            ("assumptions", "- Permit granted on time"),
        ])
        .expect("narrative prompt renders with preformatted values");
    assert!(rendered.contains("$42,000"));
    assert!(rendered.contains("$58,000"));
    assert!(rendered.contains("6 weeks"));
}
