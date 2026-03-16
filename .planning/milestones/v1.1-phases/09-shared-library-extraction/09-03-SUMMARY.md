---
phase: 09-shared-library-extraction
plan: 03
subsystem: ui
tags: [react, css-modules, vite, typescript, npm-workspaces, shared-components]

# Dependency graph
requires:
  - phase: 09-01
    provides: "@efofx/ui package skeleton with workspace configs and npm install working"
provides:
  - "5 shared React components extracted to packages/efofx-ui with co-located CSS modules"
  - "EstimationOutput type defined locally in packages/efofx-ui/src/types/estimation.ts"
  - "Barrel export in packages/efofx-ui/src/index.ts exposing all components and prop types"
  - "Both consuming apps import shared components from @efofx/ui via workspace linking"
  - "Both apps pass tsc -b and vite build"
affects: [09-04, future-verticals]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CSS modules with camelCase class names for all shared components"
    - "CSS custom property hooks (var(--brand-primary), var(--brand-secondary), var(--brand-accent)) preserved for theming"
    - "Source imports from @efofx/ui (no build step in shared package; consuming app Vite resolves TypeScript directly)"
    - "npm workspace dependency uses bare '*' syntax (not 'workspace:*' which is pnpm/yarn)"

key-files:
  created:
    - packages/efofx-ui/src/types/estimation.ts
    - packages/efofx-ui/src/components/ChatBubble/ChatBubble.tsx
    - packages/efofx-ui/src/components/ChatBubble/ChatBubble.module.css
    - packages/efofx-ui/src/components/EstimateCard/EstimateCard.tsx
    - packages/efofx-ui/src/components/EstimateCard/EstimateCard.module.css
    - packages/efofx-ui/src/components/TypingIndicator/TypingIndicator.tsx
    - packages/efofx-ui/src/components/TypingIndicator/TypingIndicator.module.css
    - packages/efofx-ui/src/components/ErrorBoundary/ErrorBoundary.tsx
    - packages/efofx-ui/src/components/LoadingSkeleton/LoadingSkeleton.tsx
    - packages/efofx-ui/src/components/LoadingSkeleton/LoadingSkeleton.module.css
  modified:
    - packages/efofx-ui/src/index.ts
    - packages/efofx-ui/package.json
    - apps/efofx-widget/package.json
    - apps/efofx-widget/src/components/ChatPanel.tsx
    - apps/efofx-widget/src/main.tsx
    - apps/efofx-dashboard/package.json
    - apps/efofx-dashboard/src/pages/Dashboard.tsx

key-decisions:
  - "npm workspace dependency uses bare '*' syntax — 'workspace:*' is pnpm/yarn; npm workspaces resolve by package name match in workspaces array"
  - "react-error-boundary v6 changed onError param type from Error to unknown — typed as unknown in shared ErrorBoundary"
  - "ChatMessage interface defined locally in ChatBubble.tsx (not in types/estimation.ts) — it's a chat/messaging concept, not estimation domain"
  - "WidgetErrorBoundary renamed to ErrorBoundary in @efofx/ui — generic name appropriate for shared package"
  - "LoadingSkeleton CSS module uses var(--color-border) fallback so it adapts to host theme without hardcoded colors"

patterns-established:
  - "CSS module pattern: all class names camelCase, no hyphenated identifiers"
  - "CSS custom property fallbacks: var(--brand-primary, #2563eb) so components work without host providing custom properties"
  - "Component co-location: {Name}/{Name}.tsx + {Name}/{Name}.module.css in same directory"

requirements-completed: [EXTR-03]

# Metrics
duration: 12min
completed: 2026-03-15
---

# Phase 09 Plan 03: Shared UI Components Summary

**5 React components (ChatBubble, EstimateCard, TypingIndicator, ErrorBoundary, LoadingSkeleton) extracted to @efofx/ui with CSS modules using camelCase class names, both apps importing via npm workspace linking and building cleanly**

## Performance

- **Duration:** 12 min
- **Started:** 2026-03-15T01:45:49Z
- **Completed:** 2026-03-15T01:57:00Z
- **Tasks:** 2
- **Files modified:** 17 (10 created, 5 deleted, 7 modified)

## Accomplishments
- All 5 components extracted with co-located CSS modules — global class names converted to camelCase
- EstimationOutput, CostCategoryEstimate, AdjustmentFactor types defined locally in packages/efofx-ui (no cross-package type dependency)
- Both apps (efofx-widget, efofx-dashboard) updated to import from @efofx/ui and build without TypeScript errors
- Original component files deleted from apps — immediate re-import, no deprecation period
- Barrel export exposes all components plus prop types from packages/efofx-ui/src/index.ts

## Task Commits

Each task was committed atomically:

1. **Task 1: Extract components to @efofx/ui with CSS modules** - `3349734` (feat)
2. **Task 2: Update consuming apps to import from @efofx/ui and verify builds** - `f8ebbce` (feat)

**Plan metadata:** (docs commit — see below)

## Files Created/Modified

