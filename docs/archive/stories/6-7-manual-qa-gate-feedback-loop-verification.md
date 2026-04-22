# Story 6.7: Manual QA Gate - Feedback Loop Verification

Status: backlog

## Story

As a QA tester,
I want a test guide to verify the feedback system closes the loop and improves accuracy,
So that I can confirm Epic 6 delivers the self-improving promise.

## Acceptance Criteria

**Given** Epic 6 implementation is complete
**When** I generate the QA test guide
**Then** the file `docs/test-guides/epic-6-feedback-qa-guide.md` includes sections for:
- Prerequisites (test tenant with 20+ estimates)
- Test Cases - Customer Feedback (magic link flow, form submission, validation)
- Test Cases - Contractor Feedback (dashboard, submission, discrepancies)
- Test Cases - Calibration Metrics (calculation, accuracy, trends)
- Test Cases - Calibration Dashboard (UI, charts, insights, trust badge)
- Test Cases - Synthetic Data Validation (nightly job, tuning, audit)
- Test Cases - Prompt Refinement (analysis, A/B testing, promotion)
- Expected Results
- Known Limitations

## Tasks / Subtasks

- [ ] Create comprehensive QA test guide
- [ ] Document prerequisites
- [ ] Create customer feedback test cases
- [ ] Create contractor feedback test cases
- [ ] Create calibration metrics test cases
- [ ] Create calibration dashboard test cases
- [ ] Create synthetic data validation test cases
- [ ] Create prompt refinement test cases
- [ ] Document expected results
- [ ] Document known limitations

## Dev Notes

### Prerequisites

Stories 6.1-6.6

### Technical Notes

- Test full feedback loop with real emails (not just localhost)
- Manually verify calibration math with spreadsheet
- Test with edge cases: zero feedback, all too high, all too low
- Monitor email deliverability (SendGrid dashboard)
- Review 10+ feedback submissions for UX friction points

### References

- [Source: docs/epics.md#Story-6-7]
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
