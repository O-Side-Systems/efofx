# Phase 2E — Feedback & Calibration: Plan

**Date:** 2026-04-25
**Branch:** `rust-port`
**Status:** Plan locked, implementation starting with 2E.1.
**Companion:** [phase-2d-checkpoint.md](./phase-2d-checkpoint.md), [RUST-PORT-PLAN.md §Phase 2E](../RUST-PORT-PLAN.md)

---

## Goal

Move every `/feedback/*` and `/calibration/*` endpoint off FastAPI onto
Rust, preserving the on-disk Mongo schema and the dashboard contract so a
client cutover in Phase 3 is a base-URL change only.

## Surface map (post-2E)

| Verb | Path | Auth | Notes |
|---|---|---|---|
| POST | `/v1/feedback` | supabase_jwt OR widget_api_key | JSON rating capture (parallel to FastAPI `Feedback`/`FeedbackCreate`) |
| GET  | `/v1/feedback/summary` | supabase_jwt | Aggregate stats (count, avg rating, accuracy averages) |
| POST | `/v1/feedback/email-requests` | supabase_jwt | Mint magic-link, send branded email, return token hash |
| GET  | `/v1/feedback/forms/:token` | public (token-gated) | HTML form / expired / submitted |
| POST | `/v1/feedback/forms/:token` | public (token-gated) | Consume token, store `FeedbackDocument` + `EstimateSnapshot` |
| GET  | `/v1/calibration/metrics` | supabase_jwt | Variance + buckets + per-RC breakdown OR below-threshold |
| GET  | `/v1/calibration/trend` | supabase_jwt | Monthly time-series OR below-threshold |

All endpoints already exist as `not_implemented` stubs in
`crates/efofx-core/src/api/{feedback,calibration}.rs`; this phase fills
them in and wires storage.

## Subphase breakdown

### 2E.1 — Storage layer

Add to `efofx-storage`:

* `feedback.rs`
  * `FeedbackRepo` — tenant-scoped reader/writer for collection `feedback`
  * `FeedbackDoc` — Mongo wire shape mirroring FastAPI's `Feedback` /
    `FeedbackDocument` blend (uses `actual_timeline` not
    `actual_timeline_weeks`, `estimation_session_id` as a free-form
    string for `sess_xxxxx` parity).
  * `NewFeedback` (JSON-flow input) and `NewFeedbackWithSnapshot`
    (magic-link flow input).
  * `FeedbackSummaryRow` projection used by `summary()`.
  * Indexes: `{tenant_id: 1, created_at: -1}`, `{tenant_id: 1,
    estimation_session_id: 1}`.
* `magic_link.rs`
  * `MagicLinkRepo` — collection `feedback_tokens`. **Not** tenant-scoped
    at the API surface (the form is unauthenticated), but every method
    threads `tenant_id` from the token document into all reads/writes
    that touch other collections.
  * `MagicLinkDoc` — `token_hash`, `tenant_id`, `estimation_session_id`,
    `customer_email`, `project_name`, `expires_at`, `opened_at`,
    `used_at`, `created_at`. Schema-identical to FastAPI's
    `FeedbackMagicLink`.
  * `create()` returns `(raw_token, token_hash)`. `resolve()` returns
    one of `Valid | Expired | Used | NotFound` plus the doc on the first
    three. `mark_opened()` is idempotent. `consume()` is atomic and
    returns `bool` (true = first consume).
  * Indexes: unique `{token_hash: 1}`, TTL `{expires_at: 1, expireAfterSeconds: 0}`.
* `lib.rs` re-exports.

**Field-naming reconciliation.** FastAPI persists `actual_timeline`
(int weeks). The Rust public API DTO uses `actual_timeline_weeks`. Storage
DTO maps Rust → Mongo with `#[serde(rename = "actual_timeline")]`. The
public surface is unaffected.

### 2E.2 — JSON feedback endpoints

