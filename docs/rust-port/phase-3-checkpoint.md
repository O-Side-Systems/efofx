# Phase 3 Checkpoint — Client Cutover & FastAPI Decommission

**Date:** 2026-04-28
**Branch:** `rust-port`
**Status:** All 6 subphases committed. Phase 3 closed.
**Companion:** [phase-3-plan.md](./phase-3-plan.md), [phase-2f-checkpoint.md](./phase-2f-checkpoint.md), [dashboard-auth.md](./dashboard-auth.md), [cutover-smoke.md](./cutover-smoke.md), [RUST-PORT-PLAN.md §Phase 3 / §Phase 5](../RUST-PORT-PLAN.md)

---

## Goal

Cut both clients (`apps/efofx-widget`, `apps/efofx-dashboard`) over to
the Rust backend and decommission the FastAPI service in the same
phase. No production traffic is at risk on the `rust-port` branch — no
parallel-run, no fallback, no compatibility shims. The wire shape we
ship at the end of Phase 3 is the shape we live with.

Phase 5 (decommission) was folded into 3.5 since there is no need to
keep FastAPI alive between phases.

## What shipped

| # | Scope |
|---|---|
| 3.1 | Widget API client rewrite — `/v1` paths, `x-api-key` header, two-call chat flow, plural widget paths |
| 3.2 | Dashboard auth migration to Supabase JS — `signInWithPassword`, session-driven JWT interceptor, sign-out wired |
| 3.3 | Dashboard calibration repoint — drop `/api/v1` prefix |
| 3.4 | `.env.example` alignment across `efofx-core`, `efofx-widget`, `efofx-dashboard`; cutover smoke runbook in `docs/rust-port/cutover-smoke.md` |
| 3.5 | FastAPI decommission — `apps/efofx-estimate/` → `apps/archive/efofx-estimate-fastapi/`; `python-checks` job removed from CI; `docs/CODEBASE-STATE.md` updated; `docs/IMPLEMENTATION-PLAN.md` marked superseded |
| 3.6 | This checkpoint |

3.1–3.6 ship as a single commit per the user's "commit all of phase 3
once complete" preference.

## Files touched (live source)

### `apps/efofx-widget/src/`
- `api/client.ts` — `x-api-key` header; `${API_BASE}/v1${path}` template.
- `api/chat.ts` — `createChatSession` + `appendMessage` (two-call flow).
- `hooks/useChat.ts` — rewritten around the two-call flow.
- `hooks/useEstimateStream.ts` — `x-api-key`; `:generate-estimate` path.
- `components/ConsultationCTA.tsx` — minor wire-shape fixup.
- `types/widget.d.ts` — types for the new chat response payloads.

### `apps/efofx-dashboard/src/`
- `lib/supabase.ts` (new) — Supabase client factory.
- `api/client.ts` — JWT interceptor reads `supabase.auth.getSession()`.
- `api/calibration.ts` — `/v1/calibration/...` paths.
- `pages/Login.tsx` — `signInWithPassword`; localStorage writes removed.
- `pages/Dashboard.tsx` — sign-out button.
- `router.tsx` — async auth guard against the live Supabase session.
- `App.css` — `.signout-button` style.
- `package.json` — `@supabase/supabase-js` added.

### Workspace
- `.github/workflows/ci.yml` — removed `python-checks` job (FastAPI tests, efofx-shared isolation test).
- `pyproject.toml` — workspace member path updated to the archive.
- `docs/CODEBASE-STATE.md` — Rust-as-authoritative rewrite.
- `docs/IMPLEMENTATION-PLAN.md` — front-matter notice marking it superseded.
- `apps/archive/README.md` (new) — explains what lives in archive.
- `apps/archive/efofx-estimate-fastapi/` — was `apps/efofx-estimate/` (git mv).

### Env examples
- `apps/efofx-core/.env.example` — Mongo, Supabase (URL/issuer/audience/JWKS refresh), crypto master key, Resend (test-mode shape), logging.
- `apps/efofx-widget/.env.example` — `VITE_API_URL`, `VITE_DEMO_API_KEY`.
- `apps/efofx-dashboard/.env.example` — `VITE_API_BASE_URL`, `VITE_SUPABASE_URL`, `VITE_SUPABASE_ANON_KEY`.

## Decisions locked during 3

1. **No FastAPI parity in Rust.** The Rust shape (two-call chat,
   plural widget paths, `x-api-key` header, no `/api` prefix) wins;
   clients adapt.
2. **Dashboard auth = Supabase JS.** The Rust core verifies JWTs via
   JWKS — no custom `/auth/login` on the Rust side, ever.
3. **Widget auth header = `x-api-key`.** `Authorization: Bearer` is
   reserved for Supabase JWTs.
4. **Path prefix = `/v1`**, not `/api/v1`. Dropped everywhere.
5. **No new client features in Phase 3.** The dashboard sign-out
   button is the sole UI addition, required to exercise the auth-path
   DoD. Lead management, tenant settings, BYOK UI, and the
   EstimationOutput-persistence gap go to Phase 4.
6. **FastAPI decommission folds in.** The original Phase 5 work
   (archive move, CI cleanup, doc refresh) shipped as 3.5 — there is
   no reason to gate on a parallel run we're not running.
7. **Routing surface stays.** Phase 2F partner-routing endpoints
   (`/v1/integration/contractor-match`, SSE `routing_tags`) ship as
   they are.
8. **`efofx-shared` (Python) stays in the workspace** but is documented
   as consumed only by the archive. New domain code lives in the Rust
   crates under `apps/efofx-core/crates/`.
9. **Archive is read-only.** New work never reaches into
   `apps/archive/efofx-estimate-fastapi/` — port to `efofx-core`
   instead. See `apps/archive/README.md`.

## Definition of Done — phase

- [x] Widget builds (`npm run build` in `apps/efofx-widget`).
- [x] Dashboard builds (`npm run build` in `apps/efofx-dashboard`).
- [x] No `Authorization: Bearer <api-key>` patterns remain in widget source.
- [x] No `/api/v1` strings remain in widget or dashboard source.
- [x] Sign-in / sign-out path uses Supabase JS exclusively.
- [x] FastAPI moved to `apps/archive/efofx-estimate-fastapi/`; CI no
      longer runs Python checks.
- [x] `docs/CODEBASE-STATE.md` reflects Rust as authoritative.
- [x] Cutover smoke runbook exists at `docs/rust-port/cutover-smoke.md`.
- [ ] Cutover smoke runbook walked through end-to-end against a clean
      Mongo state. (Manual; not part of the commit. Run before the
      first deploy off `rust-port`.)
- [ ] No client-side workflow rules duplicate server logic. (Spot-check
      called out in cutover-smoke step 5; treat any failure as Phase 4
      cleanup.)

## Follow-ups for Phase 4

- Lead list / detail dashboard pages (consume `GET /v1/leads`).
- Tenant settings UI — branding configuration, BYOK key management.
- API key management page (mint / list / revoke via `/v1/me/api-keys:rotate`).
- User profile editor (`PATCH /v1/me`).
- EstimationOutput-persistence gap (called out in the original
  RUST-PORT-PLAN; tracked there).
- Decide on `efofx-shared` Python package: archive alongside the
  FastAPI app or delete. Currently retained as a workspace member
  solely for archive buildability.
- Decide on `apps/estimator-project/` and `apps/estimator-mcp-functions/`
  — neither is on the Rust path; both should either be archived or
  promoted with a real owner.
