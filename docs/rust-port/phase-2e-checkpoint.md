# Phase 2E Checkpoint — Feedback & Calibration

**Date:** 2026-04-25
**Branch:** `rust-port`
**Status:** All 6 subphases committed. Phase 2E closed.

---

## Goal

Move every `/feedback/*` and `/calibration/*` endpoint off FastAPI onto
Rust, preserving the on-disk Mongo schema and the dashboard contract so
the Phase 3 cutover is a base-URL change only.

## What shipped

| # | Commit | Scope |
|---|---|---|
| 2E.1 | `26ce436` | `FeedbackRepo` + `MagicLinkRepo` + on-disk schema parity |
| 2E.2 | `a4c2d8a` | `POST /v1/feedback` + `GET /v1/feedback/summary` |
| 2E.3 | `2efe2f6` | `POST /v1/feedback/email-requests` (mint + Resend) |
| 2E.4 | `f8d7ead` | `GET/POST /v1/feedback/forms/:token` (public HTML) |
| 2E.5 | `ca3f87d` | `GET /v1/calibration/{metrics,trend}` + below-threshold union |
| 2E.6 | _this commit_ | OpenAPI snapshot tests + checkpoint doc |

Run `git log --oneline rust-port ^main` to see all Phase 2 commits.

## Surface summary

| Verb | Path | Auth | Notes |
|---|---|---|---|
| POST | `/v1/feedback` | supabase_jwt OR widget_api_key | JSON rating capture |
| GET  | `/v1/feedback/summary` | supabase_jwt | Aggregate stats + `feedback_by_type` histogram |
| POST | `/v1/feedback/email-requests` | supabase_jwt | Mint magic-link, Resend best-effort |
| GET  | `/v1/feedback/forms/:token` | public (token-gated) | HTML form / expired / submitted |
| POST | `/v1/feedback/forms/:token` | public (token-gated) | Consume token + persist `FeedbackDocument` + `EstimateSnapshot` |
| GET  | `/v1/calibration/metrics` | supabase_jwt | Variance + buckets + per-RC OR below-threshold (untagged union) |
| GET  | `/v1/calibration/trend` | supabase_jwt | Monthly time-series OR below-threshold (untagged union) |

## Decisions locked during 2E

1. **`actual_timeline` vs `actual_timeline_weeks`** — Rust public DTO
   keeps `actual_timeline_weeks` (already published in OpenAPI). Storage
   layer maps Rust → Mongo with `#[serde(rename = "actual_timeline")]`
   for FastAPI parity. Public surface unaffected.
2. **`session_id` representation** — feedback persists
   `estimation_session_id` as a free-form `String` (FastAPI uses
   `sess_xxxxx`, not a UUID). Domain `SessionId` (UUID) is *not* used
   in this collection.
3. **No HTML template engine** — three short inline HTML pages
   embedded as `&'static str` constants with simple `{placeholder}`
   substitution. Total HTML weight under 4 KB each.
4. **Magic-link TTL** — 72 hours, hard-coded for v1 (`MAGIC_LINK_TTL`),
   matching FastAPI's `MAGIC_LINK_TTL_HOURS = 72`.
5. **Recent-feedback list omitted** from `GET /v1/feedback/summary` —
   adds payload weight that no current consumer needs. Additive
   field — can be re-added without breaking clients.
6. **`BelowThresholdResponse` field name** — FastAPI returns
   `outcome_count`, the Rust DTO matches (`MetricsBelowThreshold` /
   `TrendBelowThreshold` both use `outcome_count`).
7. **Calibration union as parallel components** — handler annotations
   reference `CalibrationMetrics` / `CalibrationTrend` directly; the
   sibling `MetricsBelowThreshold` / `TrendBelowThreshold` are also
   listed in OpenAPI components so the `#[serde(untagged)]` runtime
   shape stays generatable. The OpenAPI doc itself doesn't pin the
   union as a single schema — Phase 3 client codegen will revisit if
   the dashboard needs the union typed.
8. **Email scanner safety** — `GET /v1/feedback/forms/:token` always
   returns `200 text/html`. The page body encodes Valid / Expired /
   Used / NotFound; the status never does. `mark_opened` is best-effort
   and not security-relevant. Mirrors the FastAPI guarantee.
9. **Form route axum 0.7 fix** — registered as `/v1/feedback/forms/:token`
   in the router (matchit 0.7 only accepts `:name`). OpenAPI doc keeps
   the `{token}` form (OpenAPI 3 standard). Same pattern Phase 2D used
   for `/v1/widget/branding/:api_key_prefix`.

## Files added in 2E