* `POST /v1/feedback` (`create_feedback`)
  * Auth: either supabase_jwt or widget_api_key (already declared in
    OpenAPI). Both produce a `TenantContext`.
  * Validate: `rating ∈ 1..=5`, `feedback_type` non-empty, `session_id`
    is a non-empty string (FastAPI uses `sess_*`, not UUID).
  * Insert via `FeedbackRepo::insert_basic`. Return `201` with
    `{feedback_id, message}`.
* `GET /v1/feedback/summary` (`feedback_summary`)
  * Auth: supabase_jwt.
  * Aggregate: total, mean rating, optional accuracy averages (skip
    `None`s), feedback-by-type histogram. Recent 10 entries omitted from
    the v1 Rust contract (they were noisy in FastAPI; FE doesn't need
    them yet — re-add later if dashboard asks).

The OpenAPI `FeedbackSummary` schema currently lacks
`feedback_by_type` and `recent_feedback`. We'll add `feedback_by_type:
HashMap<String, u64>` to keep parity with the dashboard JSON.

### 2E.3 — Magic-link mint + email

* `POST /v1/feedback/email-requests` (`request_email`)
  * Auth: supabase_jwt.
  * Validate `estimation_session_id` exists and belongs to the tenant
    (via `EstimationRepo::get`). 404 on miss.
  * Mint via `MagicLinkRepo::create()`.
  * Build URL: `{config.feedback.app_base_url}/v1/feedback/forms/{raw}`.
  * Compose plaintext email — short subject (`How did {project_name}
    go?`), body with project name + magic link + expiry note.
  * Fire via `state.email`. Best-effort: failure is logged, response is
    still `202` with `{message, token_hash}`. Mirrors the consultation
    pattern.
* New config: `EmailConfig.app_base_url` (default
  `"http://localhost:8080"`). Used to build the form URL.

### 2E.4 — Public HTML form

* `GET /v1/feedback/forms/:token` (`render_form`)
  * Resolve token state.
    * `NotFound | Expired` → render expired HTML, `200`.
    * `Used` → render thank-you HTML, `200`.
    * `Valid` → `mark_opened`, render form HTML, `200`.
  * No auth, no rate limit beyond global. Email scanners following the
    URL must not consume the token (FastAPI guarantee).
* `POST /v1/feedback/forms/:token` (`submit_form`)
  * Parse `application/x-www-form-urlencoded` payload into
    `FeedbackSubmission`.
  * Validate (`rating 1..=5`, `actual_cost > 0`, `actual_timeline_weeks >
    0`, `comment.len() ≤ 2000`).
  * `consume()` — if returns false, render thank-you (idempotent).
  * Load estimation session for snapshot; build `EstimateSnapshot`.
  * Store via `FeedbackRepo::insert_with_snapshot`.
  * Render thank-you HTML.
* HTML: minimal inline strings, no template engine. Three fixed pages
  (form, expired, submitted) embed branding tokens (company name,
  primary color) read from `tenants.settings.branding` with a
  defaults fallback. Total HTML weight under 4 KB each — the FastAPI
  Jinja2 templates are styled but not load-bearing; the dashboard cuts
  over before we polish this further.
* **Axum 0.7 fix.** Re-register `/v1/feedback/forms/{token}` as
  `/v1/feedback/forms/:token` so matchit accepts the route. OpenAPI
  spec keeps the `{token}` form (OpenAPI 3 standard).

### 2E.5 — Calibration

In `efofx-storage`, add `calibration.rs`:

* `CalibrationRepo` — wraps the `feedback` collection's aggregation
  surface. Owns:
  * `count_outcomes(ctx, date_filter) -> u64`
  * `metrics(ctx, date_filter) -> Vec<RcGroup>` (per-RC variances + count)
  * `trend(ctx, since) -> Vec<TrendPoint>` (monthly aggregation)
* Pipelines mirror FastAPI byte-for-byte:
  * **CALB-01** — `$lookup reference_classes` excludes
    `data_source = "synthetic"` in the inner pipeline.
  * **CALB-04** — `$lookup` always uses `let`/`pipeline` form with
    explicit `tenant_id` filter (`TenantContext` does not leak into
    `$lookup` automatically).

In `crates/efofx-core/src/services/calibration.rs` (new):

* `CalibrationService` — pure orchestrator.
  * `metrics(ctx, date_range) -> CalibrationMetricsResponse |
    BelowThresholdResponse`
  * `trend(ctx, months) -> CalibrationTrendResponse |
    BelowThresholdResponse`
* Threshold = 10 outcomes (CALB-03). Lifted into a new
  `CalibrationConfig.minimum_outcomes` (default 10) so a future
  per-tenant override is a config edit.

API DTOs (`api::calibration::*`) need one tweak: `BelowThresholdResponse`
is currently a sibling type. The FastAPI client treats both shapes as a
union under one endpoint. Cleanest is a tagged enum
`MetricsResponse { Metrics(...), BelowThreshold(...) }` with
`#[serde(untagged)]` so the wire shape stays identical to FastAPI. Same
for trend.

