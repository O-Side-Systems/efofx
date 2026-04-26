# Phase 2F — Partner Routing: Plan

**Date:** 2026-04-25
**Branch:** `rust-port`
**Status:** Plan locked, implementation starting with 2F.1.
**Companion:** [phase-2e-checkpoint.md](./phase-2e-checkpoint.md), [RUST-PORT-PLAN.md §Phase 2F](../RUST-PORT-PLAN.md), [TADR §10.4](../efofx_platform_core_tadr.md)

---

## Goal

Promote contractor / partner routing from "ad hoc add-on" to a
first-class platform surface (TADR §10.4). Tenants opt in by
configuring a routing target in their settings; completed estimation
sessions emit `routing_tags`; partner directories call a single
documented endpoint to get structured filter data and a redirect URL.

This phase is **net-new** — there is no FastAPI implementation to
preserve. The endpoint stub already exists at
`crates/efofx-core/src/api/integration.rs::contractor_match` (currently
501); 2F replaces the stub and wires the supporting config + event
plumbing. Demo cutover (Phase 3) is the trigger — Jeff's contractor
site needs a stable target.

## Surface map (post-2F)

| Verb | Path | Auth | Notes |
|---|---|---|---|
| POST | `/v1/integration/contractor-match` | widget_api_key | Structured input → structured output (tags, region, cost tier, `directory_url`) |
| (event) | estimate `done` SSE event | n/a | `routing_tags` field added alongside the existing payload |

The endpoint already declares its OpenAPI shape
(`ContractorMatchRequest`, `ContractorMatchResponse`); 2F fills in the
service layer and adds `routing_tags` to the SSE event payload.

## Subphase breakdown

### 2F.1 — Routing config in tenant settings

Extend `TenantSettingsDoc` (in `crates/efofx-storage/src/tenants.rs`)
with a single optional sub-document:

* `RoutingConfig`
  * `enabled: bool` (default `false`)
  * `directory_url_template: String` — URL with `{tag1,tag2,...}` style
    placeholders. The template owns its own query/path syntax; we just
    fill the tag list. Examples: `https://partner.example/find?tags={tags}`
    or `https://partner.example/{region}/contractors?type={project_type}`.
  * `tag_overrides: BTreeMap<String, String>` — optional map from
    canonical tag (e.g. `"pool"`, `"residential"`) to the partner's
    preferred wire string (e.g. `"swimming-pool"`). Empty by default.

Storage:

* `TenantSettingsDoc.routing: Option<RoutingConfig>` with
  `#[serde(default, skip_serializing_if = "Option::is_none")]`.
* No new index; routing config is read via the existing
  tenant-by-id path.

Domain / API DTO mirror so the dashboard `PATCH /v1/me` extension can
write it later (this phase: read-only — config seeded by direct Mongo
write or the same back-channel that seeds branding today).

**Decision lock:** template-substitution lives server-side in 2F.3.
We intentionally do not surface a free-form "redirect URL builder" to
tenants — they configure a template + tag overrides, and we own the
substitution to keep the URL shape predictable.

### 2F.2 — `routing_tags` on estimate `done` event

The Phase 2C.6 SSE pipeline emits a `done` event with the full
`EstimationOutput`. 2F adds a parallel `routing_tags: Vec<String>`
field derived from:

* `EstimationSession.region` → `"region:{slug}"`
* `ScopingContext.project_type` → canonical lowercased slug (after
  applying `tag_overrides` if configured)
* Cost-tier bucket from `EstimationOutput.total_cost_p50`:
  `"tier:low"` (< $25k), `"tier:mid"` (< $250k), `"tier:high"` (≥ $250k).
  Thresholds live in `RoutingConfig` as `cost_tier_breakpoints: [u64;
  2]` with the above defaults.
* `ScopingContext.location` (if set) → `"location:{lowercased}"` —
  partners can ignore if they want region-only routing.

A new `crates/efofx-core/src/services/routing.rs::RoutingService`
owns derivation. Pure function: `(session, scoping, output, config) →
Vec<String>`. Unit-tested in isolation.

The SSE handler in `api::chat::generate_estimate` calls
`RoutingService::tags_for(...)` and includes the result in the `done`
event JSON. **Additive only** — existing widget/dashboard consumers
ignore unknown fields. Tag list is empty when routing is disabled
for the tenant (so the field is always present, never omitted, to
keep the wire shape stable for partners who do parse it).

### 2F.3 — `POST /v1/integration/contractor-match`

Replace the 501 stub in `api::integration::contractor_match` with a
real handler.

