---
phase: 07-feedback-email-magic-links
plan: 04
subsystem: api
tags: [jinja2, html-templates, fastapi, magic-link, feedback, mongodb, form-submission]

# Dependency graph
requires:
  - phase: 07-02
    provides: MagicLinkService (resolve_token_state, mark_opened, consume), FeedbackDocument, EstimateSnapshot, FeedbackSubmission models, FeedbackMagicLink token structure
provides:
  - GET /feedback/form/{token} public endpoint — renders branded form, expired page, or thank-you based on token state
  - POST /feedback/form/{token} public endpoint — consumes token, stores FeedbackDocument with immutable EstimateSnapshot
  - Three Jinja2 HTML templates: feedback_form.html, feedback_expired.html, feedback_submitted.html
  - store_feedback_with_snapshot() method on FeedbackService
  - feedback_form_router registered in main.py at root path (no /api/v1 prefix)
affects: [08-calibration-pipeline, feedback-analytics]

# Tech tracking
tech-stack:
  added: [Jinja2 (already in requirements — first use for server-rendered templates)]
  patterns: [server-rendered HTML via Jinja2 (not JSON API), public token-gated endpoints, atomic consume-then-store pattern]

key-files:
  created:
    - apps/efofx-estimate/app/templates/feedback_form.html
    - apps/efofx-estimate/app/templates/feedback_expired.html
    - apps/efofx-estimate/app/templates/feedback_submitted.html
    - apps/efofx-estimate/app/api/feedback_form.py
    - apps/efofx-estimate/tests/api/test_feedback_form.py
  modified:
    - apps/efofx-estimate/app/services/feedback_service.py
    - apps/efofx-estimate/app/main.py

key-decisions:
  - "Jinja2 templates dir resolved via os.path at module level (not runtime) — fast and avoids FileSystemLoader per request"
  - "GET never consumes token (mark_opened only) — email scanners safe; only POST with valid form data consumes"
  - "feedback_form_router registered at root path with no prefix — /feedback/form/{token} is user-facing URL in emails, must be short"
  - "Race condition guard: consume() returns bool, False means another request won race — render thank-you without double-storing"
  - "EstimateSnapshot built at POST time from session doc — copy-on-write, later estimate edits do not affect stored feedback"

patterns-established:
  - "Public HTML endpoints: return HTMLResponse(template.render(**ctx)) with no JSON — full ASGI stack compatible"
  - "Token state machine: resolve_token_state -> ('valid'|'expired'|'used'|'not_found', token_doc) drives all branching"
  - "Form fields parsed via FastAPI Form(...) dependency injection for application/x-www-form-urlencoded"

requirements-completed: [FEED-05, FEED-06, FEED-07]

# Metrics
duration: 4min
completed: 2026-03-02
---

# Phase 7 Plan 4: Feedback Form Summary

**Public Jinja2 server-rendered feedback form with GET/POST token-gated endpoints, three HTML templates, and FeedbackDocument storage with immutable EstimateSnapshot**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-02T14:24:09Z
- **Completed:** 2026-03-02T14:28:13Z
- **Tasks:** 2
- **Files modified:** 7 (3 created as templates, 2 created as Python, 2 modified)

## Accomplishments

- Three mobile-responsive Jinja2 templates: branded form with P50/P80 estimate summary, expired/not-found page, and thank-you page
- GET /feedback/form/{token} endpoint with idempotent mark_opened (safe for email scanner crawls)
- POST /feedback/form/{token} endpoint that atomically consumes token, builds EstimateSnapshot from session data, and stores FeedbackDocument
- Double-submit prevention: atomic consume() check + race condition guard (consume returns False)
- store_feedback_with_snapshot() method added to FeedbackService — EstimateSnapshot copied at submit time, immutable
- 7 tests covering all token states (valid/expired/used/not_found) and race conditions

## Task Commits

Each task was committed atomically:

1. **Task 1: Create Jinja2 HTML templates** - `4f27da2` (feat)
2. **Task 2: GET/POST endpoints, store_feedback_with_snapshot, and router wiring** - `5b9e427` (feat)

**Plan metadata:** (docs commit — see below)

## Files Created/Modified

- `apps/efofx-estimate/app/templates/feedback_form.html` - Branded form with estimate summary, P50/P80 stats, cost breakdown, star rating, discrepancy dropdowns, mobile-responsive
- `apps/efofx-estimate/app/templates/feedback_expired.html` - Friendly expired/not-found page with efOfX minimal branding
- `apps/efofx-estimate/app/templates/feedback_submitted.html` - Thank-you page with company_name branding
- `apps/efofx-estimate/app/api/feedback_form.py` - Public GET/POST endpoints, feedback_form_router, _load_estimate_context helper
- `apps/efofx-estimate/app/services/feedback_service.py` - Added store_feedback_with_snapshot() method
- `apps/efofx-estimate/app/main.py` - Added feedback_form_router registration without /api/v1 prefix
- `apps/efofx-estimate/tests/api/test_feedback_form.py` - 7 tests for all endpoint states and edge cases

## Decisions Made

- GET uses mark_opened (idempotent) but never consume — email security scanners can follow magic link URLs without destroying the token
- feedback_form_router registered at root (not /api/v1) — user-facing URLs in emails need to be clean and short
- Race condition guard: after resolve_token_state returns 'valid', consume() is called — if it returns False, another request won the race; render thank-you without storing duplicate
- EstimateSnapshot fields default to 0/[] when session doc not found — form still submits, Phase 8 calibration gets minimal data rather than hard failure

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

The templates directory did not exist yet (new in this plan). Created it automatically as part of Task 1. No issues.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Complete feedback loop is now functional: magic link email (07-03) -> form (07-04) -> FeedbackDocument in MongoDB
- FeedbackDocument includes reference_class_id for Phase 8 calibration pipeline
- Remaining in Phase 7: 07-05 (end-to-end integration test or send-feedback-email wiring)
- Phase 8 (CalibrationService) can now query feedback collection for real outcomes against EstimateSnapshot

## Self-Check: PASSED

All 8 expected files found. Both task commits verified (4f27da2, 5b9e427).

---
*Phase: 07-feedback-email-magic-links*
*Completed: 2026-03-02*
