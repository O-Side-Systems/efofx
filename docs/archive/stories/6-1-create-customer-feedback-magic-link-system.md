# Story 6.1: Create Customer Feedback Magic Link System

Status: backlog

## Story

As a customer,
I want to submit actual project outcomes via email link (no login required),
So that I can help improve estimates for future projects.

## Acceptance Criteria

**Given** FR-5.1 specifies magic link feedback
**When** I implement customer feedback
**Then** after estimate generated and lead email captured, system sends follow-up email 90 days later with magic link

**And** magic link format: `https://efofx.ai/feedback/{token}` (JWT with estimate_id, email, 90-day expiration, single-use)

**And** feedback form at magic link includes:
- Project summary (read-only)
- Original estimate range
- Actual final cost (required)
- Actual timeline in weeks (required)
- Accuracy rating (1-5 stars)
- What changed from estimate? (textarea, optional)
- Would you recommend this contractor? (yes/no)

**And** `POST /api/v1/feedback/customer` endpoint validates token, saves feedback, calculates variance, invalidates token, sends thank-you emails

## Tasks / Subtasks

- [ ] Create follow-up email template
- [ ] Schedule emails 90 days after estimate (SendGrid)
- [ ] Generate JWT magic link tokens
- [ ] Create feedback form page
- [ ] Create `POST /api/v1/feedback/customer` endpoint
- [ ] Implement token validation
- [ ] Calculate variance
- [ ] Invalidate token after submission
- [ ] Send thank-you emails
- [ ] Test full feedback flow

## Dev Notes

### Prerequisites

Story 4.3 (estimates include email), Story 5.4 (lead capture)

### Technical Notes

- Use SendGrid scheduled sends (90-day delay)
- Magic link expires after 90 days from email send
- Form is mobile-friendly (60% will use mobile)
- Track email open rate and link click rate
- A/B test email timing (60 days vs 90 days vs 120 days)

### References

- [Source: docs/epics.md#Story-6-1]
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
