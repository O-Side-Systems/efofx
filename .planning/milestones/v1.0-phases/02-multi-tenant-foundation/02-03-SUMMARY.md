---
phase: 02-multi-tenant-foundation
plan: 03
subsystem: data-access-layer
tags: [tenant-isolation, mongodb, tdd, security, refactor]
dependency_graph:
  requires:
    - 01-01-SUMMARY.md  # Phase 1 $or patch replaced by this plan
    - 01-02-SUMMARY.md  # PyJWT/pwdlib/openai migration
  provides:
    - TenantAwareCollection wrapper (structural tenant isolation)
    - get_tenant_collection() factory (all services use this)
    - Compound MongoDB indexes with tenant_id as first field
  affects:
    - 02-04  # BYOK encryption will use same tenant isolation pattern
    - 02-05  # Rate limiting reads tenant_id; same scoping applies
tech_stack:
  added:
    - TenantAwareCollection (app/db/tenant_collection.py)
    - get_tenant_collection() factory (app/db/mongodb.py)
  patterns:
    - TDD (RED → GREEN commit per test class)
    - Wrapper pattern for transparent filter injection
    - Factory function for centralized collection access
key_files:
  created:
    - apps/efofx-estimate/app/db/tenant_collection.py
    - apps/efofx-estimate/tests/db/__init__.py
    - apps/efofx-estimate/tests/db/test_tenant_collection.py
    - apps/efofx-estimate/tests/db/test_indexes.py
  modified:
    - apps/efofx-estimate/app/db/mongodb.py
    - apps/efofx-estimate/app/services/rcf_engine.py
    - apps/efofx-estimate/app/services/estimation_service.py
    - apps/efofx-estimate/app/services/feedback_service.py
    - apps/efofx-estimate/app/services/chat_service.py
    - apps/efofx-estimate/app/services/reference_service.py
    - apps/efofx-estimate/tests/services/test_tenant_isolation.py
decisions:
  - "TenantAwareCollection enforced at construction — ValueError on empty/None tenant_id makes mis-use impossible"
  - "Per-operation collection instantiation (not stored in __init__) — avoids tenant_id drift across request lifecycle"
  - "reference_service methods accept optional tenant_id — None means platform admin; TenantAwareCollection not used for no-tenant path (raw collection with explicit tenant_id=None filter)"
  - "tenant_service.py deferred — admin/cross-tenant stats methods intentionally bypass tenant scoping; not in plan scope"
metrics:
  duration: 7 min
  completed_date: "2026-02-26"
  tasks_completed: 2
  tasks_total: 2
  files_created: 4
  files_modified: 7
---

# Phase 02 Plan 03: TenantAwareCollection — Hard Tenant Isolation Summary

TDD implementation of TenantAwareCollection — a Motor wrapper that auto-injects tenant_id into every MongoDB operation (find, insert, update, delete, aggregate), backed by compound indexes where tenant_id is the leftmost field.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | TDD — Write tests and implement TenantAwareCollection | 773e392 | tenant_collection.py, tests/db/__init__.py, tests/db/test_tenant_collection.py |
| 2 | Factory, compound indexes, refactor all services | cf7e713 | mongodb.py, rcf_engine.py, estimation_service.py, feedback_service.py, chat_service.py, reference_service.py, tests/db/test_indexes.py, test_tenant_isolation.py |

## What Was Built

### TenantAwareCollection (`app/db/tenant_collection.py`)

Wraps `AsyncIOMotorCollection` and auto-injects `tenant_id` on every operation:

```python
col = TenantAwareCollection(raw_motor_collection, "acme-corp")
await col.find_one({})          # → filter: {"tenant_id": "acme-corp"}
await col.find_one({"x": 1})   # → filter: {"$and": [{"tenant_id": "acme-corp"}, {"x": 1}]}
await col.insert_one({"k": 1}) # → doc: {"k": 1, "tenant_id": "acme-corp"}
```

Platform data mode (reference classes visible to all tenants):
```python
col = TenantAwareCollection(raw_col, "acme-corp", allow_platform_data=True)
await col.find({})  # → {"$or": [{"tenant_id": "acme-corp"}, {"tenant_id": None}]}
```

### get_tenant_collection() Factory (`app/db/mongodb.py`)

