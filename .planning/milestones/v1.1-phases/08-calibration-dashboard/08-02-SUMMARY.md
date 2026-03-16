---
phase: 08-calibration-dashboard
plan: 02
subsystem: ui
tags: [react, vite, typescript, react-query, react-router, axios, recharts]

# Dependency graph
requires:
  - phase: 08-calibration-dashboard
    provides: CalibrationService API endpoints (/api/v1/calibration/metrics, /api/v1/calibration/trend)
  - phase: 02-auth
    provides: POST /api/v1/auth/login endpoint returning access_token + refresh_token
provides:
  - apps/efofx-dashboard/ — standalone Vite + React 19 SPA on port 5174
  - src/types/calibration.ts — TypeScript types matching CalibrationService response shapes
  - src/api/client.ts — axios instance with JWT Bearer token interceptor
  - src/api/calibration.ts — fetchCalibrationMetrics() and fetchCalibrationTrend() functions
  - src/hooks/useCalibration.ts — React Query v5 hook for metrics data
  - src/hooks/useCalibrationTrend.ts — React Query v5 hook for trend time-series data
  - src/router.tsx — requireAuth loader redirecting unauthenticated users to /login
  - src/pages/Login.tsx — functional login form storing access_token + refresh_token
  - src/pages/Dashboard.tsx — placeholder scaffold for Plan 08-03 UI components
affects: [08-calibration-dashboard/08-03]

# Tech tracking
tech-stack:
  added:
    - "@tanstack/react-query ^5.83.0 — server state management with 5min staleTime"
    - "react-router ^7.5.0 — client-side routing with loader-based auth guard"
    - "axios ^1.9.0 — HTTP client with request interceptor"
    - "recharts ^2.15.3 — chart library for Plan 08-03 accuracy visualization"
    - "date-fns ^4.1.0 — date formatting utilities"
  patterns:
    - "React Query v5: use isPending (not isLoading) for initial loading state"
    - "React Router v7: import from react-router (not react-router-dom — consolidated)"
    - "Auth guard via loader function: throw redirect('/login') when no token"
    - "JWT injection via axios request interceptor reading localStorage access_token"
    - "CSS custom properties (no Tailwind) for Stripe/Linear muted aesthetic"

key-files:
  created:
    - apps/efofx-dashboard/package.json
    - apps/efofx-dashboard/vite.config.ts
    - apps/efofx-dashboard/tsconfig.json
    - apps/efofx-dashboard/tsconfig.app.json
    - apps/efofx-dashboard/tsconfig.node.json
    - apps/efofx-dashboard/eslint.config.js
    - apps/efofx-dashboard/index.html
    - apps/efofx-dashboard/src/main.tsx
    - apps/efofx-dashboard/src/App.tsx
    - apps/efofx-dashboard/src/App.css
    - apps/efofx-dashboard/src/router.tsx
    - apps/efofx-dashboard/src/api/client.ts
    - apps/efofx-dashboard/src/api/calibration.ts
    - apps/efofx-dashboard/src/hooks/useCalibration.ts
    - apps/efofx-dashboard/src/hooks/useCalibrationTrend.ts
    - apps/efofx-dashboard/src/pages/Dashboard.tsx
    - apps/efofx-dashboard/src/pages/Login.tsx
    - apps/efofx-dashboard/src/types/calibration.ts
  modified: []

key-decisions:
  - "Port 5174 for efofx-dashboard to avoid conflict with efofx-widget on 5173"
  - "Vite proxy /api -> localhost:8000 avoids CORS in development; VITE_API_BASE_URL for production"
  - "React Query v5 staleTime: 5 minutes — calibration data changes infrequently"
  - "CSS custom properties (no Tailwind) — Stripe/Linear aesthetic with lots of whitespace"
  - "by_reference_class typed as optional in CalibrationMetrics to match below-threshold response"

patterns-established:
  - "Auth guard pattern: requireAuth() loader throws redirect('/login') — works with React Router v7 loader API"
  - "React Query queryKey convention: ['calibration', dateRange] and ['calibrationTrend', months]"
  - "API layer separation: api/client.ts (axios instance) + api/calibration.ts (domain functions) + hooks/ (React Query wrappers)"