- `crates/efofx-storage/src/feedback.rs` — `FeedbackRepo`, `FeedbackDoc`,
  `NewFeedback`, `NewFeedbackWithSnapshot`, `FeedbackSummaryRow`
- `crates/efofx-storage/src/magic_link.rs` — `MagicLinkRepo`,
  `MagicLinkDoc`, `NewMagicLink`, `MintedToken`, `TokenState`
- `crates/efofx-storage/src/calibration.rs` — `CalibrationRepo`,
  `RcGroup`, `TrendPoint` (CALB-01 + CALB-04 pipelines)
- `crates/efofx-core/src/services/calibration.rs` — `CalibrationService`
  (threshold orchestration, untagged union response builder)
- `crates/efofx-core/tests/feedback_endpoints.rs` — auth + form +
  validation contract tests (consolidates the planned
  `feedback_email_request.rs` and `feedback_form.rs` files)
- `crates/efofx-core/tests/calibration.rs` — auth contract +
  validation-leak guard

## Files modified in 2E

- `crates/efofx-config/src/lib.rs` — `EmailConfig.app_base_url`,
  `CalibrationConfig.minimum_outcomes`
- `crates/efofx-core/src/api/feedback.rs` — handlers replace
  `not_implemented` stubs (5 endpoints)
- `crates/efofx-core/src/api/calibration.rs` — handlers replace stubs;
  adds `MetricsResponse` / `TrendResponse` untagged enums
- `crates/efofx-core/src/lib.rs` — wires `feedback`, `magic_link`,
  `calibration` services + `app_base_url` into `AppState`
- `crates/efofx-core/src/openapi.rs` — feedback + calibration entries
  in `paths(...)` and `components(...)`; new
  `openapi_feedback_surface_contract` and
  `openapi_calibration_surface_contract` snapshot tests

## Test coverage

| Suite | Coverage |
|---|---|
| `feedback_endpoints.rs` | POST without auth → 401 envelope, bogus widget key → 401, summary requires JWT, email-request requires JWT (also rejects widget key + unknown JWT kid), `GET /forms/:token` unknown token → 200 expired HTML, POST form rejects missing fields + bad rating |
| `calibration.rs` | metrics + trend require JWT, reject widget keys, reject unknown JWT kids; query-validation does not leak shape to unauthenticated callers (401 fires before 400) |
| `openapi::tests::openapi_feedback_surface_contract` | Pinned status codes + auth schemes + required schema components for all 5 feedback endpoints |
| `openapi::tests::openapi_calibration_surface_contract` | Pinned status codes + auth schemes + required schema components for both calibration endpoints |
| `openapi::tests::openapi_contains_expected_paths` | Pins all 24 `/v1/*` + system paths |

Persistence + happy-path coverage (201 with Mongo, real aggregation
pipelines against seeded outcomes) lands in the infrastructure suite
(Phase 4) — the fake Mongo handle in `TestHarness` errors on every
query, which is exactly what these contract tests need to verify the
auth + validation layers run before storage.

## Definition of Done — status

- [x] Every Phase 2E endpoint returns its real response (not 501)
- [x] `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`,
      `cargo test --workspace` all green
- [x] OpenAPI snapshot tests cover the feedback + calibration surfaces
- [x] No regression in Phase 2D widget contract tests
- [x] Checkpoint doc written and committed
- [x] FastAPI feedback / calibration endpoints can be flagged for
      removal in Phase 3 cutover (no Rust gap remaining)
- [ ] **Live integration test** against real Mongo + a seeded tenant
      (deferred to Phase 4 alongside Resend live keys)

## Follow-ups queued for later phases

- **Calibration union typed in OpenAPI** — current spec lists the
  Metrics + BelowThreshold types as parallel components rather than
  a single `oneOf`. Phase 3 dashboard client may want a generated
  union — revisit if it becomes friction.
- **Per-tenant `minimum_outcomes`** — config knob exists but is
  global. A future tenant-settings field can override.
- **HTML form polish** — three inline pages are functional but plain.
  If the magic-link flow becomes a customer-visible product surface
  (rather than a calibration backstop), revisit with `askama` or
  similar.
- **Pre-existing clippy nit in `efofx-storage::chat::tests`** —
  `field_reassign_with_default` on the `ScopingContext` test fixture
  (chat.rs:503-504), Phase 2B.6 vintage. Surfaces only under
  `--all-targets`. Trivial cleanup; not gated by `cargo clippy
  --workspace -- -D warnings`.
- Live OpenAPI byte-compat check vs. the FastAPI-generated doc
  (carry-over from Phase 2D follow-ups)
- Axum 0.8 upgrade to remove the `:name` vs `{name}` divergence
  (carry-over from Phase 2D follow-ups)
