# Phase 2F Checkpoint — Partner Routing

**Date:** 2026-04-26
**Branch:** `rust-port`
**Status:** All 4 subphases committed. Phase 2F closed.
**Companion:** [phase-2f-plan.md](./phase-2f-plan.md), [phase-2e-checkpoint.md](./phase-2e-checkpoint.md), [TADR §10.4](../efofx_platform_core_tadr.md)

---

## Goal

Promote contractor / partner routing from "ad hoc add-on" to a
first-class platform surface (TADR §10.4). Tenants opt in by
configuring a routing target in their settings; completed estimation
sessions emit `routing_tags`; partner directories call a single
documented endpoint to get structured filter data and a redirect URL.

This phase was net-new — there was no FastAPI implementation to
preserve. The endpoint stub already declared its OpenAPI shape; 2F
filled in the service layer and added `routing_tags` to the SSE event
payload.

## What shipped

| # | Commit | Scope |
|---|---|---|
| 2F.1 | `25a4ead` | `RoutingConfig` in `TenantSettingsDoc` + `fetch_routing_config` |
| 2F.2 | `b46db49` | `routing_tags` on the estimate `done` SSE event + `RoutingService` (tag derivation + URL templating) |
| 2F.3 | _this commit_ | `POST /v1/integration/contractor-match` real handler + integration router wired under `widget_api_key` |
| 2F.4 | _this commit_ | Integration auth contract tests + `openapi_integration_surface_contract` snapshot + checkpoint doc |

Run `git log --oneline rust-port ^main` to see all Phase 2 commits.

2F.3 and 2F.4 ship as a single commit — 2F.4's only deliverables are
the contract test file, the OpenAPI snapshot test, and this
checkpoint, none of which stand alone without the 2F.3 handler.

## Surface summary

| Verb | Path | Auth | Notes |
|---|---|---|---|
| POST | `/v1/integration/contractor-match` | widget_api_key | Validated request → tags, region, cost tier, optional `directory_url` |
| (event) | estimate `done` SSE event | n/a | `routing_tags: Vec<String>` field added (always present, empty when disabled) |

## Decisions locked during 2F

1. **No partner directory of our own.** We emit tags + an opaque
   `directory_url`. The partner site owns presentation. Cheap to add
   per partner, no UX surface for us to maintain.
2. **Tag namespace** — colon-prefixed (`region:`, `tier:`, `type:`,
   `location:`). Easier for partners to filter and easier for us to
   evolve without colliding with custom tags.
3. **Cost-tier breakpoints in config, not code** — defaults `25_000`
   and `250_000` (inclusive lower bounds for `mid` / `high`). Custom
   `cost_tier_breakpoints: [u64; 2]` per tenant overrides.
4. **`routing_tags` always present** on the SSE `done` event, even
   when routing is disabled (empty array). Stable wire shape beats a
   conditional field.
5. **Endpoint auth is `widget_api_key`.** Partners call this
   server-to-server with the tenant's widget API key. No new auth
   scheme; no public unauthenticated variant.
6. **Template substitution is `{name}` literal.** Hand-rolled
   single-pass scanner over the template — no mustache, tera, or
   askama. Recognised placeholders: `{tags}` (comma-joined),
   `{region}`, `{project_type}`, `{cost_tier}`. Unknown placeholders
   pass through untouched (additive forward-compat).
7. **`directory_url: Option<String>`** rather than `String` because
   tenants without a `directory_url_template` get `None`. Lets the
   partner distinguish "no preferred destination" from "empty URL".
8. **Read-only for v1.** Tenants seed `RoutingConfig` via direct
   Mongo write. The `PATCH /v1/me` write path can land later — same
   pattern as branding + allowed_origins.
9. **Synthetic zero-cost output in the contractor-match handler.**
   `EstimationOutput` is not persisted alongside the session in the
   current schema (existing gap, see `api/estimation.rs:71`). Until
   that gap closes, the handler buckets cost tier as `Low`
   deterministically by passing a zero-cost synthetic
   `EstimationOutput` to `routing::derive`. The wire shape stays
   stable; partners can still filter on `region:*`, `type:*`, and
   `location:*`.
10. **No estimation→chat-session FK.** The plan permits chat-session
    lookup to fail and fall through with empty scoping. Since no FK
    exists, the handler skips the lookup entirely — request overrides
    are the only `project_type` / `location` signal. Region still
    drives the `region:*` tag from the loaded `EstimationSession`.
11. **Override length cap = 64 chars.** `project_type` and `location`
    overrides each cap at 64 chars (UTF-8 codepoint count). Slugified
    by the same `slugify` helper that handles session region.
12. **Disabled tenant returns 200, not 404.** Partners get a
    deterministic "nothing to route" response (`tags: []`, region
    populated, `directory_url: None`) — they can still log the
    inbound call.

## Files added in 2F

- `crates/efofx-core/src/services/routing.rs` — `RoutingService` (pure
  derivation), `CostTier`, `RoutingData`, `derive`,
  `render_directory_url`, plus `slugify` / `apply_override` helpers
