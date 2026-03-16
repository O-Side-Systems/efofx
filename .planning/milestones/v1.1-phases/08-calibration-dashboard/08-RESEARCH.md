# Phase 8: Calibration Dashboard - Research

**Researched:** 2026-03-05
**Domain:** MongoDB aggregation + data migration (backend) / Vite + React 19 + Recharts (frontend)
**Confidence:** HIGH (backend patterns), MEDIUM (frontend stack versions)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Dashboard layout & structure**
- Single scrollable page — summary stats at top, charts in middle, reference class breakdown at bottom
- Minimal and data-focused visual style — clean cards, muted colors, emphasis on numbers and charts (like Stripe/Linear)
- Standalone Vite + React app at its own URL (/dashboard), not a route in an existing app
- Optional date range picker to filter calibration data (last 6 months, last year, all-time)

**Accuracy visualization**
- Headline display: single big mean variance number (e.g., ±12%) at the top
- Accuracy buckets (within 10/20/30% of actual) shown as a stacked horizontal bar — compact, easy to read at a glance
- Green-to-red gradient color scheme: within 10% = green, 20% = yellow, 30% = orange, >30% = red
- Line chart showing accuracy trend over time — helps contractors see if they're improving

**Threshold progress experience**
- Contractors with fewer than 10 real outcomes see a progress bar + count: "4 of 10 outcomes recorded"
- Brief explanation included: "We need at least 10 completed projects to calculate meaningful accuracy metrics"
- No fanfare when unlocking — dashboard simply loads with metrics when threshold is met
- Dashboard link always visible in navigation — clicking it below threshold shows the progress state

**Reference class breakdown**
- Sortable data table: Reference Class | Count | Mean Variance | Accuracy Bar (inline)
- Default sort by outcome count (most-used reference classes at top), clickable column headers to re-sort
- Reference classes with fewer than 5 outcomes shown but grayed out with "Limited data" caveat
- No drill-down into individual estimates for v1 — table shows summary stats only

### Claude's Discretion
- Loading skeleton and loading state design
- Exact spacing, typography, and card styling
- Error state handling and network failure UX
- Date range picker component choice and presets
- Trend line chart granularity (weekly, monthly, per-outcome)

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CALB-01 | Tag existing synthetic reference classes with data_source: "synthetic" | Migration pattern using Motor `update_many` on raw collection with `{"is_synthetic": True}` filter — idempotent `$set` with `$exists: false` guard |
| CALB-02 | Calibration metrics API — mean variance, accuracy buckets (10/20/30%), per-reference-class breakdown | MongoDB aggregation pipeline on `feedback` collection joining `reference_classes` via `$lookup` with `let`/`pipeline` inner scoping |
| CALB-03 | Minimum 10 real outcome threshold enforced before displaying any metrics | Service-layer count check before returning metrics; API returns `{"below_threshold": true, "outcome_count": N}` shape below threshold |
| CALB-04 | Tenant-scoped `$lookup` aggregation with explicit `tenant_id` in inner pipeline | `$lookup` must use `let`/`pipeline` syntax with `$match` + `$expr` inside the pipeline — TenantAwareCollection only scopes the source collection, NOT joined collections |
| CALB-05 | Calibration dashboard app (`apps/efofx-dashboard/`) with Recharts charts | Vite + React 19 + TypeScript scaffold using `npm create vite@latest` with `react-ts` template; Recharts 3.7.x; `@tanstack/react-query` 5.90.x |
| CALB-06 | Dashboard shows progress indicator below minimum threshold ("X more outcomes needed") | Frontend conditional render based on `below_threshold` flag from API; no partial metrics displayed |
</phase_requirements>

---

## Summary

Phase 8 has two completely independent workstreams: a Python/MongoDB backend (Plans 08-01 and 08-02) and a React frontend (Plans 08-03 and 08-04). The backend work builds on patterns already established in the codebase. The frontend work creates a net-new app in `apps/efofx-dashboard/`.

The most critical constraint is CALB-04: `TenantAwareCollection.aggregate()` prepends a `$match` stage on the *source* collection but does **not** scope any joined collection in a `$lookup`. The CalibrationService must explicitly filter `tenant_id` inside every `$lookup` inner pipeline using the `let`/`pipeline` syntax. Failing to do this would allow cross-tenant data joins — a security breach, not just a bug.

