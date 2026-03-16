---
phase: 05-tech-debt-foundation-cleanup
plan: 02
subsystem: ui, api
tags: [fastapi, pydantic, react, typescript, i18n, email, consultation, lead-capture]

# Dependency graph
requires: []
provides:
  - POST /widget/consultation endpoint (FastAPI, returns 201, API key auth)
  - ConsultationRequest and ConsultationResponse Pydantic models
  - save_consultation() service saving to widget_leads with lead_type="consultation"
  - Email notification to contractor with graceful degradation (fastapi-mail optional)
  - ConsultationForm.tsx inline contact form component with locale support
  - i18n/consultationForm.ts with en and es locale constants and getLabels() merge function
  - ConsultationCTA.tsx wired to show form on button click and success message on submit
  - submitConsultation() in chat.ts calling POST /widget/consultation
  - ConsultationFormLabels and ConsultationFormData TypeScript types
  - BrandingConfig extended with locale and consultation_form_labels (backend and frontend)
  - MAIL_USERNAME/MAIL_PASSWORD/MAIL_FROM/MAIL_PORT/MAIL_SERVER optional Settings fields
affects:
  - Phase 6 (any work touching widget lead capture or branding config)
  - Phase 7 (transactional email provider — may want to replace fastapi-mail pattern)

# Tech tracking
tech-stack:
  added:
    - fastapi-mail (optional import — only imported when email is configured; not added to requirements.txt yet as optional)
  patterns:
    - Graceful email degradation: import fastapi-mail inside try block, log warning if MAIL_SERVER/MAIL_USERNAME not set, never raise
    - i18n locale merge: getLabels(locale, overrides) — base from locale map, overrides spread on top, null-safe
    - Inline form toggle: showForm state in parent CTA component controls which child renders (button / form / success)
    - ConsultationRequest is a distinct model (not extending LeadCaptureRequest) — extra message field, different validation

key-files:
  created:
    - apps/efofx-widget/src/components/ConsultationForm.tsx
    - apps/efofx-widget/src/i18n/consultationForm.ts
  modified:
    - apps/efofx-estimate/app/models/widget.py
    - apps/efofx-estimate/app/services/widget_service.py
    - apps/efofx-estimate/app/api/widget.py
    - apps/efofx-estimate/app/core/config.py
    - apps/efofx-widget/src/components/ConsultationCTA.tsx
    - apps/efofx-widget/src/api/chat.ts
    - apps/efofx-widget/src/types/widget.d.ts
    - apps/efofx-widget/src/widget.css

key-decisions:
  - "ConsultationRequest is a distinct Pydantic model (not extending LeadCaptureRequest) — has message field and different max_length validation"
  - "Email notification uses graceful degradation: lead is always saved first, email failure never causes 500, fastapi-mail imported lazily inside try block"
  - "ConsultationForm reuses existing CSS classes (efofx-lead-input, efofx-lead-submit, efofx-lead-error) — only adds efofx-consultation-form-title, efofx-consultation-textarea, efofx-cta-success"
  - "getLabels() falls back to en locale if requested locale not found, then merges per-tenant overrides on top"

patterns-established:
  - "Inline form toggle pattern: showForm/submitted state in ConsultationCTA.tsx renders button -> form -> success conditionally"
  - "Graceful email degradation: check MAIL_SERVER and MAIL_USERNAME before importing fastapi-mail; log warning and return on missing config"
  - "Locale merge pattern: getLabels(locale, overrides) with null-safe spread"

requirements-completed:
  - DEBT-04

# Metrics
duration: 4min
completed: 2026-02-28
---

# Phase 5 Plan 02: Wire ConsultationCTA to Inline Contact Form Summary

**Inline consultation form end-to-end: ConsultationForm.tsx (en/es i18n) + POST /widget/consultation (FastAPI, save_consultation with email, graceful degradation)**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-03-01T02:46:44Z
- **Completed:** 2026-03-01T02:50:12Z
- **Tasks:** 2
- **Files modified:** 8 (modified) + 2 (created)

