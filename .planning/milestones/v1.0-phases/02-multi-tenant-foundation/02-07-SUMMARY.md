---
phase: 02-multi-tenant-foundation
plan: "07"
subsystem: api
tags: [openai, byok, llm, testing, documentation]

# Dependency graph
requires:
  - phase: 02-multi-tenant-foundation
    provides: BYOK encryption layer (encrypt/decrypt_tenant_openai_key), 402 gate implementation

provides:
  - Corrected BYOK-04 requirement text matching locked decision and implementation
  - LLMService refactored to accept per-request api_key for BYOK injection
  - 6 unit tests proving BYOK key injection contract

affects:
  - Phase 3 (LLM-01 per-request client injection)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - LLMService constructor injection pattern: LLMService(api_key=decrypted_key) for per-request BYOK key use
    - settings.OPENAI_API_KEY retained as dev/test fallback only — removed when Phase 3 wires full per-request flow

key-files:
  created:
    - apps/efofx-estimate/tests/services/test_llm_service.py
  modified:
    - .planning/REQUIREMENTS.md
    - apps/efofx-estimate/app/services/llm_service.py

key-decisions:
  - "BYOK-04 docs corrected: LLM endpoints return 402 when no BYOK key stored (no platform fallback) — aligns with locked decision from CONTEXT.md"
  - "LLMService api_key fallback to settings.OPENAI_API_KEY retained ONLY for dev/testing; WILL be removed in Phase 3 (LLM-01) per-request injection plan"

patterns-established:
  - "Phase 3 BYOK caller pattern: key = await decrypt_tenant_openai_key(tenant_id); llm = LLMService(api_key=key)"

requirements-completed: [BYOK-02, BYOK-04]

# Metrics
duration: 10min
completed: 2026-02-27
---

# Phase 2 Plan 07: Gap Closure — BYOK-04 Docs and LLMService BYOK Wiring Summary

**BYOK-04 requirement corrected to match 402-gate implementation; LLMService now accepts per-request api_key for tenant BYOK key injection in Phase 3**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-02-27T14:44:50Z
- **Completed:** 2026-02-27T14:55:00Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Fixed REQUIREMENTS.md BYOK-04 text: removed "Trial tier tenants use platform fallback OpenAI key"; replaced with "LLM endpoints return 402 when no BYOK OpenAI key is stored (no platform fallback)" — eliminates Phase 2 verification Gap 2
- Refactored LLMService.__init__ to accept optional api_key parameter with fallback to settings.OPENAI_API_KEY; added class docstring explaining the BYOK injection pattern — closes Phase 2 verification Gap 3
- Wrote 6 unit tests proving the BYOK key contract: key passed to AsyncOpenAI, fallback behavior, key flows through to API calls, generate_estimation returns EstimationOutput, api_key stored on instance

## Task Commits

Each task was committed atomically:

1. **Task 1: Update REQUIREMENTS.md BYOK-04 and refactor LLMService** - `d4ee1be` (fix)
2. **Task 2: Write unit tests for BYOK-wired LLMService** - `8b2cbb7` (test)

**Plan metadata:** (docs commit — see below)

## Files Created/Modified

- `.planning/REQUIREMENTS.md` - BYOK-04 text corrected to match implementation
- `apps/efofx-estimate/app/services/llm_service.py` - Added optional api_key parameter, api_key instance attribute, class docstring with BYOK injection pattern
- `apps/efofx-estimate/tests/services/test_llm_service.py` - 6 unit tests for BYOK key injection contract

## Decisions Made

- BYOK-04 docs corrected to match implementation: 402 gate with no platform fallback. The REQUIREMENTS.md text was wrong — the codebase and locked decision both say no platform fallback. Updated docs to match reality.
- settings.OPENAI_API_KEY fallback retained in LLMService: needed for dev/testing until Phase 3 (LLM-01) wires per-request client injection. The fallback is explicitly documented as temporary and will be removed.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 3 (LLM-01) can now call `LLMService(api_key=decrypted_key)` without changes to LLMService
- The full BYOK loop is architecturally complete: store key (02-04) -> decrypt per-request (byok_service.py) -> inject into LLMService (ready)
- Phase 2 verification Gaps 2 and 3 are fully closed

---
*Phase: 02-multi-tenant-foundation*
*Completed: 2026-02-27*

## Self-Check: PASSED

- FOUND: .planning/phases/02-multi-tenant-foundation/02-07-SUMMARY.md
- FOUND: .planning/REQUIREMENTS.md
- FOUND: apps/efofx-estimate/app/services/llm_service.py
- FOUND: apps/efofx-estimate/tests/services/test_llm_service.py
- FOUND commit: d4ee1be (fix BYOK-04 docs and LLMService)
- FOUND commit: 8b2cbb7 (test LLMService BYOK injection)
