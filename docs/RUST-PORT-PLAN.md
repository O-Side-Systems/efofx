# Efofx Rust Port — Phased Implementation Plan

**Last Updated:** 2026-04-22
**Companion to:** [efofx_platform_core_tadr.md](./efofx_platform_core_tadr.md)
**Branch:** `rust-port`
**Goal:** Migrate authoritative backend logic from `apps/efofx-estimate` (FastAPI) to a new Rust platform core (`apps/efofx-core`), per the approved TADR.

---

## Operating Rules

1. **The FastAPI backend stays running** until Rust parity is reached for each surface. No client is repointed before its Rust endpoints are contract-verified.
2. **OpenAPI-first.** Every endpoint lands in `openapi/efofx-core.yaml` *before* the handler is merged. Contract tests run in CI.
3. **No new features in FastAPI.** Bug fixes only. All new work lands in Rust.
4. **Tenant context is a required type, not a convention.** Repositories reject calls without it at compile time.
5. **No magic strings.** Every constant is an enum, a config key, or a documented default with a comment.
6. **Atomic commits.** One bounded area or one endpoint group per PR where practical.

---

## Phase Overview

| Phase | Name | Scope | Depends On |
|------:|------|-------|------------|
| 0 | Foundations | Cargo workspace, Axum skeleton, config, tracing, Mongo adapter, tenant-aware repo pattern, health + OpenAPI pipeline | — |
| 1 | Contracts & Boundaries | OpenAPI for all existing endpoints; constant classification (enum/config/default); domain type crate | 0 |
| 2A | Auth & Tenant Core | Register / verify / login / refresh / profile / BYOK endpoints in Rust | 0, 1 |
| 2B | Chat & Scoping | `/chat/send`, session state machine, scoping extraction, LLM follow-ups | 0, 1, 2A |
| 2C | Estimation & RCF | `/generate-estimate` SSE flow, reference-class engine, structured output parsing, narrative streaming | 2B |
| 2D | Widget Public Endpoints | Branding (public), lead capture, consultation, analytics | 2A, 2C |
| 2E | Feedback & Calibration | Feedback submit/summary, calibration aggregation | 2D |
| 2F | Partner Routing | Contractor-match integration endpoint, routing tags on estimate | 2C |
| 3 | Client Cutover | Repoint widget, dashboard, and partners to Rust APIs; remove duplicated logic | 2A–2F as needed |
| 4 | Hardening | Tracing dashboards, chat length/token budget policies, error envelope polish, rate-limit config, deployment runbook | 3 |
| 5 | Decommission | Archive `apps/efofx-estimate`, promote Rust docs to source of truth | 4 |

Phases 2A–2F can interleave once their dependencies are green; dashboard-only surfaces (2E) can slip behind the demo path (2A→2B→2C→2D).

---

## Demo-Path Priority

Jeff's demo needs, in order, a reliable: **widget branding fetch → auth → chat → estimate SSE → lead capture**. That maps to **Phases 0 → 1 → 2A → 2B → 2C → 2D (branding + lead only)**. Everything else (feedback, calibration, routing, dashboard features) can land after the demo is stable.

---

## Phase 0 — Foundations

**Objective:** A running Axum service with config, tracing, a Mongo connection, a compile-time-enforced tenant-aware repository pattern, and OpenAPI scaffolding. No business endpoints yet.

### Tasks

- **0.1 Cargo workspace.** Create `apps/efofx-core/` with a virtual workspace:
  - `crates/efofx-core` — binary (Axum server)
  - `crates/efofx-domain` — pure domain types, enums, value objects (no I/O)
  - `crates/efofx-config` — layered config loader (env → file → defaults) with validation
  - `crates/efofx-storage` — Mongo adapter + `TenantContext` guard + repository traits
  - `crates/efofx-openapi` — OpenAPI builder + served `/openapi.json` + contract-test harness
