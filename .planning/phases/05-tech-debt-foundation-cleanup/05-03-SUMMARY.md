---
phase: 05-tech-debt-foundation-cleanup
plan: "03"
subsystem: api
tags: [pydantic, branding, locale, i18n, python, fastapi]

# Dependency graph
requires:
  - phase: 05-02
    provides: ConsultationRequest model and consultation endpoint wired through widget_service.py

provides:
  - Per-tenant locale and consultation_form_labels propagated from BrandingConfig through BrandingConfigResponse in the branding API
  - Frontend getLabels() now receives real per-tenant values instead of always-default values

affects:
  - Phase 06 (any work touching branding or i18n)
  - Frontend widget locale rendering

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "BrandingConfigResponse constructor must include all BrandingConfig fields — no silent omissions"

key-files:
  created: []
  modified:
    - apps/efofx-estimate/app/services/widget_service.py

key-decisions:
  - "Two-line fix only: locale=branding.locale and consultation_form_labels=branding.consultation_form_labels added to BrandingConfigResponse constructor. No model changes required."

patterns-established:
  - "When BrandingConfig and BrandingConfigResponse share fields, constructor calls must forward all fields explicitly — defaults in the response model are not a substitute for wiring through stored tenant values"

requirements-completed: [DEBT-04]

# Metrics
duration: 1min
completed: "2026-02-28"
---

# Phase 05 Plan 03: Branding API Locale Fix Summary

**Two-line fix wiring per-tenant locale and consultation_form_labels from BrandingConfig through to BrandingConfigResponse so Spanish-locale tenants and custom-form-label tenants receive correct values instead of always-default en/null**

## Performance

- **Duration:** < 1 min
- **Started:** 2026-03-01T05:26:50Z
- **Completed:** 2026-03-01T05:27:30Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Added `locale=branding.locale` to the `BrandingConfigResponse` constructor call in `get_branding_by_prefix()`
- Added `consultation_form_labels=branding.consultation_form_labels` to the same constructor call
- Per-tenant locale and form label overrides now propagate from stored `BrandingConfig` to the API response
- The frontend `getLabels()` merge logic (already correct) now receives real values instead of always-default `en`/`null`

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire locale and consultation_form_labels through BrandingConfigResponse** - `8d58854` (fix)

## Files Created/Modified

- `apps/efofx-estimate/app/services/widget_service.py` - Added two missing keyword arguments to `BrandingConfigResponse` constructor in `get_branding_by_prefix()`

## Decisions Made

None — followed plan as specified. The fix was exactly as designed: two keyword argument additions, no model modifications, no frontend changes.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. The `python -c` import verification initially failed when run from the workspace root (ModuleNotFoundError: No module named 'app') but succeeded correctly when run from the `apps/efofx-estimate/` directory — the standard FastAPI app layout requires running Python from the app root. This is expected behavior, not a bug.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- DEBT-04 gap fully closed: per-tenant locale and consultation_form_labels now reach the frontend
- Phase 05 (Tech Debt & Foundation Cleanup) is now fully complete — all three plans (05-01, 05-02, 05-03) done
- Phase 06 can proceed with confidence that branding API response includes all tenant customization fields

---
*Phase: 05-tech-debt-foundation-cleanup*
*Completed: 2026-02-28*

## Self-Check: PASSED

- FOUND: `apps/efofx-estimate/app/services/widget_service.py`
- FOUND: `.planning/phases/05-tech-debt-foundation-cleanup/05-03-SUMMARY.md`
- FOUND commit: `8d58854` (fix(05-03): wire locale and consultation_form_labels through BrandingConfigResponse)
- VERIFIED: `locale=branding.locale` at line 88 in widget_service.py
- VERIFIED: `consultation_form_labels=branding.consultation_form_labels` at line 89 in widget_service.py