The data migration (CALB-01) is straightforward: existing synthetic reference class documents already have `is_synthetic: True` and `tenant_id: None`. The migration adds `data_source: "synthetic"` and is idempotent — safe to run on every deploy. The calibration aggregation simply filters `data_source != "synthetic"` to exclude them.

For the frontend, the existing `apps/efofx-widget/` provides the exact scaffold pattern (Vite + React 19 + TypeScript). The dashboard is a standard SPA, not a web component, so it uses normal `createRoot` mounting and `react-router` for navigation. Recharts 3.7.x is the version in use as of March 2026 and has resolved its React 19 compatibility issues.

**Primary recommendation:** Build backend and frontend in parallel (08-01/08-02 can run concurrently with 08-03/08-04 after the API contract is established).

---

## Standard Stack

### Core — Backend (Plans 08-01, 08-02)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Motor | 3.3.2 (pinned) | Async MongoDB driver | Already in use; Motor EOL is May 2026, migration is Phase 9+ concern |
| PyMongo | 4.6.1 (pinned) | Sync MongoDB for migration scripts | Already pinned in pyproject.toml |
| FastAPI | 0.116.1 (pinned) | API endpoint for calibration data | Existing app framework |
| Pydantic v2 | 2.11.7 (pinned) | Response model for CalibrationResponse | Existing validation layer |

### Core — Frontend (Plans 08-03, 08-04)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Vite | 6.x (latest via `npm create vite@latest`) | Build tool / dev server | Same as efofx-widget; React 19 HMR support |
| React | 19.x | UI framework | Locked decision; same as efofx-widget |
| TypeScript | ~5.9.x | Type safety | Same as efofx-widget (tsconfig pattern established) |
| Recharts | 3.7.x (`recharts@^3`) | Charts (BarChart, LineChart) | CALB-05 locked; React 19 compatible as of 3.x |
| @tanstack/react-query | 5.90.x | Server state / data fetching | Industry standard; replaces manual fetch+useEffect |
| react-router | 7.13.x | Client-side routing and auth guard | v7 packages are now unified under `react-router` (no more `react-router-dom`) |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| axios | ^1.x | HTTP client with interceptors | JWT token injection in request headers |
| date-fns | ^4.x | Date range formatting and math | Date picker presets (last 6 months, last year, all-time) |
| @tanstack/react-query-devtools | 5.x | Dev-only query inspection | Local development only |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| @tanstack/react-query | SWR | React Query has better devtools and more ecosystem adoption in 2025 |
| react-router v7 | TanStack Router | TanStack Router is excellent but adds complexity; React Router v7 is sufficient for a 2-route dashboard |
| Recharts | Victory / nivo | Recharts is the locked decision; it is the most widely used React chart library (24.8K GitHub stars) |
| axios | native fetch | Axios interceptors simplify JWT injection and refresh; fetch requires more boilerplate |

**Installation:**
```bash
# Frontend (apps/efofx-dashboard)
npm create vite@latest efofx-dashboard -- --template react-ts
cd efofx-dashboard
npm install recharts @tanstack/react-query react-router axios date-fns
npm install -D @tanstack/react-query-devtools @types/react @types/react-dom
```

---

## Architecture Patterns

### Recommended Project Structure

```
apps/efofx-dashboard/
├── src/
│   ├── api/
│   │   ├── client.ts          # axios instance + JWT interceptor
│   │   └── calibration.ts     # calibration API calls (React Query keys + fetchers)
│   ├── components/
│   │   ├── CalibrationMetrics.tsx   # metrics cards (mean variance, counts)
│   │   ├── AccuracyBucketBar.tsx    # stacked horizontal bar (10/20/30%)
│   │   ├── AccuracyTrendLine.tsx    # line chart over time
│   │   ├── ReferenceClassTable.tsx  # sortable breakdown table
│   │   ├── ThresholdProgress.tsx    # progress bar + "X more outcomes" message
│   │   └── LoadingSkeleton.tsx      # skeleton states
│   ├── hooks/
│   │   └── useCalibration.ts   # React Query hook wrapping calibration API
│   ├── pages/
│   │   ├── Dashboard.tsx       # main dashboard page
│   │   └── Login.tsx           # contractor login page
│   ├── router.tsx              # createBrowserRouter with auth guard
│   ├── App.tsx
│   └── main.tsx
├── index.html
├── vite.config.ts
├── tsconfig.json
└── package.json
```