- **0.2 Runtime stack.** Axum 0.7+, Tokio, `tower-http` (trace, cors, request-id), `tracing` + `tracing-subscriber` JSON output, `serde`, `thiserror`, `anyhow` in main only.
- **0.3 Config loader.** One `AppConfig` struct. Sources in precedence: env vars, `config/efofx-core.toml`, defaults. No config read outside this crate. Validate on boot, fail fast.
- **0.4 Tenant-aware repo.** `TenantContext` newtype with a private constructor obtained only via the auth middleware. Every tenant-scoped repo method requires `&TenantContext` in its signature. Add a `#[deny]` lint and a compile-fail test that tries to call a tenant repo without it.
- **0.5 Mongo adapter.** Thin wrapper around `mongodb` crate; expose collection handles only through repository structs.
- **0.6 Observability.** Request ID middleware, `tracing` spans per request, JSON logs, SSE-span propagation helper.
- **0.7 Health + OpenAPI endpoints.** `GET /health`, `GET /openapi.json`. Health checks Mongo reachability.
- **0.8 Error envelope.** One `ApiError` → consistent `{errors:[{code,message,meta}]}` JSON. Implement `IntoResponse`.
- **0.9 CI.** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo build --release`. Lint the OpenAPI file (`redocly lint` or equivalent).

### Definition of Done

- `cargo run -p efofx-core` starts, logs a JSON line per request, answers `GET /health` with Mongo status, serves `/openapi.json`.
- Attempting to call a tenant repo without `TenantContext` fails to compile (covered by a `tests/compile-fail/` case).
- CI green on `rust-port`.

---

## Phase 1 — Contracts & Boundaries

**Objective:** The shape of the Rust API is defined before handlers are written. Constants are classified.

### Tasks

- **1.1 Workflow inventory.** Produce `docs/rust-port/workflow-inventory.md` listing every FastAPI route, its request/response shapes, auth requirements, side effects, and target Rust module.
- **1.2 Draft OpenAPI.** Populate `openapi/efofx-core.yaml` with all retained endpoints (paths, schemas, auth schemes, error envelope). No implementation yet — contract first.
- **1.3 Constant classification.** Walk `apps/efofx-estimate/app/core/constants.py` and scattered literals. Produce `docs/rust-port/constants-classification.md` with three columns: **Enum** (stable, type-safe), **Config** (runtime-variable), **Default** (documented, rarely changed). Move each literal into the matching bucket in `efofx-domain` or `efofx-config`.
- **1.4 Domain types.** In `efofx-domain`, define: `TenantId`, `SessionId`, `ChatStatus`, `ScopingContext`, `EstimationOutput`, `CostBreakdown`, `AdjustmentFactor`, `Region`, `ReferenceClass`, `LeadStatus`. Derive `serde::{Serialize,Deserialize}` and `schemars::JsonSchema` so OpenAPI schemas are generated, not duplicated.
- **1.5 Response envelopes.** Encode sync `{data, metadata, errors}` and SSE `{event, payload, metadata?}` as reusable types.

### Definition of Done

- `openapi/efofx-core.yaml` lints clean and covers every route that will exist after migration.
- Every Python literal that survived has a home in an enum, config key, or documented default.
- Domain types compile and serialize/deserialize round-trip in tests.

---

## Phase 2A — Identity, Tenant Provisioning, BYOK

**Objective:** Rust verifies Supabase JWTs, provisions tenants on first-seen user, authenticates widget API keys, and owns BYOK storage. No password handling in our codebase.

**Auth decision:** see TADR §6.4. Supabase handles registration, login, email verify, password reset, session management. Rust does JWT verification + tenant mapping only.

### Tasks

- **2A.1 Supabase JWT verification middleware.**
  - Add `SUPABASE_URL` + `SUPABASE_JWT_AUDIENCE` to config (see Config Inventory).
  - On startup, fetch `{SUPABASE_URL}/auth/v1/.well-known/jwks.json` and cache with a periodic refresh (15 min TTL). Crate: `jsonwebtoken` (verification) + `reqwest` (fetch).
  - Middleware: parse `Authorization: Bearer <jwt>`, verify signature against JWKS, verify `aud`, `iss`, `exp`, `nbf`. On success, extract `sub` (Supabase user_id) and `email` claim.
  - Any unrecoverable token error → `401` with the common error envelope.
- **2A.2 Tenant provisioning (`IdentityService`).**
  - On a verified JWT, look up `Tenant` by `supabase_user_id`. If absent, provision: `{ tenant_id: Uuid::new_v4(), supabase_user_id, email, company_name: claims.user_metadata.company_name.unwrap_or(email), tier: "trial", created_at: now, ... }`.
  - Company name is captured at Supabase signup via user_metadata; falls back to email if missing.
  - Emit a structured `tenant.provisioned` log event on first provisioning.
- **2A.3 Widget API-key auth middleware.**
  - Parse `Authorization: Bearer sk_live_...` or `X-Api-Key` header (widget convention — confirm when porting widget).
  - Hash the key (same scheme as FastAPI: bcrypt of the raw key's suffix — or redesign to a constant-time-comparable HMAC if bcrypt-per-request becomes a bottleneck; document the choice).
  - Look up tenant by `api_key_last6` index, verify full hash matches, construct `TenantContext`.
  - **Key rotation path:** `POST /v1/me/api-keys` (rotate — new key issued once, old invalidated) lives in Rust since it has no Supabase analog.
- **2A.4 `TenantContext` unification.**
  - Both middlewares mint the same `TenantContext` type. Downstream handlers don't know which surface authenticated the request. Private constructor; only these two middlewares can build it.
- **2A.5 Tenant profile endpoints (Rust-owned).**
  - `GET /v1/me` — returns the authenticated tenant's profile (merged from Supabase user + local tenant record).
  - `PATCH /v1/me` — updates tenant settings (company_name, branding, allowed_origins, routing config). Does **not** touch email/password — those live in Supabase.
- **2A.6 BYOK endpoints.**
  - `PUT /v1/me/openai-key`, `GET /v1/me/openai-key/status`, `DELETE /v1/me/openai-key`.
  - Port HKDF-SHA256 + Fernet from `packages/efofx-shared` using `hkdf` + `fernet` crates. Preserve master-key → per-tenant-key derivation so any existing ciphertexts decrypt (moot for clean slate, but preserves portability).
- **2A.7 Dashboard integration sample.**
  - Short guide in `docs/rust-port/dashboard-auth.md` showing how `apps/efofx-dashboard` uses `@supabase/supabase-js` to sign in and attach the JWT to Rust API calls. No code changes to dashboard in Phase 2A — just the guide so cutover in Phase 3 is unblocked.
- **2A.8 Contract tests.**
  - OpenAPI drives request/response shape tests. A mock JWKS provider in tests issues valid/invalid tokens for verification coverage.

### Definition of Done

- A request with a valid Supabase JWT provisions a tenant on first call and returns that tenant's profile on subsequent calls.
- A request with a valid tenant API key returns the same tenant's profile.
- BYOK store → status → delete cycle works.
- Invalid JWT, expired JWT, wrong audience, wrong issuer, missing signature — all produce the standard 401 error envelope.
- No password hashing, email verification, or session logic exists in our codebase.

---

## Phase 2B — Chat & Scoping

**Objective:** The multi-turn intake conversation runs on Rust.

### Tasks

- **2B.1** Port prompt registry — versioned JSON files under `apps/efofx-core/config/prompts/`, loaded at startup, fail-fast if missing.
- **2B.2** `LlmProvider` trait + `OpenAiProvider` impl using `async-openai`. Streaming, structured output (function calling / response_format), retries, timeouts — all behavior configurable.
- **2B.3** `ChatService` — session state machine (`active → ready → completed|expired`), readiness evaluation (4 scoping fields OR trigger phrases), LLM follow-up generation.
- **2B.4** `ScopingExtractor` — pulls `project_type`, `size`, `location`, `timeline`, `special_conditions` from conversation state.
- **2B.5** `POST /api/v1/chat/send`, `GET /api/v1/chat/{session_id}/history`.
- **2B.6** Chat-length and per-session token budget enforcement (config-driven, TADR §11.2).

### Definition of Done

- A chat session started in Rust transitions `active → ready` when the four scoping fields are populated.
- Length and token budgets reject further messages with a clean error code.
- Contract tests pass.

---

## Phase 2C — Estimation & Reference Classes

**Objective:** The differentiator — estimation pipeline with RCF, structured output, and SSE narrative — runs on Rust.

### Tasks

- **2C.1** `ReferenceService` + `RCFEngine` — keyword extraction, scoring, cached lookups (Valkey adapter behind a `Cache` trait).
- **2C.2** Structured estimation generation — parse into `EstimationOutput` (P50/P80, breakdown, adjustments, confidence, assumptions, summary).
- **2C.3** `POST /api/v1/chat/{session_id}/generate-estimate` SSE endpoint:
  1. `thinking` → 2. retrieve + validate session = `ready` → 3. generate structured estimate → 4. emit `estimate` event → 5. stream `narrative` tokens → 6. mark session completed → 7. emit `done`.
- **2C.4** `GET /api/v1/estimate/{session_id}` (status).
- **2C.5** SSE error event with typed error codes; reconnection semantics documented.

### Definition of Done

- Full `chat → ready → generate-estimate` flow runs end-to-end on Rust against a test tenant.
- Output JSON is byte-compatible with FastAPI's `EstimationOutput` schema (verified by contract test).
- Streaming works through `tower-http` trace without dropping events.

---

## Phase 2D — Widget Public Endpoints

**Objective:** Widget can run entirely against Rust (public branding + authenticated flows).

### Tasks

- **2D.1** `GET /api/v1/widget/branding/{api_key_prefix}` — public, rate-limited 30/min, no auth.
- **2D.2** `POST /api/v1/widget/lead` — lead capture.
- **2D.3** `POST /api/v1/widget/consultation` — consultation request + email notification (email adapter trait; Resend impl).
- **2D.4** `POST|GET /api/v1/widget/analytics` — event recording.
- **2D.5** CORS policy: per-tenant `allowed_origins` middleware.

### Definition of Done

- Widget pointed at Rust completes the full demo flow (branding → chat → estimate → lead) without errors.

---

## Phase 2E — Feedback & Calibration

**Objective:** Feedback and calibration endpoints move to Rust. Calibration aggregation is designed to be queue-friendly for later (TADR §11.3).

### Tasks

- **2E.1** `POST /api/v1/feedback/submit`, `GET /api/v1/feedback/summary`.
- **2E.2** Magic-link flow for feedback (reuse email adapter).
- **2E.3** `GET /api/v1/calibration/*` — metrics, trends, threshold progress.

### Definition of Done

- Dashboard calibration views work against Rust with no regressions.

---

## Phase 2F — Partner Routing

**Objective:** Contractor routing is a first-class platform concern (TADR §10.4).

### Tasks

- **2F.1** Routing config in tenant settings (enabled, base URL template, tag mapping).
- **2F.2** `routing_tags` included on estimate `done` event.
- **2F.3** `POST /api/v1/integration/contractor-match` — structured input, structured output for partner sites.

### Definition of Done

- Partner integration can consume a single documented endpoint and receive routing-ready data.

---

## Phase 3 — Client Cutover

### Tasks

- **3.1** Repoint `apps/efofx-widget` to Rust base URL via config. Run full end-to-end against Rust.
- **3.2** Repoint `apps/efofx-dashboard`. Remove any client-side logic duplicating server rules.
- **3.3** Update Jeff's integration docs to point at Rust endpoints.
- **3.4** Keep FastAPI running on a separate port as a fallback for 1–2 weeks.

### Definition of Done

- All production traffic flows to Rust.
- No client code replicates server workflow rules.

---

## Phase 4 — Hardening

### Tasks

- **4.1** Tracing: add a Grafana/Loki (or equivalent) dashboard. SSE traces end-to-end.
- **4.2** Rate limit config finalized per tenant tier. Chat length + token budget policies exercised with load tests.
- **4.3** Error envelope: audit every non-2xx response for code + message consistency.
- **4.4** Deployment runbook in `docs/rust-port/deployment.md` (staging → prod, rollback steps, env matrix).
- **4.5** Sentry (or replacement) wired to `tracing` error events.

### Definition of Done

- On-call can diagnose any request by correlation ID in under two minutes.
- A rollback is documented and has been rehearsed in staging.

---

## Phase 5 — Decommission

### Tasks

- **5.1** Move `apps/efofx-estimate` to `apps/archive/efofx-estimate-fastapi/` with a `README.md` pointing to `apps/efofx-core`.
- **5.2** Update `docs/CODEBASE-STATE.md` to reflect Rust as source of truth.
- **5.3** Archive FastAPI-specific docs; keep historical PRD and pivot analysis untouched.
- **5.4** Remove FastAPI from CI.

### Definition of Done

- `apps/efofx-core` is the only backend referenced in docs and CI.
- FastAPI archive is clearly marked non-authoritative.

---

## Cross-Phase Concerns

### Configuration Inventory (populated during Phase 1)

Minimum keys expected:

- `server.{host,port}`
- `mongo.{uri,db_name}`
- `cache.{url,ttl_seconds}`
- `supabase.{url,jwt_audience,jwks_refresh_seconds}` (human-auth: verify-only; no service_role key in server)
- `crypto.master_key` (BYOK encryption root)
- `llm.{provider,model,fallback_order,request_timeout_ms,streaming_enabled}`
- `llm.budgets.{per_tenant_tokens,per_session_tokens,per_session_messages}`
- `rate_limits.{tier,endpoint_group → rpm}`
- `widget.{default_branding,allowed_origins_default}`
- `email.{provider,from_address,resend_api_key}`
- `features.{contractor_routing,platform_key_fallback}`
- `retention.{chat_days,lead_days,analytics_days}`

### Resolved Decisions (2026-04-22)

1. **Auth:** Supabase for human users (JWT via JWKS), tenant API keys for widget. No password storage in our code. See TADR §6.4 and Phase 2A.
2. **Supabase scope:** auth-only. MongoDB stays as system of record.
3. **Tenant mapping:** 1:1 with Supabase users for v1.
4. **Existing users:** clean slate — pre-alpha test accounts discarded, no migration.
5. **BYOK crypto:** keep HKDF-SHA256 + Fernet in application code (not Supabase Vault).
6. **Crate location:** `apps/efofx-core/` Cargo workspace.
7. **OpenAPI generator:** `utoipa` with derive macros.
8. **Demo urgency:** Jeff demo on hold pending his requirements.

### Open Questions Still to Resolve

1. **MongoDB driver.** Stick with the official `mongodb` crate (default) vs. `bson` + lower-level async. Lean: official.
2. **API-key storage scheme.** FastAPI uses bcrypt-hashed keys. Bcrypt-per-request is slow. Options: (a) keep bcrypt and accept cost, (b) switch to HMAC-SHA256 with a server-side pepper — constant-time, O(1). Lean: (b) — widget traffic is hot-path.
3. **Rate limiting storage.** In-memory (single instance), Redis/Valkey (shared across instances), or both behind a trait? Depends on deployment topology — ask Brett when we know where this runs.

### Immediate Next Step

**Install Rust toolchain** (`rustup` → stable). `cargo` is not yet on this machine. Once installed, begin Phase 0.1 (Cargo workspace scaffold).

### Supabase Setup Prerequisites (can happen in parallel with Rust install)

- [ ] Confirm Supabase project settings → Auth → URL configuration includes eventual dashboard origin
- [ ] Confirm email auth provider is enabled; disable anonymous sign-in
- [ ] Add a signup trigger or form field for `company_name` → stored in `user_metadata`
- [ ] Capture the Supabase JWT **audience** value (typically `authenticated`) and **issuer** URL from the project dashboard — needed for Rust verification config
- [ ] Create one or two test accounts (see "Test Accounts" below) once Phase 2A ships so Brett and Jeff can exercise the auth flow

### Test Accounts to Create in Supabase

Create these after Phase 2A ships, not before (we want to exercise the real provisioning path on first login):

| Email | user_metadata.company_name | Purpose |
|-------|----------------------------|---------|
| `brett+dev@oside.systems` | `efofx Dev` | Brett's local/staging testing |
| `jeff@<domain-from-jeff>` | `<Jeff's company>` | Demo account once requirements land |
| `demo@efofx.ai` | `efofx Demo` | Public marketing-page demo tenant (if that's the flow) |

Each signs up through the dashboard UI (once that page exists) or via Supabase dashboard admin invite. Their first authenticated call to the Rust API provisions the tenant in MongoDB — no extra setup.
