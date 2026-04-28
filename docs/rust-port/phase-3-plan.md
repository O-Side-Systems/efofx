# Phase 3 — Client Cutover (and FastAPI Decommission): Plan

**Date:** 2026-04-27
**Branch:** `rust-port`
**Status:** Plan draft. Implementation starts with 3.1.
**Companion:** [phase-2f-checkpoint.md](./phase-2f-checkpoint.md), [RUST-PORT-PLAN.md §Phase 3](../RUST-PORT-PLAN.md), [dashboard-auth.md](./dashboard-auth.md)

---

## Goal

Cut both client apps (`apps/efofx-widget`, `apps/efofx-dashboard`) over to
the Rust backend and decommission the FastAPI service in the same
phase. We are on the `rust-port` branch with no production traffic — no
parallel-run, no fallback, no shim. Hard cutover is fine; the
public widget/dashboard wire shape we ship at the end of Phase 3 is the
shape we live with.

Phase 5 (decommission) folds into 3.5 below since there is no need to
keep FastAPI alive between phases.

## Scope (in)

- Repoint `efofx-widget` to the Rust API surface (path prefix + auth
  header + the two-call chat flow).
- Repoint `efofx-dashboard` and migrate its login from a custom
  email/password endpoint to Supabase JS.
- Archive `apps/efofx-estimate` and update `docs/CODEBASE-STATE.md`.
- Remove FastAPI from CI.

## Scope (out)

- Jeff Brumit's contractor-directory integration. Deferred indefinitely.
- New product features. The cutover preserves existing widget /
  dashboard behavior, no more.
- Rust-side wire-shape changes for FastAPI parity. We are no longer
  bound by FastAPI compatibility — if a divergence is cleaner in Rust,
  the Rust shape wins and the *client* adapts.

## Wire-shape divergences to resolve

The Rust port deliberately reshaped a few endpoints. The client cutover
is more than a base-URL flip; the widget and dashboard need real code
changes. Cataloged here so 3.1 and 3.2 don't drift into shape-fix
churn during implementation.

### Widget (`apps/efofx-widget/src/api/`)

| Existing call | Rust path | Notes |
|---|---|---|
| `POST /api/v1/chat/send` (single-call create+append) | `POST /v1/chat/sessions` (create) → `POST /v1/chat/sessions/{id}/messages` (append) | Two-call flow. `useChat.ts` needs rewrite. |
| `POST /api/v1/widget/lead` | `POST /v1/widget/leads` | Plural, body shape matches. |
| `POST /api/v1/widget/analytics` | `POST /v1/widget/events` | Body shape matches (`event_type`). |
| `POST /api/v1/widget/consultation` | `POST /v1/widget/consultations` | Plural, body shape matches. |
| `GET /api/v1/widget/branding/{prefix}` | `GET /v1/widget/branding/{prefix}` | Path prefix only. |
| `POST /api/v1/chat/{id}/generate-estimate` | `POST /v1/chat/sessions/{id}:generate-estimate` | Path shape. SSE event stream matches: `thinking`, `estimate` (full output JSON), un-named `data:` (narrative tokens, escaped `\n`), `done` (now also includes `routing_tags`). |
| Header `Authorization: Bearer {apiKey}` | Header `x-api-key: {apiKey}` | Rust widget auth is on `x-api-key`; Bearer is reserved for Supabase JWT. |
| Path prefix `/api/v1/...` | Path prefix `/v1/...` | Rust has no `/api` outer segment. |

### Dashboard (`apps/efofx-dashboard/src/`)

| Existing call | Rust path | Notes |
|---|---|---|
| `POST /api/v1/auth/login` (email + password) | _no Rust endpoint_ | Migrate `Login.tsx` to Supabase JS sign-in. localStorage `access_token` becomes the Supabase session JWT. See `docs/rust-port/dashboard-auth.md`. |
| `GET /api/v1/calibration/metrics?date_range=…` | `GET /v1/calibration/metrics?date_range=…` | Path prefix only. Query params match. |
| `GET /api/v1/calibration/trend?months=…` | `GET /v1/calibration/trend?months=…` | Path prefix only. Query params match. |
| Header `Authorization: Bearer {access_token}` | Header `Authorization: Bearer {supabase_jwt}` | Same wire shape; the *contents* of the token change. |

The dashboard currently only consumes calibration. The other dashboard
surfaces (`/v1/me`, `/v1/leads`, `/v1/feedback/*`, `/v1/widget/events`
GET) exist on the Rust side but are not yet wired in the dashboard
client. Phase 3 does **not** add new dashboard pages — only repoints
the existing ones.