## Accomplishments
- Backend: ConsultationRequest/ConsultationResponse models, POST /widget/consultation endpoint (201, API key auth), save_consultation() service writing to widget_leads with lead_type="consultation", email notification with graceful degradation
- Frontend: ConsultationForm.tsx inline form (name/email/phone/message, controlled inputs, validation, isSubmitting state), ConsultationCTA.tsx rewritten to toggle button/form/success via showForm state, console.info stub removed
- i18n: consultationForm.ts with en and es locale constants, getLabels() merge function supporting per-tenant branding overrides
- TypeScript compiles with zero errors; no console stub remains in ConsultationCTA

## Task Commits

Each task was committed atomically:

1. **Task 1: Add backend ConsultationRequest model, save_consultation service, POST endpoint, and email config** - `84f11c9` (feat)
2. **Task 2: Build ConsultationForm frontend component with locale support and wire ConsultationCTA** - `ee050c4` (feat)

## Files Created/Modified
- `apps/efofx-estimate/app/models/widget.py` - Added ConsultationRequest, ConsultationResponse, and locale/consultation_form_labels fields to BrandingConfig/BrandingConfigResponse
- `apps/efofx-estimate/app/services/widget_service.py` - Added save_consultation() and _send_consultation_email() with graceful degradation
- `apps/efofx-estimate/app/api/widget.py` - Added POST /widget/consultation endpoint
- `apps/efofx-estimate/app/core/config.py` - Added MAIL_USERNAME/MAIL_PASSWORD/MAIL_FROM/MAIL_PORT/MAIL_SERVER optional settings
- `apps/efofx-widget/src/components/ConsultationCTA.tsx` - Rewritten to toggle button/form/success state, console stub removed
- `apps/efofx-widget/src/components/ConsultationForm.tsx` - Created: inline form component with locale support, mirrors LeadCaptureForm pattern
- `apps/efofx-widget/src/i18n/consultationForm.ts` - Created: en/es locale constants, getLabels() merge function
- `apps/efofx-widget/src/api/chat.ts` - Added submitConsultation() function
- `apps/efofx-widget/src/types/widget.d.ts` - Added ConsultationFormLabels, ConsultationFormData interfaces; extended BrandingConfig with locale/consultation_form_labels
- `apps/efofx-widget/src/widget.css` - Added efofx-consultation-form-title, efofx-consultation-textarea, efofx-cta-success CSS classes

## Decisions Made
- ConsultationRequest is a standalone Pydantic model (not extending LeadCaptureRequest) per research anti-patterns — has distinct message field and tighter name max_length (100 vs 200)
- Email notification uses graceful degradation: fastapi-mail is imported lazily inside the try block so the service works without the package installed; MAIL_SERVER/MAIL_USERNAME absence logs a warning and returns cleanly, never raising
- ConsultationForm CSS reuses existing efofx-lead-input/submit/error classes for visual consistency; only adds three new classes
- getLabels() in i18n falls back to en locale when requested locale not found, then merges tenant overrides as a last layer

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- Python import verification initially failed because the system Python lacked pydantic; resolved by using the project .venv/bin/python

## User Setup Required
None - no external service configuration required during execution. To enable consultation email notifications, contractors should set MAIL_USERNAME, MAIL_PASSWORD, MAIL_FROM, MAIL_PORT, and MAIL_SERVER environment variables. Without these, the endpoint works correctly and logs a warning.

## Next Phase Readiness
- DEBT-04 complete: ConsultationCTA button is fully wired to inline form, backend saves leads and notifies contractor
- The blocker noted in STATE.md ("ConsultationCTA destination URL requires product decision") is resolved: the destination is an inline form, not an external URL
- Phase 5 Plan 01 remains to be executed before phase is complete
- If email notifications are needed in production, MAIL_* env vars must be set and fastapi-mail added to requirements.txt

---
*Phase: 05-tech-debt-foundation-cleanup*
*Completed: 2026-02-28*