requirements-completed: [CALB-05]

# Metrics
duration: 4min
completed: 2026-03-05
---

# Phase 8 Plan 02: Calibration Dashboard Summary

**Standalone Vite + React 19 SPA with JWT auth guard, axios interceptor, and React Query hooks for calibration metrics and trend data**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-06T05:59:05Z
- **Completed:** 2026-03-06T06:03:00Z
- **Tasks:** 2
- **Files modified:** 18 (all new — no changes to existing code)

## Accomplishments

- Scaffolded apps/efofx-dashboard/ with Vite 7 + React 19 + TypeScript, port 5174, Vite proxy for /api
- Implemented full auth flow: Login form POSTs to /api/v1/auth/login, stores tokens, router redirects unauthenticated users to /login
- Created typed React Query v5 hooks (useCalibration + useCalibrationTrend) matching CalibrationService API response shapes exactly

## Task Commits

Each task was committed atomically:

1. **Task 1: Scaffold Vite + React 19 app with dependencies** - `d3589b9` (feat)
2. **Task 2: Add TypeScript types, API client, auth routing, and React Query hooks** - `f356071` (feat)

**Plan metadata:** `fe576f9` (docs)

## Files Created/Modified

- `apps/efofx-dashboard/package.json` — React 19, recharts, @tanstack/react-query v5, react-router v7, axios, date-fns
- `apps/efofx-dashboard/vite.config.ts` — port 5174, /api proxy to :8000
- `apps/efofx-dashboard/tsconfig.{json,app.json,node.json}` — project references pattern matching efofx-widget
- `apps/efofx-dashboard/src/App.tsx` — QueryClientProvider + RouterProvider wrappers, staleTime 5min
- `apps/efofx-dashboard/src/App.css` — CSS custom properties: --color-bg, --color-surface, --color-text, --color-accent, accuracy bucket colors
- `apps/efofx-dashboard/src/types/calibration.ts` — AccuracyBuckets, ReferenceClassBreakdown, CalibrationMetrics, CalibrationTrendPoint, CalibrationTrendResponse
- `apps/efofx-dashboard/src/api/client.ts` — axios instance with Bearer token interceptor from localStorage.access_token
- `apps/efofx-dashboard/src/api/calibration.ts` — fetchCalibrationMetrics(dateRange) + fetchCalibrationTrend(months)
- `apps/efofx-dashboard/src/hooks/useCalibration.ts` — useQuery wrapping metrics fetch, queryKey ['calibration', dateRange]
- `apps/efofx-dashboard/src/hooks/useCalibrationTrend.ts` — useQuery wrapping trend fetch, queryKey ['calibrationTrend', months]
- `apps/efofx-dashboard/src/router.tsx` — createBrowserRouter with requireAuth loader (throws redirect('/login'))
- `apps/efofx-dashboard/src/pages/Login.tsx` — controlled form, POST to /api/v1/auth/login, navigate to / on success
- `apps/efofx-dashboard/src/pages/Dashboard.tsx` — placeholder using useCalibration(), isPending/isError/data states

## Decisions Made

- Port 5174 for dashboard to avoid conflict with efofx-widget on 5173
- React Query v5 staleTime: 5 minutes — calibration data changes infrequently, reduces unnecessary API calls
- No Tailwind — plain CSS custom properties for Stripe/Linear muted aesthetic per user preference
- `by_reference_class` typed as optional in `CalibrationMetrics` — the below-threshold response omits it

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None — no external service configuration required. Dev server starts with `npm run dev` in apps/efofx-dashboard/.

## Next Phase Readiness

- Dashboard scaffold complete and verified — npm run build passes, TypeScript clean
- useCalibration() and useCalibrationTrend() hooks are typed and ready for consumption
- Plan 08-03 can immediately build real chart components: replace Dashboard.tsx placeholder with accuracy bucket visualization and trend line chart

---
## Self-Check: PASSED

All files verified present. All commits verified in git log.

*Phase: 08-calibration-dashboard*
*Completed: 2026-03-05*
