---
phase: 04-white-label-widget
plan: "01"
subsystem: ui
tags: [react, vite, shadow-dom, iife, widget, css-isolation, error-boundary, typescript]

# Dependency graph
requires: []
provides:
  - Shadow DOM isolated widget container with CSS injection via ?inline import
  - IIFE bundle (embed.js) loadable via single script tag with data attributes
  - WidgetConfig, WidgetPhase, BrandingConfig, LeadData, EstimationOutput, ChatMessage, ChatResponse TypeScript types
  - WidgetProvider/useWidget React context for shared phase, session, messages, branding state
  - WidgetErrorBoundary (react-error-boundary) with silent null fallback — widget errors never crash host page
  - FloatingButton: fixed-position 56px circle, bottom-right, chat bubble SVG icon, animated entrance
  - ChatPanel: slide-up panel (desktop) / full-screen takeover (mobile) with contractor branding header
  - Responsive layout via CSS media query at 480px, safe area insets for iOS notch/home indicator
  - App.tsx root component wiring FloatingButton + ChatPanel based on config.mode
  - Global try/catch in init() protects host page from widget initialization errors (WSEC-01)
affects:
  - 04-02 (branding API integration into widget)
  - 04-03 (chat UI fills efofx-messages and efofx-input-area containers)
  - 04-04 (lead capture form renders inside ChatPanel)

# Tech tracking
tech-stack:
  added:
    - react-error-boundary (silent null fallback error boundary)
    - dompurify (XSS sanitization, installed for future use)
    - "@types/dompurify"
  patterns:
    - Shadow DOM CSS injection via ?inline Vite import (not vite-plugin-css-injected-by-js which only injects to document.head)
    - document.currentScript captured synchronously at module top level (lost after async execution)
    - IIFE global try/catch as security gate for initialization errors
    - CSS custom properties on :host for branding overrides without breaking Shadow DOM isolation
    - pointer-events: auto on :host to re-enable clicks when host container has pointer-events: none

key-files:
  created:
    - apps/efofx-widget/src/types/widget.d.ts
    - apps/efofx-widget/src/context/WidgetContext.tsx
    - apps/efofx-widget/src/components/ErrorBoundary.tsx
    - apps/efofx-widget/src/components/FloatingButton.tsx
    - apps/efofx-widget/src/components/ChatPanel.tsx
    - apps/efofx-widget/src/widget.css
  modified:
    - apps/efofx-widget/src/components/ShadowDOMWrapper.tsx
    - apps/efofx-widget/src/App.tsx
    - apps/efofx-widget/src/main.tsx
    - apps/efofx-widget/vite.config.ts
    - apps/efofx-widget/package.json

key-decisions:
  - "?inline Vite import for widget.css is the only correct way to inject CSS into Shadow DOM — vite-plugin-css-injected-by-js injects to document.head which Shadow DOM does not inherit"
  - "document.currentScript captured at module top level before any async operations — it becomes null after synchronous execution completes"
  - "Removed vite-plugin-css-injected-by-js from production config; cssCodeSplit:false and exports:named added instead"
  - "Host div for floating mode uses pointer-events:none with overflow:visible and zero size; :host CSS resets pointer-events:auto for shadow content"
  - "widget.css uses all: initial on :host to fully reset host-page styles; explicit font-family re-applied after reset"

patterns-established:
  - "Shadow DOM CSS injection: import styles from '../widget.css?inline' then styleEl.textContent = widgetStyles injected before React container"
  - "Branding override: separate <style> element with :host { --brand-primary: ... } appended to shadow root when branding prop changes"
  - "Widget context pattern: WidgetProvider wraps entire React tree in shadow root, useWidget() throws if used outside provider"
  - "IIFE global error gate: entire init() wrapped in try/catch returns { destroy: () => {} } on failure"

requirements-completed: [WDGT-01, WDGT-02, WDGT-03, WDGT-04, WSEC-01]

# Metrics
duration: 4min
completed: "2026-02-27"
---

# Phase 4 Plan 01: Widget Shell Summary

**IIFE widget shell with Shadow DOM CSS isolation, responsive floating/inline layouts, branding-ready context, and silent error boundary — embeddable via 5-line script tag**

## Performance

- **Duration:** 4 min
- **Started:** 2026-02-27T18:13:38Z
- **Completed:** 2026-02-27T18:17:00Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments

- Widget renders entirely inside Shadow DOM with CSS injected via `?inline` Vite import — host page styles cannot leak in or out
- Vite build produces single `dist/embed.js` IIFE bundle (594 kB) callable as `efofxWidget.init()` with data attribute config
- FloatingButton (56px fixed circle, chat bubble SVG) + ChatPanel (slide-up desktop / full-screen mobile with iOS safe area insets) with zero efOfX branding
- TypeScript compilation passes with zero errors across all 10 created/modified files