* Auth: widget_api_key (already declared in OpenAPI).
* Validate: `estimation_session_id` non-empty; optional
  `project_type` / `location` overrides each ≤ 64 chars.
* Load `EstimationSession` via `EstimationRepo::get` for this tenant.
  404 envelope on miss (same shape as the email-request flow).
* Load the session's most-recent `ChatSession.scoping_context` to
  reuse `RoutingService::tags_for`. (If chat session lookup fails,
  fall through with empty scoping — the session-level `region` is
  still enough for a coarse tag set.)
* Apply request overrides: a non-empty `project_type` /
  `location` in the request *replaces* the corresponding scoping
  field before tag derivation.
* Build response:
  * `tags: Vec<String>` — same derivation as 2F.2.
  * `region: Option<String>` — session region as a display label.
  * `estimated_cost_tier: Option<String>` — `"low" | "mid" | "high"`.
  * `directory_url: Option<String>` — populated when
    `RoutingConfig.directory_url_template` is set; substitute
    `{tags}` (comma-joined), `{region}`, `{project_type}`,
    `{cost_tier}`. Unknown placeholders pass through untouched (no
    string panic).

If routing is disabled for the tenant: 200 with `tags: []`,
`region` populated, `directory_url: None`. Partners get a deterministic
"nothing to route" response rather than a 404 — they can still log
the inbound call.

**Endpoint stays widget_api_key auth.** Partners call this
server-to-server with the tenant's widget API key (same key they'd
use for `/v1/widget/leads`). No new credential surface.

### 2F.4 — Contract tests + checkpoint

* `crates/efofx-core/tests/integration_contractor_match.rs`
  * POST without API key → 401 envelope.
  * POST with bogus widget key → 401.
  * POST with malformed JSON → 400.
  * (Live happy path — session load + tag derivation + URL
    substitution — defers to Phase 4 with real Mongo.)
* `RoutingService` unit tests in `services/routing.rs`:
  * Tag derivation with full scoping → expected tag list (region +
    project_type + cost_tier + location).
  * Cost-tier bucketing at the breakpoint boundaries.
  * `tag_overrides` apply (canonical → partner string).
  * Disabled tenant → empty tag list.
  * URL template substitution: known placeholders fill, unknown pass
    through, repeated placeholders all fill.
* OpenAPI: extend `openapi::tests` with
  `openapi_integration_surface_contract` pinning the auth scheme +
  status codes (200/400/401/404, **not** 501) and required schema
  components (`ContractorMatchRequest`,
  `ContractorMatchResponse`). The current spec lists 501 — remove
  that response from the handler annotation when the stub goes.
* `docs/rust-port/phase-2f-checkpoint.md` — same shape as 2D's / 2E's.

## Decisions locked up front

1. **No partner directory of our own.** We emit tags + an opaque
   `directory_url`. The partner site owns presentation. This keeps the
   integration cheap to add per partner.
2. **Tag namespace** — colon-prefixed (`region:`, `tier:`,
   `project_type:`, `location:`). Easier for partners to filter and
   easier for us to evolve without colliding with custom tags.
3. **Cost-tier breakpoints in config, not code** — defaults `25000`
   and `250000`. Future per-tenant override is a config edit.
4. **`routing_tags` always present** in the SSE `done` event, even
   when routing is disabled (empty array). Stable wire shape beats a
   conditional field.
5. **Endpoint auth is widget_api_key.** Partners call it
   server-to-server with the tenant's widget API key. No new auth
   scheme; no public unauthenticated variant.
6. **Template substitution is `{name}` literal.** No mustache, no
   tera, no askama. Three placeholders, one regex pass. Add a real
   templater only if a fourth placeholder gets requested.
7. **`directory_url: Option<String>`** rather than `String` because
   tenants without a `directory_url_template` get `None`. Lets the
   partner distinguish "no preferred destination" from "empty URL".
8. **Read-only for v1.** Tenants seed `RoutingConfig` via direct
   Mongo write (or the future `PATCH /v1/me` extension). Same
   pattern as branding + allowed_origins in Phase 2D.

## Definition of Done

* [ ] `POST /v1/integration/contractor-match` returns its real
      response (not 501).
* [ ] `routing_tags` field present on the SSE `done` event.
* [ ] `RoutingConfig` round-trips through Mongo (read path verified
      by unit tests; write path lands with `PATCH /v1/me` extension
      later).
* [ ] `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`,
      `cargo test --workspace` all green.
* [ ] OpenAPI snapshot test covers the integration surface.
* [ ] Checkpoint doc written and committed.
* [ ] Jeff has a documented endpoint to integrate against; no
      blocker on the demo cutover for routing.
