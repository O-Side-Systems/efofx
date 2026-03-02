---
phase: 07-feedback-email-magic-links
plan: 01
subsystem: infra
tags: [resend, email, transactional-email, feedback, magic-links, async]

# Dependency graph
requires: []
provides:
  - FeedbackEmailService with async send_email wrapping Resend SDK via run_in_threadpool
  - RESEND_API_KEY optional setting in Settings (None default for local dev)
  - Graceful degradation: logs magic link URL to console when unconfigured
  - resend>=2.0.0 installed in project virtualenv and declared in pyproject.toml + requirements.txt
affects: [07-03, 07-04]

# Tech tracking
tech-stack:
  added: [resend>=2.0.0]
  patterns:
    - Resend SDK is synchronous — always wrapped with run_in_threadpool to avoid blocking async event loop
    - Graceful email degradation pattern (same as auth_service.py SMTP pattern): check API key, log and return None if absent
    - Email failure isolation: catch all exceptions in send_email, log error, return None — never re-raise

key-files:
  created:
    - apps/efofx-estimate/app/services/feedback_email_service.py
    - apps/efofx-estimate/tests/services/test_feedback_email_service.py
  modified:
    - apps/efofx-estimate/pyproject.toml
    - apps/efofx-estimate/requirements.txt
    - apps/efofx-estimate/app/core/config.py

key-decisions:
  - "Resend SDK installed as resend>=2.0.0 (not pinned) — SDK is actively updated and semver-stable"
  - "RESEND_API_KEY: Optional[str] = None — consistent with SMTP_USERNAME and MAIL_USERNAME optional email pattern"
  - "run_in_threadpool wraps resend.Emails.send — Resend SDK uses requests (synchronous), must not block async loop"
  - "FeedbackEmailService.send_email returns Optional[str] (Resend message ID or None) — caller can log ID but must handle None"

patterns-established:
  - "Email service init pattern: self._configured = bool(settings.RESEND_API_KEY) — checked once at construction"
  - "Dev-mode magic link logging: warning includes to_email and magic_link_url for local testing without Resend account"

requirements-completed: [FEED-01]

# Metrics
duration: 2min
completed: 2026-03-02
---

# Phase 7 Plan 01: Resend SDK Infrastructure Summary

**Resend SDK installed and FeedbackEmailService created — async-safe send_email wrapping synchronous Resend client via run_in_threadpool, with dev-mode fallback logging magic link URL to console**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-02T14:16:03Z
- **Completed:** 2026-03-02T14:18:02Z
- **Tasks:** 2
- **Files modified:** 5 (3 modified, 2 created)

## Accomplishments
- Resend SDK (v2.23.0) installed in project virtualenv and declared in pyproject.toml + requirements.txt
- RESEND_API_KEY optional setting added to Settings with None default (consistent with existing SMTP/MAIL patterns)
- FeedbackEmailService created with async send_email method, wrapping synchronous Resend SDK via run_in_threadpool
- Graceful degradation: when RESEND_API_KEY is None, logs warning with magic link URL for dev testing
- Email failure isolation: all SDK exceptions caught, logged, and swallowed — feedback request flow is never broken
- 10 unit tests covering all cases: configured, unconfigured, and error paths

## Task Commits

Each task was committed atomically:

1. **Task 1: Install Resend SDK and add RESEND_API_KEY config** - `9ae36b9` (feat)
2. **Task 2: Create FeedbackEmailService with Resend SDK wrapper and unit tests** - `100abd5` (feat)

**Plan metadata:** (docs commit — see below)

## Files Created/Modified
- `apps/efofx-estimate/app/services/feedback_email_service.py` - FeedbackEmailService with async send_email, graceful degradation, run_in_threadpool wrapper
- `apps/efofx-estimate/tests/services/test_feedback_email_service.py` - 10 unit tests covering configured/unconfigured/error cases
- `apps/efofx-estimate/app/core/config.py` - Added RESEND_API_KEY Optional[str] = None to Settings
- `apps/efofx-estimate/pyproject.toml` - Added resend>=2.0.0 to dependencies array
- `apps/efofx-estimate/requirements.txt` - Added resend>=2.0.0 under Cache section

## Decisions Made
- Resend SDK not pinned to exact version (>=2.0.0) — SDK is actively updated and maintains semver stability; pinning would cause unnecessary churn
- RESEND_API_KEY follows existing optional email config pattern (SMTP_USERNAME, MAIL_USERNAME both use Optional[str] = None)
- run_in_threadpool chosen over asyncio.get_event_loop().run_in_executor() — consistent with Starlette patterns used elsewhere in codebase
- send_email returns Optional[str] (Resend message ID or None) to allow callers to log IDs while handling None gracefully

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- pip install resend failed on system Python (PEP 668 externally-managed environment). Resolved by using project virtualenv (.venv/bin/pip install resend). No code changes needed.

## User Setup Required

**External services require manual configuration before feedback emails will send in production.**

Resend account and DNS setup needed:
1. Create Resend account at https://resend.com/signup
2. Add sending domain (e.g. efofx.com) and configure DNS — Resend Dashboard -> Domains -> Add Domain
3. Add SPF, DKIM, and DMARC DNS records as shown by Resend (your DNS provider — Cloudflare, Route53, etc.)
4. Verify domain in Resend dashboard — Resend Dashboard -> Domains -> Verify
5. Create API key — Resend Dashboard -> API Keys -> Create API Key
6. Set `RESEND_API_KEY=re_...` in production environment variables

**For local dev:** Leave RESEND_API_KEY unset. FeedbackEmailService will log the magic link URL to console so you can test without a Resend account.

## Next Phase Readiness
- FeedbackEmailService is ready for Plans 07-03 and 07-04 to import and use for magic link delivery
- Plans 07-03/07-04 need to: construct HTML email body (Jinja2 template), call `svc.send_email(to, subject, html, magic_link_url)`
- Resend account + DNS must be configured before production feedback emails will send

---
*Phase: 07-feedback-email-magic-links*
*Completed: 2026-03-02*