Single entry point for all tenant data access:
```python
col = get_tenant_collection("estimates", tenant_id)
col = get_tenant_collection("reference_classes", tenant_id, allow_platform_data=True)
```

### Compound Indexes (`create_indexes()`)

All tenant-scoped collections now have `tenant_id` as the leftmost index key:
- `estimates`: `(tenant_id, created_at)`, `(tenant_id, session_id)` UNIQUE
- `reference_classes`: `(tenant_id, category)`, `(tenant_id, name)`
- `reference_projects`: `(tenant_id, reference_class)`, `(tenant_id, region)`
- `feedback`: `(tenant_id, estimation_session_id)`, `(tenant_id, created_at)`
- `chat_sessions`: `(tenant_id, session_id)` UNIQUE

### Service Refactoring

All five services now use `get_tenant_collection()` instead of raw collection access:
- `rcf_engine.py`: Phase 1 `$or` clause removed; `TenantAwareCollection(allow_platform_data=True)` handles it structurally
- `estimation_service.py`, `feedback_service.py`, `chat_service.py`: Collection obtained per-operation via `_collection(tenant_id)` helper
- `reference_service.py`: Methods now accept optional `tenant_id`; `list_reference_classes()` alias added for API routes compatibility

## Test Results

- **Unit tests (no DB):** 22 new tests in `test_tenant_collection.py` — all pass via AsyncMock
- **Integration tests:** 8 new integration tests across `test_indexes.py` (6) and `test_tenant_isolation.py` (4) — marked `@pytest.mark.integration`
- **Full suite (unit only):** 71 passed, 0 failed

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Missing MASTER_ENCRYPTION_KEY env var**
- **Found during:** Task 1 (test collection)
- **Issue:** `app/core/config.py` requires `MASTER_ENCRYPTION_KEY` (added in 02-01) but `.env` was missing it, blocking all tests
- **Fix:** Added `MASTER_ENCRYPTION_KEY=rf8BjQXmLjFZdjkgxixzY2Gajw6DVCpBH8XvpNI5omc=` to `.env` (same as `ENCRYPTION_KEY`)
- **Files modified:** `.env` (gitignored)

**2. [Rule 2 - Missing functionality] reference_service.py had list_reference_classes missing**
- **Found during:** Task 2 (reading routes.py usage)
- **Issue:** `routes.py` calls `reference_service.list_reference_classes(category)` but the method was named `get_reference_classes()` — would cause AttributeError at runtime
- **Fix:** Added `list_reference_classes()` as an alias during the refactor
- **Files modified:** `apps/efofx-estimate/app/services/reference_service.py`

### Deferred Items

**1. `tenant_service.py` — admin statistics methods**
- `get_tenant_statistics()`, `validate_tenant_limits()`, `get_all_tenant_statistics()` still use deprecated `get_estimates_collection()` / `get_feedback_collection()` calls
- These are intentionally cross-tenant admin operations (platform-level stats across all tenants), not tenant-scoped. `get_all_tenant_statistics()` counts across all tenants by design.
- **Deferred to:** Phase 2 admin/tenant management plan — needs decision on admin auth pattern before refactoring

## Verification Checklist

- [x] TenantAwareCollection auto-injects tenant_id on every operation
- [x] Creating without valid tenant_id raises ValueError
- [x] Platform data (tenant_id=None) accessible via allow_platform_data=True
- [x] All target service files use get_tenant_collection() (rcf_engine, estimation, feedback, chat, reference)
- [x] Compound indexes have tenant_id as first field on all tenant-scoped collections
- [x] Phase 1 `$or` clause removed from rcf_engine.py (replaced by TenantAwareCollection)
- [x] All unit tests pass (71 total)

## Self-Check: PASSED

Files exist:
- `apps/efofx-estimate/app/db/tenant_collection.py` — FOUND
- `apps/efofx-estimate/app/db/mongodb.py` — FOUND (updated)
- `apps/efofx-estimate/tests/db/test_tenant_collection.py` — FOUND
- `apps/efofx-estimate/tests/db/test_indexes.py` — FOUND

Commits exist:
- `773e392` — feat(02-03): implement TenantAwareCollection wrapper
- `cf7e713` — feat(02-03): add get_tenant_collection factory, compound indexes, refactor all services