## Subphase breakdown

### 3.1 — Widget API client rewrite

Rewrite the widget's `src/api/` layer against the Rust surface.

* `src/api/client.ts`
  * `apiClient` — replace `Authorization: Bearer` with
    `x-api-key: {apiKey}`. Drop the `/api` segment from the URL
    template (now `${API_BASE}/v1${path}`).
  * `publicClient` — same prefix change, no auth header changes.
* `src/api/chat.ts`
  * Replace `sendMessage(apiKey, message, sessionId | null)` with two
    typed calls:
    * `createChatSession(apiKey, initialMessage?) → ChatSession`
      (`POST /v1/chat/sessions`)
    * `appendMessage(apiKey, sessionId, message) → AppendMessageResponse`
      (`POST /v1/chat/sessions/{id}/messages`)
  * `submitLead`, `trackEvent`, `submitConsultation` — point at the
    plural paths (`/widget/leads`, `/widget/events`,
    `/widget/consultations`).
* `src/api/branding.ts` — no change beyond the prefix flip in
  `client.ts`.
* `src/hooks/useChat.ts` — rewrite around the two-call flow. First user
  message: `createChatSession` with `initial_message`. Subsequent
  messages: `appendMessage(sessionId, …)`. Both return the full
  `ChatSession` plus an assistant message — the existing
  optimistic-render path stays intact.
* `src/hooks/useEstimateStream.ts`
  * Replace `Authorization: Bearer` with `x-api-key`.
  * Update endpoint to `/v1/chat/sessions/{id}:generate-estimate`.
  * SSE event handlers (`thinking`, `estimate`, plain `data:`, `done`,
    `error`) match Rust output as-is — the new `routing_tags` field on
    `done` is additive and ignored.

**Non-goals for 3.1.** No UI changes, no new SSE events, no new fields
in widget surface. The diff should be purely network/transport.

**Definition of Done.**
* Widget builds (`npm run build` in `apps/efofx-widget`) and the
  embed test page (`test-embed.html`) renders against a locally-running
  Rust core.
* End-to-end manual test: branding fetch, two chat messages, generate
  estimate (SSE narrative + estimate event + done), submit lead,
  submit consultation, fire an analytics event. All return 2xx.
* No FastAPI references remain in `efofx-widget/src/`.

### 3.2 — Dashboard auth migration to Supabase JS

Replace the custom email/password login flow with Supabase auth. The
dashboard already has the JWT-bearer pattern — only the *source* of
the token changes.

* Add `@supabase/supabase-js` to `apps/efofx-dashboard`.
* `src/lib/supabase.ts` (new) — single Supabase client factory keyed
  off `VITE_SUPABASE_URL` + `VITE_SUPABASE_ANON_KEY`. Mirror the
  config shape documented in `docs/rust-port/dashboard-auth.md`.
* `src/pages/Login.tsx` — replace the `apiClient.post('/api/v1/auth/login')`
  call with `supabase.auth.signInWithPassword({ email, password })`.
  On success, the SDK persists the session in localStorage; the
  `apiClient` interceptor reads from `supabase.auth.getSession()`
  rather than `localStorage.getItem('access_token')` directly. Removes
  the `refresh_token` write.
* `src/api/client.ts` — interceptor pulls the JWT from the live
  Supabase session (handles refresh transparently) instead of a
  static localStorage entry.
* Add a guard / redirect on protected routes if no session is present.

**Definition of Done.**
* Sign-in via a Supabase test account succeeds.
* Authenticated `apiClient` calls attach the Supabase JWT and reach
  the Rust core.
* Sign-out clears the session and redirects to `/login`.

### 3.3 — Dashboard API repoint

Trivial relative to 3.2 once the JWT plumbing is in place.

* `src/api/calibration.ts` — drop `/api` prefix from both calls.
* `src/api/client.ts` — `baseURL` resolves to the Rust core
  (`VITE_API_BASE_URL`). No `/v1` hard-coded — the path strings carry
  it.
* Sanity-check the dashboard renders against a Rust core with seeded
  calibration data (or the empty-state below-threshold response when
  no data is present — both are first-class on the Rust side).

**Definition of Done.**
* Calibration metrics + trend pages render against the Rust core.
* No `/api/v1` strings remain in `efofx-dashboard/src/`.

### 3.4 — End-to-end smoke + config alignment

Before declaring cutover done, exercise both clients against the same
Rust core, end to end.