## Task Commits

Each task was committed atomically:

1. **Task 1: Install dependencies, create types, context, and error boundary** - `09fd0c0` (feat)
2. **Task 2: Build widget shell — ShadowDOM CSS injection, main.tsx IIFE entry, FloatingButton, ChatPanel, responsive layouts** - `a06612c` (feat, committed together with 04-02 prep work that was pre-staged)

## Files Created/Modified

- `apps/efofx-widget/src/types/widget.d.ts` - WidgetConfig, WidgetPhase, BrandingConfig, LeadData, EstimationOutput, ChatMessage, ChatResponse TypeScript type contracts
- `apps/efofx-widget/src/context/WidgetContext.tsx` - WidgetProvider and useWidget hook with phase/sessionId/messages/branding state
- `apps/efofx-widget/src/components/ErrorBoundary.tsx` - WidgetErrorBoundary with react-error-boundary, silent null fallback, console.error logging
- `apps/efofx-widget/src/widget.css` - All widget styles as plain CSS with :host custom properties, floating button, slide-up/full-screen panel, safe area insets, no efOfX branding
- `apps/efofx-widget/src/components/ShadowDOMWrapper.tsx` - Injects widget.css string into shadow root via ?inline import, optional branding override via second <style> element
- `apps/efofx-widget/src/components/FloatingButton.tsx` - Fixed-position 56px circle with chat bubble SVG, renders only in floating+idle phase, animated entrance
- `apps/efofx-widget/src/components/ChatPanel.tsx` - Slide-up panel (desktop) / full-screen (mobile via CSS @media), contractor logo/company name header, placeholder message+input containers
- `apps/efofx-widget/src/App.tsx` - Root component: WidgetProvider wraps FloatingButton+ChatPanel (floating) or ChatPanel only (inline)
- `apps/efofx-widget/src/main.tsx` - IIFE entry: synchronous document.currentScript capture, data-api-key/data-mode/data-container reading, global try/catch in init(), auto-init in DEV
- `apps/efofx-widget/vite.config.ts` - Removed css-injected-by-js plugin, added cssCodeSplit:false and exports:named for clean IIFE output

## Decisions Made

- **Shadow DOM CSS injection via ?inline**: vite-plugin-css-injected-by-js injects into `document.head` which Shadow DOM does not inherit. The `?inline` Vite import returns CSS as a string for manual injection into the shadow root — this is the only correct approach.
- **Removed vite-plugin-css-injected-by-js**: Plugin removed entirely from production config since all styles go through Shadow DOM. Added `cssCodeSplit: false` and `exports: 'named'` to rollup output.
- **pointer-events pattern**: Host container div uses `pointer-events: none; overflow: visible; width: 0; height: 0` so it doesn't block page clicks; `:host` in CSS resets `pointer-events: auto` so shadow DOM buttons are clickable.
- **all: initial on :host**: Fully resets host page style inheritance; explicit `font-family` reapplied after reset to ensure consistent typography.

## Deviations from Plan

None - plan executed exactly as written. The `?inline` import pattern for CSS was already specified in the plan instructions. One minor addition (pointer-events re-enable on :host) was made as a correctness fix (Rule 1) to ensure the floating button remains clickable.

### Auto-fixed Issues

**1. [Rule 1 - Bug] Re-enable pointer-events on :host after host div sets pointer-events:none**
- **Found during:** Task 2 (FloatingButton + main.tsx)
- **Issue:** Host container div requires `pointer-events: none` to not block page interaction, but this inherited into shadow DOM, making the floating button unclickable
- **Fix:** Added `pointer-events: auto` to `:host` CSS block in widget.css
- **Files modified:** apps/efofx-widget/src/widget.css
- **Verification:** CSS rule present in built embed.js bundle
- **Committed in:** a06612c (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - Bug)
**Impact on plan:** Essential for button clickability. No scope creep.

## Issues Encountered

None - build and TypeScript compilation succeeded on first attempt.

## User Setup Required

None - no external service configuration required. Widget auto-initializes in DEV mode via `if (import.meta.env.DEV) { init(); }`.

## Next Phase Readiness

- Widget shell is ready for Plan 04-02 (branding API fetches BrandingConfig and passes to ShadowDOMWrapper/App)
- ChatPanel placeholder containers (efofx-messages, efofx-input-area) ready for Plan 04-03 chat UI
- All TypeScript types defined — future plans can import from `../types/widget`
- WidgetProvider context ready — future components use `useWidget()` to access phase/messages/branding

---
*Phase: 04-white-label-widget*
*Completed: 2026-02-27*
