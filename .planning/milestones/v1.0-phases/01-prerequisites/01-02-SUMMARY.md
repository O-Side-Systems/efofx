---
phase: 01-prerequisites
plan: 02
subsystem: infra
tags: [pyjwt, pwdlib, openai, structured-outputs, pydantic, python-311, security]

# Dependency graph
requires:
  - phase: 01-01
    provides: Bug fixes and DB_COLLECTIONS patch — codebase in clean state for dependency migration
provides:
  - PyJWT-based JWT auth (security.py migrated from python-jose)
  - OpenAI v2 structured output via client.beta.chat.completions.parse()
  - EstimationOutput Pydantic model (CostCategoryEstimate, AdjustmentFactor)
  - Python 3.11 runtime pin (runtime.txt) for DigitalOcean App Platform
  - Clean dependency set: no python-jose CVE, no passlib, openai>=2.20.0
affects:
  - 02-tenant-isolation (auth layer uses PyJWT security.py)
  - 03-rcf-engine (LLM structured output is the estimation return type)
  - All phases deploying to DigitalOcean (runtime.txt sets Python 3.11)

# Tech tracking
tech-stack:
  added:
    - PyJWT==2.11.0 (replaces python-jose[cryptography]==3.3.0)
    - pwdlib[bcrypt]==0.3.0 (replaces passlib[bcrypt]==1.7.4)
    - openai>=2.20.0 (replaces openai==1.51.0)
  patterns:
    - OpenAI v2 structured output: client.beta.chat.completions.parse(response_format=PydanticModel)
    - PyJWT exception handling: except jwt.InvalidTokenError (not JWTError)
    - P50/P80 estimate schema: CostCategoryEstimate per-category, AdjustmentFactor multipliers, 0-100 confidence score

key-files:
  created:
    - apps/efofx-estimate/runtime.txt
    - apps/efofx-estimate/app/models/estimation.py (EstimationOutput, CostCategoryEstimate, AdjustmentFactor added)
  modified:
    - apps/efofx-estimate/requirements.txt
    - apps/efofx-estimate/pyproject.toml
    - apps/efofx-estimate/app/core/security.py
    - apps/efofx-estimate/app/core/config.py
    - apps/efofx-estimate/app/services/llm_service.py
    - apps/efofx-estimate/.do/app.yaml

key-decisions:
  - "PyJWT 2.x encode() returns str directly — no .decode() needed on result"
  - "jwt.InvalidTokenError is the correct PyJWT exception for token validation failures (not JWTError)"
  - "EstimationOutput uses P50/P80 per-category (not a flat total) with named AdjustmentFactor multipliers — matches user's transparency requirement"
  - "generate_estimation() returns typed EstimationOutput, not Dict[str,Any] — callers expecting dict should call .model_dump()"
  - "gpt-4o-mini required for client.beta.chat.completions.parse() structured outputs"
  - "pytest-asyncio bumped to 1.3.0 to match pytest 8.4.1 (was already decided in 01-01)"

patterns-established:
  - "OpenAI structured output: always use client.beta.chat.completions.parse() with Pydantic response_format — never parse free-form LLM text"
  - "Dependency migration: update both requirements.txt and pyproject.toml in lockstep"
  - "PyJWT import: import jwt (not from jose import jwt); except jwt.InvalidTokenError"

requirements-completed: [PRQT-01, PRQT-04, PRQT-05]

# Metrics
duration: 2min
completed: 2026-02-26
---

# Phase 1 Plan 02: Dependency Migration Summary

**Eliminated python-jose CVE-2025-61152 and openai v1 stub by migrating to PyJWT==2.11.0, pwdlib==0.3.0, openai>=2.20.0 with real structured LLM output via client.beta.chat.completions.parse()**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-26T19:58:04Z
- **Completed:** 2026-02-26T20:00:42Z
- **Tasks:** 2 of 2
- **Files modified:** 8

## Accomplishments

- Replaced all three abandoned/vulnerable dependencies (python-jose, passlib, openai v1) with maintained alternatives
- Migrated security.py JWT auth from python-jose to PyJWT — correct import and exception handling
- Replaced `_parse_estimation_response()` hardcoded stub with real OpenAI v2 structured output (`client.beta.chat.completions.parse()`)
- Created `EstimationOutput` Pydantic model with P50/P80 ranges, named adjustment factors, and typed confidence score
- Pinned Python 3.11 runtime for DigitalOcean App Platform via `runtime.txt`
- Updated OPENAI_MODEL default and DigitalOcean env var to `gpt-4o-mini` (required for structured outputs)

## Task Commits

Each task was committed atomically:

1. **Task 1: Replace abandoned dependencies and migrate security.py to PyJWT** - `d907b05` (feat)
2. **Task 2: Replace LLM parsing stub with OpenAI v2 structured output** - `d244b47` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `apps/efofx-estimate/requirements.txt` - PyJWT==2.11.0, pwdlib[bcrypt]==0.3.0, openai>=2.20.0; removed python-jose and passlib
- `apps/efofx-estimate/pyproject.toml` - Same dep changes; requires-python >=3.11; black/mypy targets py311
- `apps/efofx-estimate/app/core/security.py` - `import jwt` (PyJWT); `except jwt.InvalidTokenError`
- `apps/efofx-estimate/app/core/config.py` - OPENAI_MODEL default changed to gpt-4o-mini
- `apps/efofx-estimate/app/services/llm_service.py` - Real structured output; removed stubs; typed return EstimationOutput
- `apps/efofx-estimate/app/models/estimation.py` - Added EstimationOutput, CostCategoryEstimate, AdjustmentFactor models
- `apps/efofx-estimate/.do/app.yaml` - OPENAI_MODEL env var updated to gpt-4o-mini
- `apps/efofx-estimate/runtime.txt` - Created with python-3.11.14 for DigitalOcean buildpack

## Decisions Made

- `generate_estimation()` now returns `EstimationOutput` (typed Pydantic model), not `Dict[str, Any]`. Any callers needing a dict should call `.model_dump()`.
- The `EstimationOutput` schema uses P50/P80 per cost category (not a flat breakdown) with named `AdjustmentFactor` multipliers — this matches the product requirement for transparent, explainable estimates.
- Confidence score is 0–100 (not 0.0–1.0) for readability with end users.
- `generate_response()` kept for classify_project and estimation_service flows that still use free-form LLM calls.

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Phase 1 complete: all abandoned dependencies replaced, JWT auth on PyJWT, LLM estimation returns real structured AI output
- Phase 2 (tenant isolation) can proceed — security.py is clean and ready for TenantAwareCollection middleware work
- Callers of `llm_service.generate_estimation()` (if any outside llm_service.py) should be updated to accept `EstimationOutput` instead of `Dict[str, Any]` — the current `estimation_service.py` uses `generate_response()` directly, not `generate_estimation()`, so no immediate breaking change

---
*Phase: 01-prerequisites*
*Completed: 2026-02-26*