**Created in packages/efofx-ui:**
- `src/types/estimation.ts` - Standalone EstimationOutput + CostCategoryEstimate + AdjustmentFactor interfaces
- `src/components/ChatBubble/ChatBubble.tsx` - ChatBubble component with ChatMessage type, CSS module imports
- `src/components/ChatBubble/ChatBubble.module.css` - bubbleWrapper/bubbleUser/bubbleAssistant camelCase classes
- `src/components/EstimateCard/EstimateCard.tsx` - P50/P80 range bar + accordion, imports from local types
- `src/components/EstimateCard/EstimateCard.module.css` - estimateCard/rangeBar/accordion camelCase classes
- `src/components/TypingIndicator/TypingIndicator.tsx` - Three-dot bounce animation
- `src/components/TypingIndicator/TypingIndicator.module.css` - typingIndicator/typingDot with @keyframes bounce
- `src/components/ErrorBoundary/ErrorBoundary.tsx` - Wraps react-error-boundary, silent fallback
- `src/components/LoadingSkeleton/LoadingSkeleton.tsx` - Calibration dashboard pulsing skeleton
- `src/components/LoadingSkeleton/LoadingSkeleton.module.css` - loadingSkeleton/statsGrid/skeleton classes

**Modified:**
- `packages/efofx-ui/src/index.ts` - Barrel exports all 5 components and prop types
- `packages/efofx-ui/package.json` - Added react-error-boundary as peer dependency
- `apps/efofx-widget/package.json` - Added "@efofx/ui": "*" workspace dependency
- `apps/efofx-widget/src/components/ChatPanel.tsx` - Imports ChatBubble, TypingIndicator, EstimateCard from @efofx/ui
- `apps/efofx-widget/src/main.tsx` - Imports ErrorBoundary from @efofx/ui (replaces WidgetErrorBoundary)
- `apps/efofx-dashboard/package.json` - Added "@efofx/ui": "*" workspace dependency
- `apps/efofx-dashboard/src/pages/Dashboard.tsx` - Imports LoadingSkeleton from @efofx/ui

**Deleted:**
- `apps/efofx-widget/src/components/ChatBubble.tsx`
- `apps/efofx-widget/src/components/EstimateCard.tsx`
- `apps/efofx-widget/src/components/TypingIndicator.tsx`
- `apps/efofx-widget/src/components/ErrorBoundary.tsx`
- `apps/efofx-dashboard/src/components/LoadingSkeleton.tsx`

## Decisions Made

- **npm workspace syntax**: Uses `"*"` not `"workspace:*"` — npm workspaces don't support the `workspace:` protocol (that's pnpm/yarn)
- **react-error-boundary v6 type change**: `onError` param changed from `Error` to `unknown` in v6 — typed accordingly to avoid TypeScript error
- **ChatMessage co-location**: Defined in ChatBubble.tsx directly, not in types/estimation.ts, since it's a chat concept not an estimation domain type
- **WidgetErrorBoundary → ErrorBoundary**: Renamed to generic `ErrorBoundary` in shared package — widget-specific naming not appropriate in shared lib

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed react-error-boundary v6 type incompatibility**
- **Found during:** Task 2 (verify builds)
- **Issue:** ErrorBoundary.tsx typed `onError` callback with `(error: Error, info: ErrorInfo)` but react-error-boundary v6 changed the type to `(error: unknown, info: ErrorInfo)` — TypeScript build failed with TS2322
- **Fix:** Changed parameter type from `Error` to `unknown`
- **Files modified:** `packages/efofx-ui/src/components/ErrorBoundary/ErrorBoundary.tsx`
- **Verification:** Both apps pass `tsc -b && vite build`
- **Committed in:** f8ebbce (Task 2 commit)

**2. [Rule 3 - Blocking] Fixed npm workspace:* protocol not supported**
- **Found during:** Task 2 (npm install step)
- **Issue:** Plan specified `"workspace:*"` dependency syntax but npm workspaces use bare `"*"` — `workspace:` protocol is pnpm/yarn only; npm install failed with EUNSUPPORTEDPROTOCOL
- **Fix:** Changed `"@efofx/ui": "workspace:*"` to `"@efofx/ui": "*"` in both app package.json files
- **Files modified:** `apps/efofx-widget/package.json`, `apps/efofx-dashboard/package.json`
- **Verification:** npm install succeeded, @efofx/ui symlink created in node_modules
- **Committed in:** f8ebbce (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 bug fix, 1 blocking issue)
**Impact on plan:** Both auto-fixes necessary for correctness. No scope creep.

## Issues Encountered

- The plan frontmatter specified `workspace:*` dependency syntax. npm workspaces require bare `*`. This is a pnpm/yarn protocol that npm does not support.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- @efofx/ui package fully operational with 5 components and CSS modules
- Both consuming apps building cleanly with workspace imports
- Ready for plan 09-04: Python shared library extraction (efofx-shared)
- CSS custom property theming hooks preserved throughout all components

---
*Phase: 09-shared-library-extraction*
*Completed: 2026-03-15*
