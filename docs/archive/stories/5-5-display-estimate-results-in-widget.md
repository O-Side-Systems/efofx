# Story 5.5: Display Estimate Results in Widget

Status: backlog

## Story

As a website visitor,
I want to see the estimate clearly formatted in the chat,
So that I understand the cost range and next steps.

## Acceptance Criteria

**Given** chat session reaches estimate-ready state
**When** estimate is generated
**Then** `apps/efofx-widget/src/components/EstimateDisplay.tsx` renders:
- Project summary (type, size, region)
- P50/P80 costs with labels
- Range visualization (horizontal bar)
- Timeline estimate
- Cost breakdown (collapsible accordion)
- Assumptions list (collapsible)
- Risks list (collapsible)
- LLM narrative (150-300 words)
- "Request Detailed Quote" CTA button

**And** estimate layout is mobile-responsive (stacked on mobile, two-column on desktop)

**And** when user clicks "Request Quote"
**Then** email sent to contractor with lead info and estimate summary

## Tasks / Subtasks

- [ ] Create EstimateDisplay.tsx component
- [ ] Display project summary
- [ ] Display P50/P80 costs
- [ ] Create range visualization
- [ ] Display timeline estimate
- [ ] Create collapsible cost breakdown
- [ ] Create collapsible assumptions/risks
- [ ] Display LLM narrative
- [ ] Add "Request Quote" CTA button
- [ ] Make layout responsive
- [ ] Implement quote request email
- [ ] Test on mobile and desktop

## Dev Notes

### Prerequisites

Story 4.3 (estimate API with narrative), Story 5.4 (lead captured)

### Technical Notes

- Use Chart.js or Recharts for P50/P80 visualization
- Collapsible sections default to collapsed on mobile
- Print-friendly CSS (optional: "Download PDF" button)
- Track conversion: lead capture → estimate view → quote request
- Store estimate in sessionStorage for revisiting

### References

- [Source: docs/epics.md#Story-5-5]
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