- `crates/efofx-core/tests/integration_contractor_match.rs` — auth
  rejection contract tests for the new endpoint

## Files modified in 2F

- `crates/efofx-storage/src/tenants.rs` — `RoutingConfig` struct +
  `TenantSettingsDoc.routing` field + `fetch_routing_config` query
- `crates/efofx-storage/src/lib.rs` — re-export `RoutingConfig`
- `crates/efofx-core/src/services/mod.rs` — export `derive_routing`,
  `render_directory_url`, `CostTier`, `RoutingData`
- `crates/efofx-core/src/services/estimation.rs` — `EstimationOutcome`
  surfaces the chat session's `scoping_context` so the SSE handler
  can derive tags without a re-fetch
- `crates/efofx-core/src/api/chat.rs` — SSE `done` event includes
  `routing_tags` field
- `crates/efofx-core/src/api/integration.rs` — handler replaces 501
  stub; routes wrap `widget_api_key` middleware
- `crates/efofx-core/src/api/mod.rs` — pass `state` to
  `integration::routes`
- `crates/efofx-core/src/openapi.rs` — drop the 501 response from the
  `contractor_match` annotation, add 400; new
  `openapi_integration_surface_contract` snapshot test

## Test coverage

| Suite | Coverage |
|---|---|
| `services::routing::tests` | Cost-tier bucketing at breakpoint boundaries, NaN/negative/infinite garbage inputs, disabled-tenant empty tags but populated metadata, `None` config treated as disabled, full-scoping all four tag kinds, `tag_overrides` apply, missing optional scoping fields, URL substitution (known placeholders, unknown pass-through, no template → `None`) |
| `tenants::tests` | `RoutingConfig` default disabled, explicit breakpoints win, full round-trip through Mongo BSON, omits empty optional fields on serialize, legacy tenants without `settings.routing` deserialize without error |
| `integration_contractor_match.rs` | POST without API key → 401 envelope, POST with bogus widget key → 401, malformed JSON without auth → 401 (auth runs before parse) |
| `openapi::tests::openapi_integration_surface_contract` | Pinned status codes (200/400/401/404, **not** 501) + `widget_api_key` auth scheme + required `ContractorMatchRequest` / `ContractorMatchResponse` components |
| `openapi::tests::openapi_contains_expected_paths` | Pins `/v1/integration/contractor-match` |

Live happy-path coverage (real Mongo, real session load, real tag
derivation, real URL substitution) defers to Phase 4. The
`TestHarness` fake Mongo handle errors on every query, which is
exactly what the contract tests need to verify auth + validation run
before storage.

## Definition of Done — status

- [x] `POST /v1/integration/contractor-match` returns its real
      response (not 501)
- [x] `routing_tags` field present on the SSE `done` event
- [x] `RoutingConfig` round-trips through Mongo (read path verified
      by unit tests; write path lands with `PATCH /v1/me` extension
      later)
- [x] `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`,
      `cargo test --workspace` all green
- [x] OpenAPI snapshot test covers the integration surface
- [x] Checkpoint doc written and committed
- [x] Jeff has a documented endpoint to integrate against; no
      blocker on the demo cutover for routing
- [ ] **Live integration test** against real Mongo + a seeded tenant
      with a configured `RoutingConfig` (deferred to Phase 4)

## Follow-ups queued for later phases

- **Persist `EstimationOutput` alongside `EstimationSession`.** The
  largest known gap in 2F: cost tier in the contractor-match response
  is always `low` until output is stored. Phase 4 schema work should
  add an `output` subdoc (or sibling collection) so `get_session`
  surfaces the cost figures the routing layer needs. Same gap blocks
  the `estimate_snapshot` field in `FeedbackDocument` from holding
  real values — already documented in `api/feedback.rs`.
- **`PATCH /v1/me` extension for `RoutingConfig`.** Tenants currently
  seed the config via direct Mongo write. Wire a settings-write path
  alongside branding + allowed_origins.
- **Estimation → chat-session linkage.** Adding a `chat_session_id`
  field to `EstimationSession` would let the contractor-match handler
  recover the customer-supplied scoping context (project_type,
  location) without depending on partner-supplied request overrides.
  Currently a soft gap because overrides cover the same surface; hard
  gap if partners want zero-config integration.
- **Per-tenant tag-namespace versioning.** If we ever rename a tag
  prefix (e.g. `type:` → `category:`), partners with hard-coded
  filters break. A future major-version field on `RoutingConfig`
  could pin the wire form per tenant.
- **OpenAPI byte-compat check vs. FastAPI-generated doc** — carry-over
  from earlier checkpoints, still open.
- **Pre-existing clippy nits under `--all-targets`** —
  `field_reassign_with_default` in `efofx-storage/src/chat.rs:503` and
  `bool_assert_comparison` in `efofx-storage/src/tenants.rs:580`.
  Surface only under `--all-targets`; not gated by
  `cargo clippy --workspace -- -D warnings`. Carry-over from earlier
  phases.
- **Axum 0.8 upgrade** to remove the `:name` vs `{name}` divergence
  (carry-over from Phase 2D follow-ups).
