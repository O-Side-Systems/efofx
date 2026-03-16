# Phase 8: Calibration Dashboard - Context

**Gathered:** 2026-03-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Contractors can see how accurate their estimates have been against real outcomes. Accuracy metrics are displayed only when statistically meaningful (10+ real outcomes), and synthetic data is never mixed into calibration calculations. Delivers: data migration to tag synthetic records, a CalibrationService for tenant-scoped aggregation, and a standalone dashboard app with charts and reference class breakdown.

</domain>

<decisions>
## Implementation Decisions

### Dashboard layout & structure
- Single scrollable page — summary stats at top, charts in middle, reference class breakdown at bottom
- Minimal and data-focused visual style — clean cards, muted colors, emphasis on numbers and charts (like Stripe/Linear)
- Standalone Vite + React app at its own URL (/dashboard), not a route in an existing app
- Optional date range picker to filter calibration data (last 6 months, last year, all-time)

### Accuracy visualization
- Headline display: single big mean variance number (e.g., ±12%) at the top
- Accuracy buckets (within 10/20/30% of actual) shown as a stacked horizontal bar — compact, easy to read at a glance
- Green-to-red gradient color scheme: within 10% = green, 20% = yellow, 30% = orange, >30% = red
- Line chart showing accuracy trend over time — helps contractors see if they're improving

### Threshold progress experience
- Contractors with fewer than 10 real outcomes see a progress bar + count: "4 of 10 outcomes recorded"
- Brief explanation included: "We need at least 10 completed projects to calculate meaningful accuracy metrics"
- No fanfare when unlocking — dashboard simply loads with metrics when threshold is met
- Dashboard link always visible in navigation — clicking it below threshold shows the progress state

### Reference class breakdown
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

</decisions>

<specifics>
## Specific Ideas

- Dashboard should feel like a Stripe or Linear dashboard — clean, not cluttered
- Accuracy buckets are cumulative: "within 10%" is a subset of "within 20%" which is a subset of "within 30%"
- The stacked bar for accuracy buckets should make it immediately obvious what proportion of estimates are accurate

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 08-calibration-dashboard*
*Context gathered: 2026-03-05*
