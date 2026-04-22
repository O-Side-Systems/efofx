# Story 5.8: Implement Widget Analytics & Monitoring

Status: backlog

## Story

As a contractor,
I want to see widget performance metrics,
So that I know how many visitors engage and request estimates.

## Acceptance Criteria

**Given** tenants need visibility into widget effectiveness
**When** I implement analytics
**Then** widget tracks events: widget_loaded, widget_opened, message_sent, estimate_generated, lead_captured, quote_requested, widget_error

**And** events sent to backend: `POST /api/v1/widget/events`

**And** tenant dashboard displays metrics:
- Widget views, engagement rate, estimate conversion, lead capture rate, quote request rate, average conversation length
- Funnel visualization showing conversion at each step

**And** widget performance monitoring (load time, error rate, API response times, alerts)

## Tasks / Subtasks

- [ ] Define analytics event types
- [ ] Create `POST /api/v1/widget/events` endpoint
- [ ] Implement event tracking in widget
- [ ] Store events in database
- [ ] Create tenant dashboard metrics (post-MVP)
- [ ] Create funnel visualization (post-MVP)
- [ ] Implement performance monitoring
- [ ] Set up alerts for degraded performance
- [ ] Test event tracking

## Dev Notes

### Prerequisites

Story 5.7 (widget deployed)

### Technical Notes

- Use lightweight analytics (avoid Google Analytics bloat)
- Batch events (send every 30 seconds, not real-time)
- Respect Do Not Track (DNT) headers
- Store events for 90 days
- Dashboard built in tenant portal (post-MVP: separate story)

### References

- [Source: docs/epics.md#Story-5-8]
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
