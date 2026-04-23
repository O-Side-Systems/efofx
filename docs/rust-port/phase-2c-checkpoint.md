# Phase 2C Checkpoint — resume instructions

**Date:** 2026-04-23
**Branch:** `rust-port`
**Status:** 3 of 7 subphases committed. Pause point for context reset.

---

## What shipped

| # | Commit | Scope |
|---|---|---|
| 2C.1 | `502ceed` | Reference domain + `ReferenceRepo` |
| 2C.2 | `f3ab78d` | `Cache` trait + `InMemoryCache` |
| 2C.3 | `96c47a7` | RCF engine (`efofx-rcf` crate) |

Run `git log --oneline rust-port ^main` to see all Phase 2 commits.

## What's next (4 subphases left)

| # | Scope | Output |
|---|---|---|
| 2C.4 | `EstimationRepo` + `EstimationSession` domain type | New `estimates` collection, save/get/expire_if_past |
| 2C.5 | `EstimationService::generate_from_chat` | LLM-classify → pull reference projects → `complete_structured` call → persist. Mock-driven tests. |
| 2C.6 | SSE handler on `POST /v1/chat/sessions/{id}:generate-estimate` + `GET /v1/estimates/{id}` | Stream `thinking → estimate → narrative tokens → done`; `error` on any failure. |
| 2C.7 | Seed binary + residential fixtures | `apps/efofx-core/src/bin/seed-references.rs`, JSON fixtures, docs/runbook |

RCF engine integration: **not** on chat→estimate path per FastAPI parity. Available for a future direct-estimate endpoint or to provide baseline hints to the LLM (defer until after 2C.6 lands).

## Design decisions already made (don't re-litigate)

- **Unified `ReferenceClass` schema**: Rust uses the newer FastAPI domain-agnostic schema (cost_distribution, timeline_distribution, cost_breakdown_template, attributes map). The older `app/models/reference.py` schema is not ported.
- **Platform + tenant visibility**: `ReferenceRepo` accepts `Option<&TenantContext>` and folds platform rows (`tenant_id: null`) into tenant results. `None` ctx = platform-only.
- **Cache**: trait in `efofx-cache`. In-memory TTL impl ships; Valkey deferred to Phase 4. RCF matcher uses SHA-256 canonical-JSON keys.
- **Reference data writes**: no admin endpoints in 2C. Seeded via 2C.7 binary; tenant-customization UI lands later.
- **SSE path**: `POST /v1/chat/sessions/{id}:generate-estimate` (resource-oriented), not the FastAPI `/chat/{id}/generate-estimate` shape. Already pinned in `openapi.rs` path list.
- **BYOK**: every LLM call takes the tenant's decrypted key per-call; no fallback to a platform key. Pattern is set in `services/chat.rs`.
- **Structured output**: use `LlmProvider::complete_structured` with a **hand-built** JSON schema for `EstimationOutput` (don't reach for schemars derive; schema needs `additionalProperties: false` and `required` arrays that OpenAI strict mode demands).

## Key files / locations

**Already touched in 2C:**
- `apps/efofx-core/crates/efofx-domain/src/reference.rs` — `ReferenceClass`, `ReferenceProject`, `CostDistribution`, `TimelineDistribution`
- `apps/efofx-core/crates/efofx-storage/src/reference.rs` — `ReferenceRepo`, `decode_reference_class` helper
- `apps/efofx-core/crates/efofx-cache/` — new crate
- `apps/efofx-core/crates/efofx-rcf/` — new crate

**Will need edits in 2C.4–2C.7:**
- `apps/efofx-core/crates/efofx-domain/src/estimation.rs` — add `EstimationSession` next to existing `EstimationOutput`
- `apps/efofx-core/crates/efofx-storage/src/` — new `estimation.rs` with `EstimationRepo`
- `apps/efofx-core/crates/efofx-core/src/services/` — new `estimation.rs` (orchestrator)
- `apps/efofx-core/crates/efofx-core/src/api/chat.rs` — replace `generate_estimate` stub (currently `not_implemented`)
- `apps/efofx-core/crates/efofx-core/src/api/estimation.rs` — wire `get_estimation`
- `apps/efofx-core/crates/efofx-core/src/lib.rs` — wire new services into `AppState` + `build_app_state`
- `apps/efofx-core/src/bin/seed-references.rs` — new binary (2C.7)
- `apps/efofx-core/config/reference-data/*.json` — new fixtures (2C.7)

**FastAPI references to port:**
- `apps/efofx-estimate/app/services/estimation_service.py::EstimationService.generate_from_chat` — orchestration to mirror
- `apps/efofx-estimate/app/services/llm_service.py::LLMService.generate_estimation` — structured-output call
- `apps/efofx-estimate/app/api/routes.py::generate_estimate_stream` — SSE handler shape (event sequence: `thinking → estimate → data (narrative tokens) → done` / `error`)
- `apps/efofx-estimate/app/models/estimation.py::EstimationSession` — Mongo document shape to mirror

**Prompts already in place:**
- `apps/efofx-core/config/prompts/v1.0.0-estimation.json` — estimation system/user prompt
- `apps/efofx-core/config/prompts/v1.1.0-narrative.json` — narrative prompt (numeric placeholders are plain `{foo}` — caller pre-formats dollar/week values into display strings, no Python `:,.0f` specs)

## Build / test commands

```bash
# Rust toolchain is at ~/.cargo/bin (not on default PATH in this shell)
export PATH="$HOME/.cargo/bin:$PATH"
cd apps/efofx-core

cargo build --workspace
cargo test --workspace
cargo fmt --check
cargo clippy --workspace -- -D warnings   # CI-style; do not add --all-targets
```

Pre-existing `--all-targets` clippy warnings exist in `chat.rs` test code (`field_reassign_with_default`) — **not** blocking; they're not introduced by this phase.

## Pending sanity checks before merging 2C

- [ ] Confirm `POST /v1/chat/sessions/{id}:generate-estimate` actually streams (test against a real OpenAI key if possible, or wire a mock stream)
- [ ] Byte-compat check: Rust `EstimationOutput` JSON vs. FastAPI's. Already lined up in 2C.1 domain work, but verify once an LLM call lands.
- [ ] Chat session transitions `ready → completed` on successful estimate (verify in 2C.6)

## Resume prompt template

When resuming in a fresh session, paste this:

> I'm resuming Phase 2C of the Rust port. Read
> `docs/rust-port/phase-2c-checkpoint.md` and
> `docs/RUST-PORT-PLAN.md` (§Phase 2C), then start on **2C.4 —
> EstimationRepo + EstimationSession domain type**. Land one subphase
> per commit, following the cadence of 2B.1–2B.6 and 2C.1–2C.3.
