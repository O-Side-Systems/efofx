---
gsd_state_version: 1.0
milestone: v1.1
milestone_name: Feedback & Quality
status: unknown
last_updated: "2026-03-02T14:34:26.433Z"
progress:
  total_phases: 3
  completed_phases: 3
  total_plans: 8
  completed_plans: 8
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-28)

**Core value:** Trust through transparency — probabilistic estimates with explainable breakdowns that build contractor credibility with customers
**Current focus:** v1.1 Feedback & Quality — Phase 8 (Calibration Dashboard) in progress

## Current Position

Phase: 8 of 9 (Calibration Dashboard)
Plan: 2 of 3 complete (08-02 done — dashboard app scaffolded: Vite + React 19, auth guard, axios JWT interceptor, React Query hooks for calibration metrics + trend)
Status: In Progress
Last activity: 2026-03-05 — 08-02-PLAN.md executed: apps/efofx-dashboard/ SPA on port 5174, types/calibration.ts matching CalibrationService shapes, apiClient with Bearer token interceptor, useCalibration + useCalibrationTrend hooks, requireAuth router loader, Login page, Dashboard placeholder (CALB-05 partial)

Progress: [██████░░░░] 50% (9/18 plans complete across all v1.1 phases)

## Performance Metrics

**Velocity:**
- Total plans completed: 4 (v1.1)
- Average duration: ~3-4min
- Total execution time: ~16min

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| v1.1 starting | — | — | — |
| Phase 05-tech-debt-foundation-cleanup P02 | 4min | 2 tasks | 10 files |
| Phase 05-tech-debt-foundation-cleanup P01 | 4min | 2 tasks | 7 files modified, 2 deleted |
| Phase 05-tech-debt-foundation-cleanup P03 | 1 | 1 tasks | 1 files |
| Phase 06-valkey-infrastructure P01 | 4min | 3 tasks | 7 files |
| Phase 07-feedback-email-magic-links P01 | 2min | 2 tasks | 5 files |
| Phase 07-feedback-email-magic-links P02 | 4min | 2 tasks | 5 files |
| Phase 07-feedback-email-magic-links P03 | 8min | 2 tasks | 6 files |
| Phase 07-feedback-email-magic-links P04 | 4min | 2 tasks | 7 files |
| Phase 08-calibration-dashboard P02 | 4min | 2 tasks | 18 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.

Key decisions affecting v1.1 work:
- Magic link token: GET is idempotent (sets opened_at, does NOT consume token); only form POST consumes token — prevents email scanner redemption
- Valkey cache keys: prefixed with tenant_id, use hashlib.sha256 (never hash()), serialized as JSON (never pickle)
- CalibrationService $lookup: must manually scope tenant_id in inner pipeline — TenantAwareCollection only scopes source collection
- Email provider: select transactional provider (Resend/Postmark/SendGrid) as first story of Phase 7 before writing any magic-link code
- Minimum calibration threshold: 10 real outcomes before any metric displays (product decision, validate before Phase 8)
- ConsultationCTA destination URL: confirm destination before Phase 5 begins (one-line fix, product decision required)
- [Phase 05-tech-debt-foundation-cleanup]: DEBT-04 consultation form: ConsultationRequest is a distinct model (not extending LeadCaptureRequest) with message field; email notification uses graceful degradation via fastapi-mail with MAIL_* settings
- [Phase 05-01]: EstimationSession.result and EstimationResponse.result changed to Optional[Any] after EstimationResult class deleted — always None in new generate_from_chat flow
- [Phase 05-01]: slowapi pin updated from >=0.1.0 to ==0.1.9 to match pyproject.toml exactly
- [Phase 05-03]: Two-line fix only: locale=branding.locale and consultation_form_labels=branding.consultation_form_labels added to BrandingConfigResponse constructor. No model changes required.
- [Phase 06-01]: ValkeyCache key format locked as efofx:llm:{tenant_id}:{input_hash} — tenant prefix before hash for tenant-scoped Redis SCAN
- [Phase 06-01]: ValkeyCache._get_client() lazy init (not at import) avoids connection attempt during test collection; warning cooldown uses module-level _last_warn_at to throttle across all requests via singleton
- [Phase 06-01]: VALKEY_URL must use rediss:// scheme (not valkeys://) for slowapi/limits library and valkey.asyncio compatibility in production
- [Phase 07-01]: Resend SDK installed as resend>=2.0.0 (not pinned) — actively updated SDK; RESEND_API_KEY: Optional[str] = None consistent with existing SMTP/MAIL optional pattern
- [Phase 07-01]: run_in_threadpool wraps resend.Emails.send — Resend SDK uses requests (synchronous), must not block async event loop; send_email returns Optional[str] (Resend ID or None)
- [Phase 07-02]: SHA-256 hash stored in feedback_tokens, never raw token — database compromise cannot replay magic links
- [Phase 07-02]: mark_opened idempotent via {opened_at: None} filter at DB level — no application-level read-before-write needed
- [Phase 07-02]: consume returns bool (modified_count > 0) — avoids extra read to check used_at state
- [Phase 07-03]: Jinja2 Environment loaded once at module level (_jinja_env) — avoids filesystem I/O on every request
- [Phase 07-03]: BackgroundTasks wraps async _send() closure for fire-and-forget email dispatch — non-blocking by design
- [Phase 07-03]: Estimate data extracted via .get() fallback chain (estimation_output, result, {}) — tolerates both schema versions
- [Phase 07-04]: feedback_form_router registered at root path (no /api/v1) — user-facing magic link URLs in emails must be short and clean
- [Phase 07-04]: Race condition guard in POST: consume() returns False when another request won the race — render thank-you without double-storing
- [Phase 07-04]: EstimateSnapshot built at POST time from session doc — copy-on-write, later estimate edits do not affect stored feedback context
- [Phase 08-02]: Port 5174 for efofx-dashboard (5173 is efofx-widget); Vite proxy /api -> :8000 avoids CORS in dev
- [Phase 08-02]: React Query v5 staleTime 5min for calibration data (changes infrequently)
- [Phase 08-02]: CSS custom properties (no Tailwind) — Stripe/Linear muted aesthetic per user preference
- [Phase 08-02]: by_reference_class typed as optional in CalibrationMetrics — below-threshold response omits it

### Pending Todos

None.

### Blockers/Concerns

- Phase 5: All plans complete (05-01, 05-02, and 05-03 done). Phase 5 fully complete.
- Phase 6: DigitalOcean Managed Valkey cluster still needs provisioning (06-01 user_setup). Use rediss:// scheme (confirmed: slowapi requires this, not valkeys://)
- Phase 7: 07-01 (Resend SDK), 07-02 (MagicLinkService + models), 07-03 (email trigger endpoint + template), and 07-04 (feedback form) complete. SPF/DKIM/DMARC DNS setup + RESEND_API_KEY still needed in production. Plan 07-05 still pending.
- Phase 8: Confirm 10-outcome minimum threshold with stakeholder before Phase 8 begins

## Session Continuity

Last session: 2026-03-05
Stopped at: Completed 08-02-PLAN.md (dashboard app scaffold — Vite + React 19 SPA, auth guard router, JWT axios interceptor, useCalibration + useCalibrationTrend hooks, Login page, Dashboard placeholder — CALB-05 partial)
Resume file: None
