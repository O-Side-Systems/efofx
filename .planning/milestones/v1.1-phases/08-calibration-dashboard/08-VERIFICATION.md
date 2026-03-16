---
phase: 08-calibration-dashboard
verified: 2026-03-15T00:00:00Z
status: human_needed
score: 12/12 must-haves verified
re_verification: false
human_verification:
  - test: "Start backend (uvicorn app.main:app --reload) and dashboard (npm run dev in apps/efofx-dashboard), open http://localhost:5174"
    expected: "Browser redirects to /login — no access_token in localStorage triggers the requireAuth loader"
    why_human: "Auth redirect requires a live browser session; can only verify router config statically"
  - test: "Log in with valid contractor credentials, then observe the dashboard"
    expected: "Either ThresholdProgress ('X of 10 outcomes recorded') or the full metrics view (stat cards + accuracy bar + trend line + reference class table)"
    why_human: "Conditional rendering on real API data — below_threshold vs above_threshold path requires live data"
  - test: "If above threshold: click each column header in the reference class table"
    expected: "Rows re-sort ascending/descending on each click; active column shows arrow indicator"
    why_human: "Sort interaction requires visual inspection in a live browser"
  - test: "Switch date range filter between '6 Months', '1 Year', 'All Time'"
    expected: "Active button turns accent-blue, data refetches silently; skeleton may briefly appear"
    why_human: "React Query queryKey-driven refetch requires live network to observe"
  - test: "Verify visual style matches Stripe/Linear aesthetic"
    expected: "Clean whitespace, muted colors (--color-text #1a1a2e, --color-bg #fafafa), cards with subtle borders, data-focused layout"
    why_human: "Subjective design quality cannot be verified programmatically"
---

# Phase 8: Calibration Dashboard Verification Report

**Phase Goal:** Contractors can see how accurate their estimates have been against real outcomes — accuracy metrics are displayed only when statistically meaningful, and synthetic data is never mixed into calibration calculations

**Verified:** 2026-03-15
**Status:** human_needed — all automated checks pass; 5 items require human browser verification
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | All synthetic reference class documents have data_source: "synthetic" after migration | VERIFIED | `migrate_synthetic_reference_classes()` in mongodb.py L278: filter `{is_synthetic: True, data_source: {$exists: False}}`, update `{$set: {data_source: "synthetic"}}`, idempotent guard confirmed |
| 2 | CalibrationService returns below_threshold response when fewer than 10 real outcomes exist | VERIFIED | calibration_service.py L297–302: `count < CALIBRATION_THRESHOLD` returns `{below_threshold: True, outcome_count, threshold}` in both `get_metrics()` and `get_trend()` |
| 3 | CalibrationService returns mean variance, accuracy buckets, and per-reference-class breakdown when 10+ outcomes exist | VERIFIED | calibration_service.py L336–344: full response shape with `mean_variance_pct`, `accuracy_buckets` (exclusive slices via `_compute_accuracy_buckets`), `by_reference_class` with `limited_data` flag |
| 4 | Every $lookup in the calibration pipeline explicitly filters tenant_id in its inner pipeline | VERIFIED | calibration_service.py L104–133: `$lookup` uses `let: {tenant: tenant_id}` with inner `$match $expr $and [$eq[$tenant_id, $$tenant]]` plus CALB-01 `$ne[$data_source, "synthetic"]` — 1 dedicated test confirms this |
| 5 | GET /api/v1/calibration/metrics returns correct JSON for both below-threshold and above-threshold states | VERIFIED | calibration.py L20–36: endpoint wired to `CalibrationService().get_metrics(tenant.tenant_id, date_range)`; 23 tests pass including `test_get_calibration_metrics_below_threshold` and `test_get_calibration_metrics_invalid_date_range` |
| 6 | GET /api/v1/calibration/trend returns monthly time-series array | VERIFIED | calibration.py L39–56: endpoint wired to `CalibrationService().get_trend(tenant.tenant_id, months)`; `_build_trend_pipeline` groups by YYYY-MM, sorts chronologically |
| 7 | CalibrationService.get_trend() groups feedback documents by month (submitted_at) and computes per-month mean variance | VERIFIED | calibration_service.py L196–253: `$project` emits `period: $dateToString {%Y-%m}`, `$group` by `_id: $period` with `$avg: $variance_pct`, `$sort {_id: 1}` |
| 8 | Dashboard app starts on port 5174 with Vite dev server | VERIFIED | vite.config.ts confirmed `server: {port: 5174}`; `npm run build` succeeds (vite 7.3.1, 1071 modules) |
| 9 | Unauthenticated users are redirected to /login | VERIFIED | router.tsx L5–8: `requireAuth()` throws `redirect('/login')` when `localStorage.getItem('access_token')` is null — requires human to confirm in browser |
| 10 | API requests include JWT Bearer token from localStorage | VERIFIED | api/client.ts L7–11: axios interceptor reads `localStorage.getItem('access_token')` and sets `Authorization: Bearer {token}` |
| 11 | Contractors with fewer than 10 outcomes see a progress bar with "X of 10 outcomes recorded" and no metrics | VERIFIED | ThresholdProgress.tsx L13–15: renders `{outcomeCount} of {threshold} outcomes recorded` with CSS progress bar; Dashboard.tsx L35–39 renders ONLY ThresholdProgress when `data.below_threshold` — no charts wired |
| 12 | Contractors with 10+ outcomes see mean variance headline, stacked accuracy bar, trend line chart, and reference class table | VERIFIED | Dashboard.tsx L42–63: conditional block renders CalibrationMetrics, AccuracyBucketBar, AccuracyTrendLine, ReferenceClassTable when `!data.below_threshold` |