**Backend addition:**
```
apps/efofx-estimate/app/
├── services/
│   └── calibration_service.py   # new — tenant-scoped aggregation + metrics
├── api/
│   └── calibration.py           # new — GET /api/v1/calibration/metrics endpoint
└── db/
    └── mongodb.py               # modified — add migrate_synthetic_reference_classes()
```

### Pattern 1: Idempotent Data Migration (CALB-01)

**What:** Add `data_source: "synthetic"` to all reference class documents where `is_synthetic: True`. Safe to run on every deploy.

**When to use:** Any backfill migration where the target state is a field that may or may not exist.

**Example:**
```python
# Source: existing pattern from mongodb.py (migrate_estimation_session_tenant_id)
async def migrate_synthetic_reference_classes():
    """
    CALB-01: Tag all synthetic reference class documents with data_source: "synthetic".

    Idempotent: uses $exists: false guard so re-runs are no-ops.
    """
    db = get_database()
    result = await db["reference_classes"].update_many(
        {
            "is_synthetic": True,
            "data_source": {"$exists": False},  # idempotency guard
        },
        {"$set": {"data_source": "synthetic"}},
    )
    logger.info(
        "CALB-01 migration: tagged %d synthetic reference classes with data_source='synthetic'",
        result.modified_count,
    )
```

**Registration in `mongodb.py` lifespan:**
```python
# In main.py lifespan, after create_indexes():
await migrate_synthetic_reference_classes()  # CALB-01
```

### Pattern 2: Tenant-Scoped $lookup Aggregation (CALB-04)

**What:** MongoDB `$lookup` with `let`/`pipeline` syntax to filter the joined collection by `tenant_id` inside the inner pipeline. This is **required** because `TenantAwareCollection.aggregate()` only scopes the source collection.

**When to use:** Any `$lookup` in a multi-tenant system.

**Example:**
```python
# Source: MongoDB Docs + project pattern established in STATE.md (CALB-04 note)
pipeline = [
    # TenantAwareCollection prepends: {"$match": {"tenant_id": tenant_id}}
    # This scopes the feedback collection (source). The $lookup below must
    # independently scope reference_classes (joined collection).
    {
        "$match": {
            "data_source": {"$ne": "synthetic"},  # CALB-01: exclude synthetic
        }
    },
    {
        "$lookup": {
            "from": "reference_classes",
            "let": {"rc_id": "$reference_class_id", "tenant": tenant_id},
            "pipeline": [
                {
                    "$match": {
                        "$expr": {
                            "$and": [
                                {"$eq": ["$name", "$$rc_id"]},
                                # CALB-04: explicit tenant_id filter in inner pipeline
                                {
                                    "$or": [
                                        {"$eq": ["$tenant_id", "$$tenant"]},
                                        {"$eq": ["$tenant_id", None]},  # platform data
                                    ]
                                },
                            ]
                        }
                    }
                }
            ],
            "as": "reference_class_doc",
        }
    },
    # ... group, project stages
]
```

### Pattern 3: CalibrationService Response Shape

**What:** A single endpoint returns either a below-threshold state or full metrics. The frontend branches on `below_threshold`.

**Example:**
```python
# app/services/calibration_service.py
from pydantic import BaseModel
from typing import Optional

class AccuracyBuckets(BaseModel):
    within_10_pct: float   # proportion (0.0–1.0)
    within_20_pct: float
    within_30_pct: float
    beyond_30_pct: float

class ReferenceClassBreakdown(BaseModel):
    reference_class: str
    outcome_count: int
    mean_variance_pct: float
    accuracy_buckets: AccuracyBuckets
    limited_data: bool  # True when outcome_count < 5

class CalibrationMetrics(BaseModel):
    below_threshold: bool
    outcome_count: int
    threshold: int = 10
    mean_variance_pct: Optional[float] = None     # None when below_threshold
    accuracy_buckets: Optional[AccuracyBuckets] = None
    by_reference_class: list[ReferenceClassBreakdown] = []
    # date range filtering
    date_range: Optional[str] = None  # "6months" | "1year" | "all"
```

