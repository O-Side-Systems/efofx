# Story 5.9: Manual QA Gate - Widget Integration Verification

Status: backlog

## Story

As a QA tester,
I want a comprehensive test guide to verify the widget works perfectly on real contractor websites,
So that I can approve Epic 5 before production launch.

## Acceptance Criteria

**Given** Epic 5 implementation is complete
**When** I generate the QA test guide
**Then** the file `docs/test-guides/epic-5-widget-qa-guide.md` includes sections for:
- Prerequisites (test websites, devices, browsers)
- Test Cases - Installation (embed on various platforms)
- Test Cases - Branding (logo, colors, messages)
- Test Cases - Conversational UI (chat flow, persistence)
- Test Cases - Lead Capture (validation, notifications)
- Test Cases - Estimate Display (formatting, responsiveness)
- Test Cases - Security (XSS, API keys, HTTPS, rate limiting)
- Test Cases - Performance (bundle size, load time, Lighthouse score)
- Test Cases - Cross-Browser Compatibility (Chrome, Firefox, Safari, Edge)
- Expected Results
- Known Limitations

## Tasks / Subtasks

- [ ] Create comprehensive QA test guide
- [ ] Document prerequisites
- [ ] Create installation test cases
- [ ] Create branding test cases
- [ ] Create conversational UI test cases
- [ ] Create lead capture test cases
- [ ] Create estimate display test cases
- [ ] Create security test cases
- [ ] Create performance test cases
- [ ] Create cross-browser test cases
- [ ] Document expected results
- [ ] Document known limitations

## Dev Notes

### Prerequisites

Stories 5.1-5.8

### Technical Notes

- Test on real contractor websites before launch
- Use BrowserStack for cross-browser testing
- Monitor Sentry for widget errors during QA
- Create video recordings of successful flows
- Get feedback from 3 non-technical users

### References

- [Source: docs/epics.md#Story-5-9]
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