**Score:** 12/12 truths verified (automated) + 5 items flagged for human verification

---

## Required Artifacts

| Artifact | Provides | Status | Details |
|----------|----------|--------|---------|
| `apps/efofx-estimate/app/services/calibration_service.py` | CalibrationService with get_metrics(), get_trend(), tenant-scoped aggregation | VERIFIED | 397 lines, full implementation, CALIBRATION_THRESHOLD=10, all helpers present |
| `apps/efofx-estimate/app/api/calibration.py` | GET /calibration/metrics and GET /calibration/trend | VERIFIED | calibration_router exported, both endpoints with auth dependency |
| `apps/efofx-estimate/tests/services/test_calibration_service.py` | Unit tests for CalibrationService | VERIFIED | 18 test functions |
| `apps/efofx-estimate/tests/api/test_calibration.py` | Unit tests for calibration API | VERIFIED | 5 test functions; all 23 pass |
| `apps/efofx-dashboard/package.json` | React 19, Recharts, React Query, react-router, axios | VERIFIED | react@^19.2.0, recharts@^2.15.3, @tanstack/react-query@^5.83.0, react-router@^7.5.0, axios@^1.9.0 |
| `apps/efofx-dashboard/src/router.tsx` | Client-side routing with auth guard | VERIFIED | requireAuth loader throws redirect('/login'), Dashboard route protected |
| `apps/efofx-dashboard/src/api/client.ts` | Axios instance with JWT interceptor | VERIFIED | localStorage access_token injected into Authorization header |
| `apps/efofx-dashboard/src/hooks/useCalibration.ts` | React Query hook for calibration metrics | VERIFIED | useQuery with queryKey ['calibration', dateRange], wraps fetchCalibrationMetrics |
| `apps/efofx-dashboard/src/hooks/useCalibrationTrend.ts` | React Query hook for trend time-series | VERIFIED | useQuery with queryKey ['calibrationTrend', months], wraps fetchCalibrationTrend |
| `apps/efofx-dashboard/src/types/calibration.ts` | TypeScript types matching CalibrationService responses | VERIFIED | All 5 required interfaces exported: CalibrationMetrics, AccuracyBuckets, ReferenceClassBreakdown, CalibrationTrendPoint, CalibrationTrendResponse |
| `apps/efofx-dashboard/src/components/ThresholdProgress.tsx` | Progress bar for below-threshold state | VERIFIED | Renders "X of {threshold} outcomes recorded", CSS progress bar, explanation text |
| `apps/efofx-dashboard/src/components/CalibrationMetrics.tsx` | Summary stats cards | VERIFIED | Exists and is imported + rendered in Dashboard.tsx |
| `apps/efofx-dashboard/src/components/AccuracyBucketBar.tsx` | Stacked horizontal bar chart | VERIFIED | Recharts BarChart layout="vertical", 4 stacked Bar components with green/yellow/orange/red (#22c55e / #eab308 / #f97316 / #ef4444), stackId="a" |
| `apps/efofx-dashboard/src/components/AccuracyTrendLine.tsx` | Line chart for monthly trend | VERIFIED | Imports useCalibrationTrend, Recharts LineChart with formatPeriod date-fns formatter, renders null when below_threshold |
| `apps/efofx-dashboard/src/components/ReferenceClassTable.tsx` | Sortable reference class table | VERIFIED | useState sort state, handleSort toggles direction, limited_data rows get "Limited data" caveat |
| `apps/efofx-dashboard/src/components/DateRangeFilter.tsx` | 3-button date range selector | VERIFIED | "6months", "1year", "all" options with active class styling |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `app/api/calibration.py` | `app/services/calibration_service.py` | `CalibrationService().get_metrics()` and `.get_trend()` | WIRED | calibration.py L35: `await svc.get_metrics(tenant.tenant_id, date_range)` / L55: `await svc.get_trend(tenant.tenant_id, months)` |
| `app/main.py` | `app/db/mongodb.py` | `migrate_synthetic_reference_classes()` in lifespan | WIRED | main.py L25 imports the function; L45 calls `await migrate_synthetic_reference_classes()` in lifespan |
| `app/main.py` | `app/api/calibration.py` | `app.include_router(calibration_router)` | WIRED | main.py L23 imports calibration_router; L147: `app.include_router(calibration_router, prefix="/api/v1")` |
| `src/api/client.ts` | localStorage access_token | axios interceptor injects Authorization header | WIRED | client.ts L8: `localStorage.getItem('access_token')` → `config.headers.Authorization = 'Bearer {token}'` |
| `src/hooks/useCalibration.ts` | `src/api/calibration.ts` | React Query wraps metrics API call | WIRED | useCalibration.ts L7: `queryFn: () => fetchCalibrationMetrics(dateRange)` |
| `src/hooks/useCalibrationTrend.ts` | `src/api/calibration.ts` | React Query wraps trend API call | WIRED | useCalibrationTrend.ts L7: `queryFn: () => fetchCalibrationTrend(months)` |
| `src/router.tsx` | `src/pages/Dashboard.tsx` | requireAuth loader redirects to /login if no token | WIRED | router.tsx L7: `if (!token) throw redirect('/login')` as loader on Dashboard route |
| `src/pages/Dashboard.tsx` | `src/hooks/useCalibration.ts` | `useCalibration(dateRange)` hook call | WIRED | Dashboard.tsx L13: `const { data, isPending, isError, error, refetch } = useCalibration(dateRange)` |
| `src/pages/Dashboard.tsx` | `ThresholdProgress / CalibrationMetrics` | conditional render on `data.below_threshold` | WIRED | Dashboard.tsx L35–39: ThresholdProgress when `data.below_threshold`; L42–63: full metrics when `!data.below_threshold` |
| `src/components/AccuracyBucketBar.tsx` | Recharts BarChart | stacked horizontal bar | WIRED | AccuracyBucketBar.tsx L76: `<BarChart data={data} layout="vertical">` with stackId="a" on all Bar components |
| `src/components/AccuracyTrendLine.tsx` | `src/hooks/useCalibrationTrend.ts` | `useCalibrationTrend(12)` | WIRED | AccuracyTrendLine.tsx L55: `const { data, isPending, isError } = useCalibrationTrend(12)` |
| `src/components/AccuracyTrendLine.tsx` | Recharts LineChart | monthly time-series | WIRED | AccuracyTrendLine.tsx L86: `<LineChart data={data.trend}>` with `Line dataKey="mean_variance_pct"` |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| CALB-01 | 08-01 | Tag existing synthetic reference classes with data_source: "synthetic" | SATISFIED | `migrate_synthetic_reference_classes()` in mongodb.py L278–304; idempotent `$exists: False` guard; called in lifespan at main.py L45 |
| CALB-02 | 08-01 | Calibration metrics API — mean variance, accuracy buckets (10/20/30%), per-reference-class breakdown | SATISFIED | `get_metrics()` returns `mean_variance_pct`, `accuracy_buckets` (exclusive slices summing to 1.0), `by_reference_class` with `limited_data` flag; `get_trend()` returns monthly time-series; 18 service tests pass |
| CALB-03 | 08-01 | Minimum 10 real outcome threshold enforced before displaying any metrics | SATISFIED | CALIBRATION_THRESHOLD=10; both `get_metrics()` L297 and `get_trend()` L366 return `{below_threshold: True}` when count < 10; ThresholdProgress renders "X of 10 outcomes recorded" in UI |
| CALB-04 | 08-01 | Tenant-scoped $lookup aggregation with explicit tenant_id in inner pipeline | SATISFIED | `_build_pipeline()` L108: `let: {tenant: tenant_id}` with `$match $expr $and [$eq[$tenant_id, $$tenant], $ne[$data_source, synthetic]]`; dedicated test `test_lookup_pipeline_includes_tenant_id` verifies this |
| CALB-05 | 08-02, 08-03 | Calibration dashboard app with Recharts charts, sortable table, date range filtering | SATISFIED | apps/efofx-dashboard/ built successfully; AccuracyBucketBar (Recharts 2.x stacked bar), AccuracyTrendLine (Recharts LineChart), ReferenceClassTable (sortable), DateRangeFilter (3 options) all wired in Dashboard.tsx; `npm run build` passes with TypeScript clean |
| CALB-06 | 08-03 | Dashboard shows progress indicator below minimum threshold | SATISFIED | ThresholdProgress.tsx renders "X of 10 outcomes recorded" with progress bar; Dashboard.tsx renders ONLY ThresholdProgress when `data.below_threshold` — no partial metrics shown |

All 6 CALB requirements satisfied. No orphaned requirements found.

---

## Anti-Patterns Found

No blockers or warnings found.

- All `return null` instances in components are legitimate guard clauses (Recharts tooltip hidden when inactive; AccuracyTrendLine hidden when below_threshold — both correct behavior per spec)
- No TODO/FIXME/PLACEHOLDER comments in any phase 08 files
- No stub implementations (empty handlers, unimplemented returns)
- TypeScript compiles clean, build succeeds

---

## Human Verification Required

### 1. Auth Guard Redirect

**Test:** Open http://localhost:5174 with no token in localStorage (fresh browser tab or clear localStorage)
**Expected:** Browser redirects to /login immediately
**Why human:** React Router loader redirect requires live browser — static analysis confirms the code is correct but not that the browser navigation occurs

### 2. Dashboard Data Rendering

**Test:** Log in with valid contractor credentials; observe the dashboard page
**Expected:** One of two states: (a) ThresholdProgress showing "X of 10 outcomes recorded" with progress bar and explanation text, or (b) full dashboard with mean variance card, accuracy bucket bar chart (green/yellow/orange/red stacked), accuracy trend line, and reference class table
**Why human:** Conditional rendering is correct in code, but actual data flow from the API to rendered components needs visual confirmation

### 3. Reference Class Table Sorting

**Test:** If above threshold, click the "Count", "Mean Variance", and "Reference Class" column headers
**Expected:** Rows reorder on each click; active column shows arrow (up/down); double-click same header toggles direction
**Why human:** Interactive sort requires live browser to verify re-render behavior

### 4. Date Range Filter Refetch

**Test:** Click "6 Months", then "1 Year", then "All Time" in the date range filter
**Expected:** Active button highlights in --color-accent blue; data refreshes (brief loading state possible); metrics may change if date-filtered data differs
**Why human:** React Query queryKey-driven refetch requires live network + API to observe the fetch cycle

### 5. Visual Design Quality

**Test:** Assess overall visual style of the dashboard
**Expected:** Clean, data-focused layout with lots of whitespace; muted dark navy text on light gray background; cards with subtle borders; accent blue for interactive elements; no cluttered UI
**Why human:** Subjective aesthetic quality judgment (Stripe/Linear aesthetic per user decision)

---

## Summary

All 12 observable truths verified. All 16 artifacts exist and are substantive (no stubs). All 12 key links confirmed wired. All 6 CALB requirements satisfied with direct code evidence. 23 backend tests pass. Dashboard TypeScript compiles clean and builds successfully.

The only remaining gate is human visual/functional verification — the code is correct and complete, but 5 items (auth redirect, data rendering, sort interaction, filter refetch, visual design) require a live browser to confirm.

Phase 8 is automated-complete. Awaiting human verification of Task 3 checkpoint from Plan 08-03.

---

_Verified: 2026-03-15_
_Verifier: Claude (gsd-verifier)_