### Pattern 4: React Query + JWT Auth Pattern (Plan 08-03)

**What:** Axios instance with interceptor injects JWT from localStorage into every request. React Query wraps data fetching. React Router v7 auth guard redirects unauthenticated users.

**Example:**
```typescript
// src/api/client.ts
import axios from 'axios';

export const apiClient = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8000',
});

apiClient.interceptors.request.use((config) => {
  const token = localStorage.getItem('access_token');
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});
```

```typescript
// src/router.tsx — auth guard pattern (React Router v7)
import { createBrowserRouter, redirect } from 'react-router';

function requireAuth() {
  const token = localStorage.getItem('access_token');
  if (!token) throw redirect('/login');
  return null;
}

export const router = createBrowserRouter([
  { path: '/login', element: <LoginPage /> },
  {
    path: '/',
    loader: requireAuth,
    element: <Dashboard />,
  },
]);
```

```typescript
// src/hooks/useCalibration.ts
import { useQuery } from '@tanstack/react-query';
import { apiClient } from '../api/client';

export function useCalibration(dateRange: string) {
  return useQuery({
    queryKey: ['calibration', dateRange],
    queryFn: async () => {
      const { data } = await apiClient.get('/api/v1/calibration/metrics', {
        params: { date_range: dateRange },
      });
      return data;
    },
  });
}
```

### Pattern 5: Recharts 3.x Stacked Horizontal Bar

**What:** Accuracy bucket bar using `BarChart` with `layout="vertical"` and multiple `Bar` components sharing `stackId`.

**Key breaking change from Recharts 2 to 3:** `CategoricalChartState` removed. Custom tooltips now use `TooltipContentProps`. `CartesianGrid` requires explicit `xAxisId`/`yAxisId` props.

**Example:**
```tsx
// AccuracyBucketBar.tsx — stacked horizontal bar (Recharts 3.x)
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer } from 'recharts';

const data = [{
  name: 'Accuracy',
  within_10: 0.35,
  within_20: 0.25,
  within_30: 0.20,
  beyond_30: 0.20,
}];

export function AccuracyBucketBar({ buckets }: { buckets: AccuracyBuckets }) {
  return (
    <ResponsiveContainer width="100%" height={60}>
      <BarChart layout="vertical" data={data}>
        <XAxis type="number" domain={[0, 1]} hide />
        <YAxis type="category" dataKey="name" hide />
        <Tooltip />
        <Bar dataKey="within_10" stackId="a" fill="#22c55e" name="Within 10%" />
        <Bar dataKey="within_20" stackId="a" fill="#eab308" name="Within 20%" />
        <Bar dataKey="within_30" stackId="a" fill="#f97316" name="Within 30%" />
        <Bar dataKey="beyond_30" stackId="a" fill="#ef4444" name="Beyond 30%" />
      </BarChart>
    </ResponsiveContainer>
  );
}
```

**Note on accuracy bucket semantics:** The CONTEXT.md specifies buckets are cumulative ("within 10% is a subset of within 20%"). The display bar should show *exclusive* slices for visual proportion: `[0–10%]`, `(10–20%]`, `(20–30%]`, `>30%`. The CalibrationService must compute exclusive slices for the chart data, not cumulative totals.

### Anti-Patterns to Avoid

