---
phase: 08-calibration-dashboard
plan: 01
subsystem: api
tags: [mongodb, fastapi, motor, aggregation, calibration, metrics, tenant-isolation]

# Dependency graph
requires:
  - phase: 07-feedback-email-magic-links
    provides: FeedbackDocument model with actual_cost, estimate_snapshot.total_cost_p50, reference_class_id, submitted_at stored in feedback collection
  - phase: 05-tech-debt-foundation-cleanup
    provides: TenantAwareCollection with aggregate() that prepends tenant $match to source collection
depends_on: []

provides:
  - migrate_synthetic_reference_classes() — idempotent CALB-01 migration tagging is_synthetic=True docs with data_source="synthetic"
  - CalibrationService.get_metrics(tenant_id, date_range) — mean_variance_pct, accuracy_buckets (exclusive slices), by_reference_class breakdown, threshold enforcement
  - CalibrationService.get_trend(tenant_id, months) — monthly time-series [{period, mean_variance_pct, outcome_count}] sorted chronologically
  - GET /api/v1/calibration/metrics — date_range query param (6months|1year|all)
  - GET /api/v1/calibration/trend — months query param (1-36)
  - CALIBRATION_THRESHOLD = 10 (CALB-03 minimum outcome guard)

affects: [08-02, 08-03, frontend-calibration-dashboard]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "$lookup with let/pipeline syntax for tenant isolation in inner pipelines (CALB-04) — TenantAwareCollection only scopes source, not joined collections"
    - "Exclusive accuracy bucket slices: [0-10], (10-20], (20-30], >30 as proportions summing to 1.0"
    - "Idempotent migration with $exists: False guard to prevent re-tagging on re-deploy"
    - "Two-stage threshold check: count_documents before aggregation to avoid expensive pipeline on sparse data"

key-files:
  created:
    - apps/efofx-estimate/app/services/calibration_service.py
    - apps/efofx-estimate/app/api/calibration.py
    - apps/efofx-estimate/tests/services/test_calibration_service.py
    - apps/efofx-estimate/tests/api/test_calibration.py
  modified:
    - apps/efofx-estimate/app/db/mongodb.py
    - apps/efofx-estimate/app/main.py

key-decisions:
  - "CalibrationService._build_pipeline returns pipeline without leading $match on tenant_id — TenantAwareCollection.aggregate() prepends that automatically; only $lookup inner pipeline needs explicit tenant scoping"
  - "get_trend threshold check counts ALL outcomes (no date filter), not just those in the months window — consistent with CALB-03 intent: minimum 10 real outcomes overall"
  - "Accuracy buckets are exclusive slices (not cumulative): within_10=[0,10], within_20=(10,20], within_30=(20,30], beyond_30=>30"
  - "API test pattern: use app.dependency_overrides[get_current_tenant] for auth mocking, not patch('app.api.calibration.get_current_tenant') which doesn't intercept FastAPI dependency resolution"

patterns-established:
  - "CALB-04 pattern: $lookup inner pipeline with let:{tenant: tenant_id} and $match $expr $and [$eq[$tenant_id,$$tenant]] — always needed when joining reference_classes from feedback context"
  - "Trend pipeline: $match -> $project(variance_pct + period) -> $group(_id=period) -> $sort(_id:1) -> $project(rename _id to period) — standard pattern for time-series aggregations"

requirements-completed: [CALB-01, CALB-02, CALB-03, CALB-04]

# Metrics
duration: 5min
completed: 2026-03-06
---

# Phase 08 Plan 01: CalibrationService and Calibration API Summary

**CalibrationService with tenant-scoped $lookup, exclusive accuracy buckets, monthly trend aggregation, and CALB-01 synthetic data migration via Motor/MongoDB**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-03-06T05:59:13Z
- **Completed:** 2026-03-06T06:03:56Z
- **Tasks:** 2 (TDD: RED + GREEN)
- **Files modified:** 6 (2 modified + 4 created)

## Accomplishments

