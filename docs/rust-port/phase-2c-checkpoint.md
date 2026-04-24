# Phase 2C Checkpoint — resume instructions

**Date:** 2026-04-23 (refreshed 2026-04-24 after 2C.4–2C.7)
**Branch:** `rust-port`
**Status:** All 7 subphases committed. Phase 2C closed.

---

## What shipped

| # | Commit | Scope |
|---|---|---|
| 2C.1 | `502ceed` | Reference domain + `ReferenceRepo` |
| 2C.2 | `f3ab78d` | `Cache` trait + `InMemoryCache` |
| 2C.3 | `96c47a7` | RCF engine (`efofx-rcf` crate) |
| 2C.4 | `8b51172` | `EstimationRepo` + `EstimationSession` domain type |
| 2C.5 | `013666e` | `EstimationService::generate_from_chat` |
| 2C.6 | `4dc9895` | `POST :generate-estimate` SSE + `GET /v1/estimates/{id}` |
| 2C.7 | _this commit_ | `seed-references` bin + residential fixtures + runbook |

Run `git log --oneline rust-port ^main` to see all Phase 2 commits.

RCF engine integration: **not** on chat→estimate path per FastAPI parity. Available for a future direct-estimate endpoint or to provide baseline hints to the LLM.

## Follow-ups queued for later phases

- Byte-compat check of Rust `EstimationOutput` JSON vs. FastAPI
- Live end-to-end SSE verification against a real OpenAI key (currently covered by mock-driven tests only)
- Tenant-scoped reference-class CRUD surface (deferred until the first tenant-customization UI)

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

**Landed in 2C.4–2C.7:**
- `apps/efofx-core/crates/efofx-domain/src/estimation.rs` — `EstimationSession` alongside `EstimationOutput`
- `apps/efofx-core/crates/efofx-storage/src/estimation.rs` — `EstimationRepo`
- `apps/efofx-core/crates/efofx-core/src/services/estimation.rs` — orchestrator
- `apps/efofx-core/crates/efofx-core/src/api/chat.rs` — SSE `generate_estimate` wired
- `apps/efofx-core/crates/efofx-core/src/api/estimation.rs` — `GET /v1/estimates/{id}`
- `apps/efofx-core/crates/efofx-core/src/lib.rs` — services wired through `AppState`
- `apps/efofx-core/crates/efofx-core/src/bin/seed-references.rs` — 2C.7 binary (note: lives inside the core crate, not a top-level `src/bin/`)
- `apps/efofx-core/config/reference-data/*.json` — residential fixtures (2C.7)
- `apps/efofx-core/crates/efofx-storage/src/reference.rs` — `upsert_platform_*` + `delete_platform_*` added for the seeder (2C.7)

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

## Seeding the reference catalog

After a fresh deployment (including local dev) platform reference data
is empty — chat→estimate will return placeholder results until it's
populated. Run:

```bash
EFOFX_MONGO__URI=... \
EFOFX_SUPABASE__URL=... EFOFX_SUPABASE__ISSUER=... \
cargo run -p efofx-core --bin seed-references -- --dry-run
cargo run -p efofx-core --bin seed-references
```

See `docs/rust-port/seeding-runbook.md` for the full runbook.