- **$lookup without inner tenant_id filter:** The biggest security pitfall. `TenantAwareCollection.aggregate()` does NOT protect joined collections. Always use `let`/`pipeline` with `$match`+`$expr` inside `$lookup`.
- **Running migration on tenant-scoped collection:** The `reference_classes` synthetic documents have `tenant_id: None` (platform data). Run CALB-01 migration against the **raw** collection (`get_collection("reference_classes")`), not via `get_tenant_collection`.
- **Recharts 2.x patterns in 3.x:** `activeIndex` prop removed from `Bar`, `CategoricalChartState` removed, `CartesianGrid` needs explicit axis IDs. Use 3.x documentation.
- **Storing JWT in httpOnly cookie for this SPA:** The existing API uses Bearer token auth (`Authorization: Bearer <token>`). Store token in `localStorage` for simplicity — this is an internal contractor tool, not a public-facing app.
- **React Query v4 patterns in v5:** In v5, `isLoading` is renamed to `isPending` for mutations. `status === "loading"` is now `status === "pending"`. Always check v5 docs.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Stacked bar chart | Custom SVG/canvas bars | Recharts `BarChart` with `stackId` | Recharts handles responsive sizing, tooltips, accessibility |
| Line chart over time | Custom D3 | Recharts `LineChart` | Same — complex axis math, animation |
| Data fetching state machine | `useState` + `useEffect` + manual error handling | `@tanstack/react-query` `useQuery` | Cache invalidation, background refetch, loading/error states |
| JWT interceptor | Per-fetch auth header | Axios interceptors | DRY; handles token injection before every request without per-call boilerplate |
| Sortable table | Custom sort state + CSS | Native `Array.sort` + `useState` for sort key/direction | Simple enough for a 3-column table — no third-party table library needed |

**Key insight:** The reference class breakdown table has at most ~20 rows (7 construction types × few regions). A native `Array.sort` with a `useState` sort key is sufficient — adding a heavy table library (react-table, AG Grid) would be over-engineering.

---

## Common Pitfalls

### Pitfall 1: Cross-Tenant $lookup (CALB-04 Security)

**What goes wrong:** CalibrationService queries feedback for Tenant A, then `$lookup` joins `reference_classes` without a tenant filter — it returns reference classes from Tenant B.

**Why it happens:** Developers assume `TenantAwareCollection.aggregate()` protects all collections in the pipeline. It only prepends `$match` to the **source** collection.

**How to avoid:** Always use `$lookup` with `let`/`pipeline`/`$match`+`$expr` pattern. Never use the simple `$lookup: { from, localField, foreignField, as }` shorthand in a multi-tenant aggregation.

**Warning signs:** Any `$lookup` without a `pipeline:` key in a CalibrationService method.

### Pitfall 2: Synthetic Data in Calibration Query (CALB-01)

**What goes wrong:** Calibration calculates variance using synthetic reference data instead of real contractor outcomes, producing meaningless accuracy metrics.

**Why it happens:** Migration runs after the aggregation was written; `data_source` field missing on older documents so the filter `{"data_source": {"$ne": "synthetic"}}` passes them through.

**How to avoid:** Run CALB-01 migration *before* CalibrationService goes live. Add `data_source` to the migration as a startup step. Add an assertion in the CalibrationService that verifies no `data_source: "synthetic"` documents appear in results during development.

**Warning signs:** Calibration accuracy showing perfect 0% variance — synthetic data was generated from the same distributions used for estimation.

### Pitfall 3: Recharts 3.x Breaking Changes from 2.x

**What goes wrong:** Code copied from tutorials or AI using Recharts 2.x patterns breaks silently in 3.x (e.g., accessing `activeIndex` prop on `Bar`, expecting `CategoricalChartState` in `<Customized />`).

**Why it happens:** Most tutorials and AI training data pre-date the Recharts 3.0 migration (released mid-2024).