- CALB-01: `migrate_synthetic_reference_classes()` added to `mongodb.py` — idempotent, tags `is_synthetic=True` docs with `data_source="synthetic"`, registered in lifespan
- CALB-02: `CalibrationService.get_metrics()` returns mean_variance_pct, 4-bucket accuracy distribution (exclusive slices), and per-reference-class breakdown with `limited_data` flag; `get_trend()` returns monthly time-series sorted chronologically
- CALB-03: Both methods return `{below_threshold: True, outcome_count, threshold}` when fewer than 10 real outcomes exist
- CALB-04: `$lookup` in `_build_pipeline()` uses `let/{$$tenant}` syntax to explicitly scope `tenant_id` in inner pipeline — security requirement since `TenantAwareCollection.aggregate()` only scopes the source collection
- 23 tests pass (18 service + 5 API), including CALB-04 isolation security tests

## Task Commits

1. **Task 1 (RED): Failing tests for CalibrationService, trend endpoint, and calibration API** - `0de8065` (test)
2. **Task 2 (GREEN): Implement CalibrationService, migration, metrics and trend endpoints** - `e85347c` (feat)

## Files Created/Modified

- `apps/efofx-estimate/app/services/calibration_service.py` - CalibrationService with get_metrics(), get_trend(), _build_pipeline(), _build_trend_pipeline(), helpers; CALIBRATION_THRESHOLD=10
- `apps/efofx-estimate/app/api/calibration.py` - calibration_router: GET /calibration/metrics + GET /calibration/trend
- `apps/efofx-estimate/app/db/mongodb.py` - Added migrate_synthetic_reference_classes() function
- `apps/efofx-estimate/app/main.py` - Registered migration in lifespan, included calibration_router at /api/v1
- `apps/efofx-estimate/tests/services/test_calibration_service.py` - 18 service tests
- `apps/efofx-estimate/tests/api/test_calibration.py` - 5 API tests

## Decisions Made

- **get_trend threshold counts all outcomes** (not just in the months window): ensures the minimum threshold applies to the tenant's overall data quality, not just a potentially sparse time slice
- **API test auth pattern**: `app.dependency_overrides[get_current_tenant]` required for FastAPI Depends() — patching the imported name at module level does not intercept dependency injection (deviation auto-fixed during GREEN task 2)
- **Accuracy buckets are exclusive slices**: [0,10], (10,20], (20,30], >30 — each variance falls into exactly one bucket, proportions sum to 1.0

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed API test mock strategy from patch() to dependency_overrides**
- **Found during:** Task 2 (GREEN — running tests after implementing API)
- **Issue:** `patch("app.api.calibration.get_current_tenant", return_value=_mock_tenant())` produced 401 because FastAPI resolves `Depends()` at request time, not at the module's import-time binding
- **Fix:** Changed to `app.dependency_overrides[get_current_tenant] = _mock_get_tenant` with proper cleanup in finally block
- **Files modified:** `apps/efofx-estimate/tests/api/test_calibration.py`
- **Verification:** Test `test_get_calibration_metrics_below_threshold` changed from 401 to 200 pass; all 23 tests green
- **Committed in:** `e85347c` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - bug in test mock strategy)
**Impact on plan:** Single test mock pattern fix — no scope creep, no implementation changes.

## Issues Encountered

None beyond the auto-fixed API test mock pattern.

## User Setup Required

None - no external service configuration required. Migration runs automatically on service startup.

## Next Phase Readiness

- CalibrationService backend complete — `/api/v1/calibration/metrics` and `/api/v1/calibration/trend` endpoints are production-ready
- CALB-01 migration will run on next deploy and tag existing synthetic reference class documents
- Ready for Phase 08-02: Frontend calibration dashboard consuming these endpoints
- Remaining concern: CALB-01 migration requires MongoDB connection on deploy — no action needed, standard pattern

## Self-Check: PASSED

- apps/efofx-estimate/app/services/calibration_service.py — FOUND
- apps/efofx-estimate/app/api/calibration.py — FOUND
- apps/efofx-estimate/tests/services/test_calibration_service.py — FOUND
- apps/efofx-estimate/tests/api/test_calibration.py — FOUND
- .planning/phases/08-calibration-dashboard/08-01-SUMMARY.md — FOUND
- Commit 0de8065 (RED test) — FOUND
- Commit e85347c (GREEN implementation) — FOUND

---
*Phase: 08-calibration-dashboard*
*Completed: 2026-03-06*
