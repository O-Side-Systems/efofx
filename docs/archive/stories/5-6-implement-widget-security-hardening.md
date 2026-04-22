# Story 5.6: Implement Widget Security Hardening

Status: backlog

## Story

As a developer,
I want the widget to be secure against XSS, CSRF, and data leakage,
So that it's safe to deploy on public contractor websites.

## Acceptance Criteria

**Given** FR-6.5 specifies security requirements
**When** I implement security measures
**Then** widget security includes:
- XSS Protection (DOMPurify sanitization, no dangerouslySetInnerHTML, CSP compatible)
- API Key Protection (session token exchange, never expose tenant API key)
- HTTPS Enforcement (widget and API requests only over HTTPS)
- Rate Limiting (50 messages per session, 10 sessions per IP/hour)
- CORS Configuration (proper credentials handling)

**And** security audit checklist passed (all items verified)

## Tasks / Subtasks

- [ ] Implement DOMPurify for input sanitization
- [ ] Create session token exchange endpoint
- [ ] Enforce HTTPS for widget and API
- [ ] Implement session message rate limiting
- [ ] Implement IP-based session rate limiting
- [ ] Configure CORS properly
- [ ] Complete security audit checklist
- [ ] Test XSS prevention
- [ ] Test rate limiting
- [ ] Document security practices

## Dev Notes

### Prerequisites

Story 5.3 (chat UI implemented)

### Technical Notes

- Use DOMPurify for sanitizing user messages
- Implement session token rotation (optional)
- Monitor widget for suspicious activity (Sentry)
- Document security practices for tenant awareness
- Consider Subresource Integrity (SRI) for CDN-hosted widget

### References

- [Source: docs/epics.md#Story-5-6]
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
