# Story 5.4: Implement Lead Capture Form

Status: backlog

## Story

As a contractor,
I want to capture visitor email/phone before showing estimate,
So that I can follow up on leads even if they don't book immediately.

## Acceptance Criteria

**Given** FR-6.4 specifies configurable lead capture
**When** I implement lead capture
**Then** `apps/efofx-widget/src/components/LeadCapture.tsx` displays form when chat reaches ready_for_estimate and lead not captured

**And** form fields: Email (required), Phone (optional), Name (optional), Privacy policy agreement (required if GDPR)

**And** tenant configuration controls: lead_capture enabled/disabled, timing (before/after estimate)

**And** when lead submits form
**Then** `POST /api/v1/widget/leads` saves lead data and sends notifications to tenant and lead

## Tasks / Subtasks

- [ ] Create LeadCapture.tsx component
- [ ] Add form fields (email, phone, name, privacy checkbox)
- [ ] Implement email validation
- [ ] Implement phone formatting
- [ ] Create `POST /api/v1/widget/leads` endpoint
- [ ] Send tenant notification email
- [ ] Send lead confirmation email
- [ ] Implement GDPR compliance (geo-IP detection)
- [ ] Implement lead deduplication
- [ ] Test full lead capture flow

## Dev Notes

### Prerequisites

Story 5.3 (chat UI ready)

### Technical Notes

- Email validation: RFC 5322 compliant regex
- Phone formatting: support US/international formats
- GDPR compliance: Privacy policy link required for EU visitors (use geo-IP)
- Lead deduplication: Same email + tenant within 7 days = existing lead
- A/B test lead capture timing (before vs after estimate)

### References

- [Source: docs/epics.md#Story-5-4]
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
