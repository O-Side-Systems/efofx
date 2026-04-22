# Story 6.3: Calculate Calibration Metrics

Status: backlog

## Story

As a system,
I want to calculate accuracy metrics from feedback data,
So that tenants can see how well estimates match reality.

## Acceptance Criteria

**Given** FR-5.3 specifies calibration metrics
**When** I implement metrics calculation
**Then** `app/services/calibration_service.py` calculates:
- Per-Tenant Metrics (total estimates, feedback rate, cost/timeline variance %, within-20% percentage, within-P50-P80 percentage)
- Per-Reference-Class Metrics (count, feedback, variance, trend)
- Calculation triggers: real-time after feedback, nightly batch, on-demand

**And** `GET /api/v1/calibration/metrics` endpoint returns metrics with tenant_id, period, counts, cost/timeline metrics, by-reference-class breakdown

**And** metrics are cached (1 hour TTL) for performance

**And** historical trend data stored for graphing (last 12 months)

## Tasks / Subtasks

- [ ] Create `app/services/calibration_service.py`
- [ ] Implement per-tenant metrics calculation
- [ ] Implement per-reference-class metrics
- [ ] Create `GET /api/v1/calibration/metrics` endpoint
- [ ] Implement real-time calculation trigger
- [ ] Implement nightly batch job
- [ ] Add caching (1 hour TTL)
- [ ] Store historical trend data
- [ ] Test metrics accuracy
- [ ] Test performance

## Dev Notes

### Prerequisites

Stories 6.1-6.2 (feedback data exists)

### Technical Notes

- Use MongoDB aggregation pipeline for efficient calculation
- Handle edge cases: zero feedback, all estimates too high/low
- Calculate confidence intervals for small sample sizes (<10 feedback)
- Track "accuracy improving over time" as key metric
- Alert tenant if accuracy degrading (>5% worse month-over-month)

### References

- [Source: docs/epics.md#Story-6-3]
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
