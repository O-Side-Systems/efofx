# Story 6.2: Create Contractor Feedback Dashboard

Status: backlog

## Story

As a contractor (tenant),
I want to submit actual project outcomes and explain discrepancies,
So that the system learns from my real-world experience.

## Acceptance Criteria

**Given** FR-5.2 specifies contractor feedback
**When** I implement contractor feedback interface
**Then** tenant dashboard shows "Estimates Awaiting Feedback" list (filterable, sortable)

**And** clicking estimate opens feedback form with:
- Original estimate summary (read-only)
- Customer feedback (if submitted, read-only)
- Actual cost breakdown by category
- Actual timeline (start/end dates)
- Discrepancy explanation (textarea)
- Flags: scope_creep, hidden_costs, market_changes, estimate_too_high, estimate_too_low

**And** `POST /api/v1/feedback/contractor` endpoint saves feedback, links to customer feedback, flags >20% discrepancies, updates estimate status

**And** when customer and contractor feedback variance >10%
**Then** system creates review task for admin

## Tasks / Subtasks

- [ ] Create contractor feedback dashboard UI
- [ ] Create estimates awaiting feedback list
- [ ] Add filtering and sorting
- [ ] Create feedback form
- [ ] Create `POST /api/v1/feedback/contractor` endpoint
- [ ] Link to customer feedback
- [ ] Flag large discrepancies (>20%)
- [ ] Update estimate status
- [ ] Create admin review tasks
- [ ] Test feedback submission

## Dev Notes

### Prerequisites

Story 6.1 (customer feedback system)

### Technical Notes

- Contractor feedback optional but encouraged (gamify: "80% feedback rate!")
- Allow partial feedback (cost only, timeline only)
- Auto-fill cost breakdown from original estimate as starting point
- Track contractor feedback rate per tenant
- Highlight outliers (>30% variance) for investigation

### References

- [Source: docs/epics.md#Story-6-2]
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
