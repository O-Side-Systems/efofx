---
phase: 01-prerequisites
plan: 01
subsystem: api
tags: [fastapi, mongodb, motor, pytest, pytest-asyncio, tenant-isolation, security]

requires: []
provides:
  - "DB_COLLECTIONS import fix in security.py (eliminates NameError on every auth request)"
  - "Tenant-scoped $or query in rcf_engine.py (eliminates cross-tenant data leak)"
  - "Integration test suite proving zero cross-tenant leakage at the MongoDB query level"
affects:
  - 01-02-PLAN.md
  - Phase 2 TenantAwareCollection middleware work

tech-stack:
  added: [pytest-asyncio==1.3.0 (upgraded from 0.23.5 for pytest 8.4.1 compatibility)]
  patterns:
    - "Per-test Motor client pattern: create AsyncIOMotorClient in fixture, inject into app._mdb globals, close after yield"
    - "Tenant-scoped MongoDB $or query: {category: X, $or: [{tenant_id: T}, {tenant_id: null}]}"

key-files:
  created:
    - apps/efofx-estimate/tests/services/test_tenant_isolation.py
  modified:
    - apps/efofx-estimate/app/core/security.py
    - apps/efofx-estimate/app/services/rcf_engine.py

key-decisions:
  - "Minimal query patch only: add $or clause to existing query in find_matching_reference_class(); Phase 2 builds TenantAwareCollection middleware for full coverage"
  - "Per-test Motor client in test fixture: avoids event-loop conflicts between session-scoped conftest DB and function-scoped test loops in pytest-asyncio strict mode"
  - "pytest-asyncio upgraded from 0.23.5 to 1.3.0: version 0.23.5 incompatible with pytest 8.4.1 (FixtureDef.unittest attribute removed)"

patterns-established:
  - "Integration tests for rcf_engine use @pytest.mark.integration + @pytest.mark.asyncio + @pytest_asyncio.fixture, excluded from unit-only runs with -m 'not integration'"
  - "Tenant isolation: $or clause always merges tenant-owned docs (tenant_id=T) and platform docs (tenant_id=None); no-tenant queries return platform-only"

requirements-completed: [PRQT-02, PRQT-03]

duration: 4min
completed: 2026-02-26
---

# Phase 1 Plan 1: Bug Fixes and Tenant Isolation Summary

**DB_COLLECTIONS NameError fixed in security.py, tenant-scoped $or query added to rcf_engine.py, and 4-test integration suite proving zero cross-tenant MongoDB leakage**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-02-26T19:50:50Z
- **Completed:** 2026-02-26T19:54:50Z
- **Tasks:** 2
- **Files modified:** 3 (security.py, rcf_engine.py, test_tenant_isolation.py)

## Accomplishments

- Fixed PRQT-03: `from app.core.constants import API_MESSAGES, HTTP_STATUS, DB_COLLECTIONS` — one-line import fix eliminates NameError on every API authentication request
- Fixed PRQT-02: Replaced `query = {"category": category}` in `find_matching_reference_class()` with `$or` clause filtering by `tenant_id` and platform data (`tenant_id=None`)
- Created 4-test integration suite (`test_tenant_isolation.py`) that connects directly to MongoDB and proves zero cross-tenant leakage at the query level

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix DB_COLLECTIONS NameError and tenant-scoped query** - `ac97b96` (fix)
2. **Task 2: Write integration test for tenant isolation** - `0656cd0` (feat)

## Files Created/Modified

- `apps/efofx-estimate/app/core/security.py` - Added `DB_COLLECTIONS` to import line 16
- `apps/efofx-estimate/app/services/rcf_engine.py` - Replaced bare category query with `$or` tenant-scoped query
- `apps/efofx-estimate/tests/services/test_tenant_isolation.py` - New: 4 integration tests for tenant isolation

## Decisions Made

- Used minimal query patch (not TenantAwareCollection middleware) — Phase 2 builds the proper middleware layer; this fix is the smallest safe change to unblock auth
- Per-test Motor client in fixture avoids event-loop conflicts: each test creates `AsyncIOMotorClient`, injects it into `app.db.mongodb._client/_database` globals, and closes it after yield
- pytest-asyncio upgraded from 0.23.5 to 1.3.0 because `FixtureDef.unittest` attribute was removed in pytest 8.4.1

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] pytest-asyncio 0.23.5 incompatible with pytest 8.4.1**
- **Found during:** Task 2 (tenant isolation integration tests)
- **Issue:** `AttributeError: 'FixtureDef' object has no attribute 'unittest'` — pytest-asyncio 0.23.5 accesses `FixtureDef.unittest` which was removed in pytest 8.4.1
- **Fix:** Upgraded pytest-asyncio from 0.23.5 to 1.3.0 via `pip install "pytest-asyncio>=0.24.0"` in the project venv
- **Files modified:** (pip-managed — no pyproject.toml change needed; venv .dist-info updated)
- **Verification:** All 4 integration tests pass after upgrade
- **Committed in:** 0656cd0 (Task 2 commit)

**2. [Rule 3 - Blocking] Motor event-loop isolation required per-test client**
- **Found during:** Task 2 (tenant isolation integration tests)
- **Issue:** The session-scoped `test_db` fixture in conftest.py creates a Motor client on a session event loop; each pytest-asyncio test in strict mode gets its own function-scoped event loop, causing "Task got Future attached to a different loop" RuntimeError
- **Fix:** Fixture creates a fresh `AsyncIOMotorClient` bound to the current test's event loop and injects it directly into `app.db.mongodb._client/_database` globals, bypassing the session-scoped connection
- **Files modified:** `apps/efofx-estimate/tests/services/test_tenant_isolation.py`
- **Verification:** All 4 tests pass (3.39s wall time with live Atlas connection)
- **Committed in:** 0656cd0 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 Rule 3 blocking)
**Impact on plan:** Both fixes essential to make integration tests runnable. No scope creep. Pre-existing TestRCFMatching failures in test_rcf_engine.py are out of scope (same event-loop issue, pre-existing before this plan).

## Issues Encountered

- Pre-existing `TestRCFMatching` test failures in `test_rcf_engine.py` (9 tests) exist before and after this plan — same Motor event-loop issue. Out of scope per deviation rule scope boundary. Logged to deferred-items.

## Next Phase Readiness

- Backend starts without NameError — auth requests no longer crash with 500
- rcf_engine.py queries are tenant-isolated at the MongoDB level
- Plan 01-02 (dependency migration: python-jose → PyJWT, passlib → pwdlib, etc.) can proceed

---
*Phase: 01-prerequisites*
*Completed: 2026-02-26*

## Self-Check: PASSED

- FOUND: apps/efofx-estimate/app/core/security.py
- FOUND: apps/efofx-estimate/app/services/rcf_engine.py
- FOUND: apps/efofx-estimate/tests/services/test_tenant_isolation.py
- FOUND: .planning/phases/01-prerequisites/01-01-SUMMARY.md
- FOUND commit: ac97b96 (fix DB_COLLECTIONS + tenant query)
- FOUND commit: 0656cd0 (tenant isolation tests)