### 2E.6 — Contract tests + checkpoint

* New tests under `crates/efofx-core/tests/`:
  * `feedback_endpoints.rs` — POST without auth → 401 envelope; POST
    with bogus widget key → 401; summary without JWT → 401.
  * `feedback_email_request.rs` — auth-rejection envelope.
  * `feedback_form.rs` — GET unknown token → expired-HTML response,
    `text/html`. POST without form fields → `400`.
  * `calibration.rs` — metrics + trend require JWT; bad
    `?date_range=junk` → `400` validation envelope.
* Extend `openapi::tests`:
  * Add `openapi_feedback_surface_contract` (status codes per endpoint,
    auth schemes, `FeedbackDocument` / `FeedbackSubmission` / new union
    types in components).
  * Add `openapi_calibration_surface_contract`.
* `docs/rust-port/phase-2e-checkpoint.md` — same shape as 2D's.

## Decisions locked up front

1. **`actual_timeline` vs `actual_timeline_weeks`** — Rust public DTO
   keeps `actual_timeline_weeks` (already published in OpenAPI).
   Storage DTO renames to `actual_timeline` for FastAPI Mongo parity.
2. **`session_id` representation** — feedback persists
   `estimation_session_id` as a free-form `String` (FastAPI uses
   `sess_xxxxx`, not a UUID). Domain `SessionId` (UUID) is *not* used
   in this collection.
3. **No HTML template engine** — three short inline HTML pages embedded
   as `&'static str` constants with simple `{placeholder}` substitution.
   We can swap to `askama` if/when we add a fourth page.
4. **Magic-link TTL** — 72 hours, lifted from FastAPI's
   `MAGIC_LINK_TTL_HOURS = 72`. Hard-coded for v1; configurable later.
5. **Recent-feedback list omitted** from `GET /v1/feedback/summary` —
   adds payload weight that no current consumer needs. Can be re-added
   without breaking clients (additive field).
6. **`BelowThresholdResponse` field name** — FastAPI returns
   `outcome_count`, the existing Rust DTO has `count`. Rename Rust to
   `outcome_count` so the wire shape matches FastAPI exactly. Update
   the OpenAPI snapshot test if it pinned the old name.
7. **Email content** — plaintext only. Resend supports HTML, but
   FastAPI's branded Jinja template is a polish item; the
   demand-cutover deadline is the link working, not the gradient.

## Definition of Done

* [ ] Every Phase 2E endpoint returns its real response (not 501).
* [ ] `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`,
      `cargo test --workspace` all green.
* [ ] OpenAPI snapshot tests cover the feedback + calibration surfaces.
* [ ] No regression in Phase 2D widget contract tests.
* [ ] Checkpoint doc written and committed.
* [ ] FastAPI feedback / calibration endpoints can be flagged for
      removal in Phase 3 cutover (no Rust gap remaining).