**How to avoid:** Reference the [Recharts 3.0 migration guide](https://github.com/recharts/recharts/wiki/3.0-migration-guide). Key changes: use `useActiveTooltipLabel` hook instead of `CategoricalChartState`; custom tooltips use `TooltipContentProps` (not `TooltipProps`); `CartesianGrid` needs explicit `xAxisId`/`yAxisId`.

**Warning signs:** TypeScript errors mentioning `CategoricalChartState`, `activeIndex` prop not accepted on `Bar`.

### Pitfall 4: Motor aggregate() Returns Cursor (Not Awaitable Directly)

**What goes wrong:** `await col.aggregate(pipeline)` returns a cursor, not a list. Developers call `await` on it expecting results.

**Why it happens:** Motor's `aggregate()` is a synchronous call returning `AsyncIOMotorCommandCursor` — it's the `.to_list()` call that's async.

**How to avoid:** Always chain `.to_list(None)`:
```python
# Correct Motor async aggregation pattern
cursor = await feedback_col.aggregate(pipeline)
results = await cursor.to_list(None)
```

**Warning signs:** `TypeError: object AsyncIOMotorCommandCursor can't be used in 'await' expression`

Note: `TenantAwareCollection.aggregate()` is already `async def` and returns the cursor directly — use:
```python
cursor = await tenant_col.aggregate(pipeline)
results = await cursor.to_list(None)
```

### Pitfall 5: React Router v7 Package Consolidation

**What goes wrong:** Developer installs `react-router-dom` (v7 still exists but recommends dropping it), gets confused about which package provides which exports.

**Why it happens:** In React Router v7, `react-router-dom` was consolidated into `react-router`. Both packages work, but the official recommendation is to import from `react-router` only.

**How to avoid:** Install `react-router` (not `react-router-dom`) and import everything from `react-router`. The `BrowserRouter`, `createBrowserRouter`, `RouterProvider`, `useNavigate`, `redirect` — all from `react-router`.

**Warning signs:** Duplicate exports error, or importing from both packages.

---

## Code Examples

Verified patterns from official sources and project codebase:

### CalibrationService Skeleton (Backend)
```python
# app/services/calibration_service.py
import logging
from typing import Optional
from datetime import datetime, timedelta, timezone

from app.db.mongodb import get_collection, get_tenant_collection
from app.core.constants import DB_COLLECTIONS

logger = logging.getLogger(__name__)

CALIBRATION_THRESHOLD = 10  # CALB-03: minimum real outcomes

class CalibrationService:
    async def get_metrics(
        self,
        tenant_id: str,
        date_range: Optional[str] = None,
    ) -> dict:
        """
        Compute calibration metrics for a tenant.

        Returns below-threshold state if fewer than CALIBRATION_THRESHOLD
        real outcomes exist. Never includes data_source='synthetic' docs.
        """
        feedback_col = get_tenant_collection(
            DB_COLLECTIONS["FEEDBACK"], tenant_id
        )

        # Build date filter
        date_filter = {}
        if date_range == "6months":
            date_filter["submitted_at"] = {
                "$gte": datetime.now(timezone.utc) - timedelta(days=182)
            }
        elif date_range == "1year":
            date_filter["submitted_at"] = {
                "$gte": datetime.now(timezone.utc) - timedelta(days=365)
            }

        # Count real outcomes first (CALB-03 threshold check)
        real_count = await feedback_col.count_documents(date_filter)

        if real_count < CALIBRATION_THRESHOLD:
            return {
                "below_threshold": True,
                "outcome_count": real_count,
                "threshold": CALIBRATION_THRESHOLD,
            }

        # Full aggregation pipeline
        pipeline = [
            {"$match": date_filter},
            {
                "$lookup": {
                    "from": "reference_classes",
                    "let": {"rc_id": "$reference_class_id", "tenant": tenant_id},
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$and": [
                                        {"$eq": ["$name", "$$rc_id"]},
                                        # CALB-04: explicit tenant filter in inner pipeline
                                        {
                                            "$or": [
                                                {"$eq": ["$tenant_id", "$$tenant"]},
                                                {"$eq": ["$tenant_id", None]},
                                            ]
                                        },
                                        # CALB-01: exclude synthetic
                                        {"$ne": ["$data_source", "synthetic"]},
                                    ]
                                }
                            }
                        }
                    ],
                    "as": "reference_class_doc",
                }
            },
            # ... $group, $project for variance and accuracy buckets
        ]

        cursor = await feedback_col.aggregate(pipeline)
        results = await cursor.to_list(None)

        return _compute_metrics(results, real_count)
```

### Accuracy Variance Calculation

Mean variance is: `mean(abs(actual_cost - estimated_cost_p50) / actual_cost * 100)`

Per-bucket calculation (exclusive slices for stacked bar display):
```python
def _compute_accuracy_buckets(variances: list[float]) -> dict:
    """Compute exclusive accuracy bucket proportions for stacked bar display."""
    total = len(variances)
    if total == 0:
        return {"within_10_pct": 0, "within_20_pct": 0, "within_30_pct": 0, "beyond_30_pct": 0}

    # Exclusive slices (not cumulative) for stacked bar proportions
    within_10 = sum(1 for v in variances if v <= 10) / total
    between_10_20 = sum(1 for v in variances if 10 < v <= 20) / total
    between_20_30 = sum(1 for v in variances if 20 < v <= 30) / total
    beyond_30 = sum(1 for v in variances if v > 30) / total

    return {
        "within_10_pct": round(within_10, 3),
        "within_20_pct": round(between_10_20, 3),
        "within_30_pct": round(between_20_30, 3),
        "beyond_30_pct": round(beyond_30, 3),
    }
```

### FastAPI Calibration Endpoint
```python
# app/api/calibration.py
from fastapi import APIRouter, Depends, Query
from app.core.security import get_current_tenant
from app.models.tenant import Tenant
from app.services.calibration_service import CalibrationService

calibration_router = APIRouter(prefix="/calibration", tags=["calibration"])

@calibration_router.get("/metrics")
async def get_calibration_metrics(
    tenant: Tenant = Depends(get_current_tenant),
    date_range: str = Query(default="all", pattern="^(6months|1year|all)$"),
) -> dict:
    """Get calibration accuracy metrics for the authenticated contractor."""
    svc = CalibrationService()
    return await svc.get_metrics(tenant.tenant_id, date_range)
```

### Vite Config for Dashboard (Plan 08-03)
```typescript
// apps/efofx-dashboard/vite.config.ts
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5174,  // different from efofx-widget (5173)
    proxy: {
      '/api': {
        target: 'http://localhost:8000',
        changeOrigin: true,
      },
    },
  },
})
```

### ThresholdProgress Component
```tsx
// src/components/ThresholdProgress.tsx
interface ThresholdProgressProps {
  outcomeCount: number;
  threshold: number;
}

export function ThresholdProgress({ outcomeCount, threshold }: ThresholdProgressProps) {
  const remaining = threshold - outcomeCount;
  const pct = Math.min((outcomeCount / threshold) * 100, 100);

  return (
    <div className="threshold-progress">
      <p className="threshold-count">
        {outcomeCount} of {threshold} outcomes recorded
      </p>
      <div className="progress-bar-track">
        <div className="progress-bar-fill" style={{ width: `${pct}%` }} />
      </div>
      <p className="threshold-explanation">
        We need at least {threshold} completed projects to calculate meaningful
        accuracy metrics. {remaining} more to go.
      </p>
    </div>
  );
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `react-router-dom` as separate package | Import all from `react-router` only | React Router v7 (2024) | Eliminates package confusion; `react-router-dom` still works but is legacy |
| Recharts 2.x `CategoricalChartState` | Hooks: `useActiveTooltipLabel` etc. | Recharts 3.0 (2024) | Breaking change — custom chart components need update |
| React Query v4 `isLoading` | v5 `isPending` for mutations | TanStack Query v5 (Oct 2023) | Minor rename but causes subtle bugs if mixing docs |
| Motor (async MongoDB driver) | PyMongo Async API | Motor EOL May 2026 | Not urgent for this phase; migration is Phase 9+ concern |

**Deprecated/outdated:**
- `react-router-dom`: Still works in v7 but all APIs consolidated into `react-router`. New apps should use `react-router`.
- Recharts 2.x `TooltipProps` for custom tooltips: Replaced by `TooltipContentProps` in 3.x.
- Motor: Will be EOL May 2026 / critical bug fixes until May 2027. This phase uses Motor 3.3.2 as already pinned — no action required now.

---

## Open Questions

1. **CORS: Will the dashboard app's origin be allowed by TenantAwareCORSMiddleware?**
   - What we know: `TenantAwareCORSMiddleware` checks static `ALLOWED_ORIGINS` first, then tenant-cached origins. Development uses `localhost:5174`.
   - What's unclear: Whether `settings.ALLOWED_ORIGINS` includes `http://localhost:5174` in dev, or whether `*` is set.
   - Recommendation: Plan 08-03 should add `http://localhost:5174` to `.env` `ALLOWED_ORIGINS` for dev. Production URL needs to be added to `ALLOWED_ORIGINS` env var.

2. **FeedbackDocument.reference_class_id field: is it always populated?**
   - What we know: `FeedbackDocument` has `reference_class_id: Optional[str] = None`. Phase 7 stores it from `ESTIMATE_CTX["reference_class_id"]`.
   - What's unclear: Whether all feedback documents from Phase 7 have this field populated, or only some.
   - Recommendation: CalibrationService must handle `None` reference_class_id gracefully (group as "Unknown" or skip). The `$lookup` join will return an empty array if `reference_class_id` is None.

3. **Accuracy buckets: cumulative vs exclusive in the API response**
   - What we know: CONTEXT.md says "buckets are cumulative: within 10% is a subset of within 20%." The stacked bar visualization needs exclusive slices.
   - What's unclear: Should the API return cumulative percentages (for tooltips) or exclusive slices (for bar segments), or both?
   - Recommendation: Return **exclusive slices** in the API (better for the stacked bar). Document that "within_10_pct" means "exactly within 10%", "within_20_pct" means "between 10% and 20%", etc. Frontend can compute cumulative if needed for tooltips.

---

## Validation Architecture

> `workflow.nyquist_validation` is **not set** (config.json has no `nyquist_validation` key — defaults to false). Skip automated test mapping.

Tests should follow the existing project pattern: `pytest` with `asyncio_mode = "auto"`, mocks via `unittest.mock.AsyncMock`, test files in `tests/services/test_calibration_service.py` and `tests/api/test_calibration.py`.

Key tests to create in Wave 0 of Plan 08-02:

| Area | Test | Type |
|------|------|------|
| CALB-01 | Migration is idempotent (second run modifies 0 docs) | unit |
| CALB-02 | `get_metrics` returns correct mean variance | unit |
| CALB-03 | `get_metrics` returns `below_threshold: True` when count < 10 | unit |
| CALB-04 | Aggregation pipeline includes tenant_id in $lookup inner pipeline | unit (assert on pipeline structure) |
| CALB-06 | API returns `below_threshold` flag in response schema | unit |

---

## Sources

### Primary (HIGH confidence)
- Project codebase: `apps/efofx-estimate/app/db/tenant_collection.py` — confirmed `aggregate()` only scopes source collection
- Project codebase: `apps/efofx-estimate/app/db/mongodb.py` — migration pattern for `migrate_estimation_session_tenant_id` (CALB-01 model)
- Project codebase: `apps/synthetic-data-generator/generators/common.py` — confirms synthetic docs have `is_synthetic: True`, `tenant_id: None`
- Project codebase: `apps/efofx-estimate/app/models/feedback.py` — confirms `FeedbackDocument.reference_class_id: Optional[str]`
- MongoDB Docs: `$lookup` with `let`/`pipeline` syntax — official aggregation documentation
- Recharts GitHub wiki: [3.0 migration guide](https://github.com/recharts/recharts/wiki/3.0-migration-guide) — confirmed breaking changes

### Secondary (MEDIUM confidence)
- npm search result: Recharts 3.7.0 is the latest stable version as of March 2026 — React 19 compatible
- npm search result: `@tanstack/react-query` 5.90.21 is current as of March 2026
- npm search result: `react-router` 7.13.1 is current as of March 2026
- WebSearch: React Router v7 consolidates `react-router-dom` into `react-router`
- WebSearch: Motor EOL confirmed May 2026 (bug fixes until May 2027)

### Tertiary (LOW confidence)
- WebSearch: Vite 6.x is the current major version — not independently verified via official docs

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all backend libraries are already pinned in the project; frontend versions confirmed via npm search results
- Architecture: HIGH — backend patterns derived directly from existing codebase; frontend patterns match efofx-widget
- Pitfalls: HIGH — CALB-04 pitfall confirmed by reading `TenantAwareCollection.aggregate()` implementation; Recharts 3.x pitfall confirmed from official migration guide

**Research date:** 2026-03-05
**Valid until:** 2026-04-05 (frontend library versions change frequently; backend stack is stable)
