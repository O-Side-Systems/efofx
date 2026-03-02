---
phase: 07-feedback-email-magic-links
plan: 03
subsystem: api
tags: [jinja2, email, fastapi, background-tasks, magic-links, html-templates]

# Dependency graph
requires:
  - phase: 07-01
    provides: FeedbackEmailService with Resend SDK, send_email() interface
  - phase: 07-02
    provides: MagicLinkService with create_magic_link(), token lifecycle

provides:
  - POST /api/v1/feedback/request-email/{session_id} endpoint (feedback_email_router)
  - feedback_email.html Jinja2 template with inline CSS and tenant branding
  - Non-blocking email dispatch via BackgroundTasks
  - Jinja2>=3.0 dependency added to pyproject.toml and requirements.txt

affects: [07-04, 07-05]

# Tech tracking
tech-stack:
  added: [Jinja2>=3.0 (explicit dependency; was transitive via FastAPI)]
  patterns:
    - Jinja2 Environment loaded once at module level (not per-request)
    - BackgroundTasks wraps async send_email for fire-and-forget dispatch
    - Estimate data extracted from session_doc.estimation_output or .result (flexible schema)
    - Tenant branding sourced from tenant.settings["branding"] dict with defaults

key-files:
  created:
    - apps/efofx-estimate/app/templates/feedback_email.html
    - apps/efofx-estimate/app/api/feedback_email.py
    - apps/efofx-estimate/tests/services/test_feedback_email_trigger.py
  modified:
    - apps/efofx-estimate/app/main.py
    - apps/efofx-estimate/pyproject.toml
    - apps/efofx-estimate/requirements.txt

key-decisions:
  - "Jinja2 Environment loaded once at module level (_jinja_env) to avoid filesystem I/O on every request"
  - "BackgroundTasks wraps an async closure (_send) that calls FeedbackEmailService.send_email — matches fire-and-forget pattern from plan spec"
  - "Estimate data extracted with .get() fallback chain (estimation_output, result, {}) — tolerates both schema versions"
  - "Branding dict sourced from tenant.settings['branding'] with sane defaults — no extra DB lookup needed"

patterns-established:
  - "Email template pattern: 600px table-based HTML, all CSS inline, conditional Jinja2 sections for optional data"
  - "Feedback endpoint auth: Depends(get_current_tenant) + per-tenant rate limit via @limiter.limit(get_tier_limit)"

requirements-completed: [FEED-04]

# Metrics
duration: 8min
completed: 2026-03-02
---

# Phase 7 Plan 3: Feedback Email Trigger and Template Summary

**POST /api/v1/feedback/request-email/{session_id} endpoint with Jinja2 HTML email template featuring inline CSS tenant branding, estimate context, and BackgroundTasks dispatch**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-03-02T14:20:10Z
- **Completed:** 2026-03-02T14:28:11Z
- **Tasks:** 2
- **Files modified:** 6 (3 created, 3 modified)

## Accomplishments
- Created `feedback_email.html` — a 600px table-based Jinja2 template with all CSS inline, tenant branding (logo, colors, company name), cost range/timeline, cost breakdown table, assumptions list, and "Share Your Feedback" CTA button
- Created `POST /api/v1/feedback/request-email/{session_id}` endpoint: validates session, creates magic link, renders branded HTML, queues email via BackgroundTasks (non-blocking)
- Wired `feedback_email_router` into `main.py` under `/api/v1` prefix
- Added `Jinja2>=3.0` to `pyproject.toml` and `requirements.txt`
- 5 unit tests: success, 404, 401, template rendering with/without optional sections — all pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Create Jinja2 HTML email template** - `e6a9089` (feat)
2. **Task 2: Create feedback email trigger endpoint and wire router** - `a0838e3` (feat)

**Plan metadata:** (pending final commit)

## Files Created/Modified
- `apps/efofx-estimate/app/templates/feedback_email.html` - Jinja2 HTML email with inline CSS, tenant branding, estimate summary, CTA button
- `apps/efofx-estimate/app/api/feedback_email.py` - POST endpoint, FeedbackEmailRequest/Response models, BackgroundTasks dispatch
- `apps/efofx-estimate/tests/services/test_feedback_email_trigger.py` - 5 unit tests for endpoint and template
- `apps/efofx-estimate/app/main.py` - Added `feedback_email_router` import and `app.include_router` registration
- `apps/efofx-estimate/pyproject.toml` - Added `Jinja2>=3.0` dependency
- `apps/efofx-estimate/requirements.txt` - Added `Jinja2>=3.0` dependency

## Decisions Made
- Jinja2 Environment loaded once at module level (`_jinja_env`) to avoid filesystem I/O on every request
- BackgroundTasks wraps an async closure `_send()` that calls `FeedbackEmailService.send_email()` — fire-and-forget, non-blocking
- Estimate data uses flexible extraction: `session_doc.get("estimation_output") or session_doc.get("result") or {}` — tolerates both schema versions present in the DB
- Branding dict sourced from `tenant.settings.get("branding", {})` with defaults matching `BrandingConfig` model values — no additional DB lookup

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added Jinja2>=3.0 to pyproject.toml and requirements.txt**
- **Found during:** Task 1 (template creation and verification)
- **Issue:** Jinja2 was not explicitly listed in project dependencies (it was a transitive dependency via FastAPI/uvicorn[standard]), but the project needed it explicitly for the feedback email template. Also, system Python lacked Jinja2 while venv had it — missing from explicit deps list.
- **Fix:** Added `"Jinja2>=3.0"` to `pyproject.toml` dependencies and `requirements.txt`. Installed via `pip install Jinja2>=3.0 --no-input` on system Python for script verification.
- **Files modified:** `pyproject.toml`, `requirements.txt`
- **Verification:** `python -c "import jinja2"` succeeds; template render test passes
- **Committed in:** `a0838e3` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking dependency)
**Impact on plan:** Necessary to make Jinja2 an explicit declared dependency. No scope creep.

## Issues Encountered
- System Python (3.13, homebrew) lacked Jinja2; venv already had it as a transitive dep. The template verification command in the plan works correctly when run via the venv python. Added explicit Jinja2 declaration to deps to prevent future confusion.

## User Setup Required
None - no external service configuration required. Existing RESEND_API_KEY from Phase 7-01 covers email dispatch.

## Next Phase Readiness
- Contractor can now trigger feedback emails via POST /api/v1/feedback/request-email/{session_id}
- Template renders correctly with all branding + estimate context variables
- Magic link URL embedded in CTA button pointing to /feedback/form/{raw_token}
- Ready for 07-04: customer feedback form HTML page (GET/POST /feedback/form/{token})
- Ready for 07-05: any remaining feedback flow work

---
*Phase: 07-feedback-email-magic-links*
*Completed: 2026-03-02*

## Self-Check: PASSED

- FOUND: `apps/efofx-estimate/app/templates/feedback_email.html`
- FOUND: `apps/efofx-estimate/app/api/feedback_email.py`
- FOUND: `apps/efofx-estimate/tests/services/test_feedback_email_trigger.py`
- FOUND: `.planning/phases/07-feedback-email-magic-links/07-03-SUMMARY.md`
- FOUND: commit `e6a9089` (Task 1 - email template)
- FOUND: commit `a0838e3` (Task 2 - endpoint + tests)