* `apps/efofx-core/.env.example` — pin the Supabase URL, JWKS endpoint,
  Mongo URI, Resend key (test mode), and master key shape.
* `apps/efofx-widget/.env.example` — `VITE_API_URL` pointing at the
  local Rust core port.
* `apps/efofx-dashboard/.env.example` — `VITE_API_BASE_URL`,
  `VITE_SUPABASE_URL`, `VITE_SUPABASE_ANON_KEY`.
* Manual end-to-end script in `docs/rust-port/cutover-smoke.md`
  walking through: provision a tenant via dashboard → embed widget on
  `test-embed.html` with the tenant's API key → run a full chat +
  estimate flow → check the estimate appears in the dashboard's
  calibration pipeline.

**Definition of Done.**
* `cutover-smoke.md` walkthrough passes from a clean Mongo state.
* No client-side workflow rules duplicate server logic — all gating
  (readiness, status transitions, budget enforcement, expiry) is
  server-side.

### 3.5 — Decommission FastAPI

Folds in the original Phase 5 tasks since there's no parallel-run
period to gate them.

* `git mv apps/efofx-estimate apps/archive/efofx-estimate-fastapi/`
  with a top-level `README.md` pointing at `apps/efofx-core` as the
  authoritative backend. Keep the archive readable for historical
  context (PRD references, calibration prompt history) but mark it
  non-authoritative.
* Update `docs/CODEBASE-STATE.md` — replace "FastAPI is the source of
  truth" framing with "Rust core (`apps/efofx-core`) is authoritative;
  FastAPI archive lives at `apps/archive/efofx-estimate-fastapi/` for
  reference only."
* Update `docs/IMPLEMENTATION-PLAN.md` — mark superseded; link to
  `RUST-PORT-PLAN.md` as the live plan.
* CI — drop any FastAPI build / test jobs from the GitHub Actions
  config (or wherever they live). Keep the workspace lint pipeline
  for the Rust workspace.
* Delete unused FastAPI-specific docs (or move them under
  `apps/archive/efofx-estimate-fastapi/docs/`). Keep historical PRD
  and pivot analysis untouched per the original Phase 5 spec.

**Definition of Done.**
* `apps/efofx-core` is the only backend referenced in current docs.
* `apps/archive/efofx-estimate-fastapi/` is clearly non-authoritative.
* CI runs only the Rust + client pipelines.

### 3.6 — Phase 3 checkpoint

Same shape as 2D / 2E / 2F checkpoints. Pin decisions, list files
touched, list follow-ups for Phase 4.

## Decisions locked up front

1. **No FastAPI parity in Rust.** Where the Rust shape diverges
   (chat two-call flow, plural widget paths, `x-api-key` header), the
   *clients* adapt. We are not adding FastAPI compatibility shims to
   the Rust core.
2. **Dashboard auth = Supabase JS.** No custom `/auth/login` on the
   Rust side. The Rust API verifies Supabase JWTs via JWKS (already
   shipped in 2A); the dashboard mints them via the Supabase JS SDK.
3. **Widget auth header = `x-api-key`.** Bearer is reserved for
   Supabase JWT. The widget's `Authorization: Bearer` was a FastAPI
   convention not preserved on the Rust side.
4. **Path prefix is `/v1`, not `/api/v1`.** Drop the outer `/api`
   segment in all client code. Rust core has no `/api` namespace.
5. **No new client features in Phase 3.** A repoint, not a redesign.
   New dashboard pages, new widget surfaces, and the
   EstimationOutput-persistence gap go to Phase 4.
6. **FastAPI decommission folds in.** Phase 5 tasks (archive, CI
   cleanup, doc updates) ship in 3.5. No reason to hold them for a
   phase that gates only on a parallel-run we're not running.
7. **Routing surface stays.** Phase 2F partner-routing endpoints
   (`/v1/integration/contractor-match`, SSE `routing_tags`) ship as
   they are. They were FastAPI-parity port work, not Jeff-driven —
   no new feature work, but no rip-out either.

## Definition of Done (phase)

- [ ] `apps/efofx-widget` and `apps/efofx-dashboard` both run against
      the Rust core with no FastAPI references in source.
- [ ] Manual cutover smoke test in `docs/rust-port/cutover-smoke.md`
      passes from a clean Mongo state.
- [ ] FastAPI moved to `apps/archive/efofx-estimate-fastapi/`; CI no
      longer builds or tests it.
- [ ] `docs/CODEBASE-STATE.md` reflects Rust as authoritative.
- [ ] Checkpoint doc written and committed.
- [ ] No client-side logic duplicates server workflow rules.
