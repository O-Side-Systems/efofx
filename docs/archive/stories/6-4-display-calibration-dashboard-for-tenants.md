# Story 6.4: Display Calibration Dashboard for Tenants

Status: backlog

## Story

As a contractor (tenant),
I want to see how accurate my estimates are over time,
So that I can build trust with customers and improve my quoting.

## Acceptance Criteria

**Given** calibration metrics are calculated
**When** I implement the dashboard UI
**Then** tenant portal includes "Calibration Metrics" page with:
- Summary Cards (feedback rate, accuracy, variance, trust score)
- Charts (accuracy over time, variance by reference class, distribution histogram)
- Insights (actionable recommendations)
- Public Trust Badge (embeddable, updates monthly)

**And** calibration data exportable as CSV for tenant analysis

## Tasks / Subtasks

- [ ] Create Calibration Metrics page in tenant portal
- [ ] Create summary cards (feedback rate, accuracy, variance, trust score)
- [ ] Create line chart (accuracy over time)
- [ ] Create bar chart (variance by reference class)
- [ ] Create distribution histogram
- [ ] Generate insights/recommendations
- [ ] Create embeddable trust badge
- [ ] Implement CSV export
- [ ] Test dashboard UI

## Dev Notes

### Prerequisites

Story 6.3 (metrics calculated)

### Technical Notes

- Use Chart.js or Recharts for visualizations
- Trust Score algorithm: `(within_20_pct * 0.6 + feedback_rate * 0.3 + trend * 0.1) * 100`
- Public badge requires tenant opt-in (privacy)
- Benchmark against industry standards (if available)
- Highlight improving trends to encourage feedback collection

### References

- [Source: docs/epics.md#Story-6-4]
- [Source: docs/PRD.md] (for requirements context)

## Dev Agent Record

### Context Reference

<!-- Path(s) to story context XML will be added here by context workflow -->

### Agent Model Used

<!-- To be filled by dev agent -->

### Debug Log References

<!-- To be filled by dev agent during implementation -->

### Completion Notes List

<!-- To be filled by dev agent upon completion -->

### File List

<!-- NEW/MODIFIED/DELETED files will be listed here by dev agent -->
