# Story 5.7: Create Widget Embed Code & Documentation

Status: backlog

## Story

As a contractor,
I want simple copy-paste embed code for my website,
So that I can install the widget without technical expertise.

## Acceptance Criteria

**Given** FR-6.1 specifies <5 lines of code
**When** I create embed code
**Then** tenant dashboard displays simple embed code (<5 lines)

**And** `embed.js` behavior:
- Loads asynchronously (doesn't block page load)
- Creates floating button (bottom-right, customizable position)
- Button shows unread message badge if applicable
- Clicking button opens chat overlay
- Overlay closable via X button or clicking outside
- Widget state persists across page navigation

**And** installation documentation includes step-by-step guides for WordPress, Squarespace, Wix

**And** when widget fails to load
**Then** graceful fallback: "Chat temporarily unavailable" message

**And** widget bundle size: <50KB gzipped (meets NFR-U1)

## Tasks / Subtasks

- [ ] Create simple embed code template
- [ ] Implement async widget loading
- [ ] Create floating button component
- [ ] Add unread message badge
- [ ] Implement chat overlay
- [ ] Add close functionality
- [ ] Persist widget state across pages
- [ ] Create installation documentation
- [ ] Optimize bundle size (<50KB gzipped)
- [ ] Test on various platforms

## Dev Notes

### Prerequisites

Stories 5.1-5.6 (widget fully functional)

### Technical Notes

- Host `embed.js` on CDN (CloudFlare, DigitalOcean Spaces)
- Version embed.js: `/embed.js?v=1.0.0` for cache busting
- Provide test API key for demo purposes
- Create video tutorial for visual learners
- Track installation success rate (analytics)

### References

- [Source: docs/epics.md#Story-5-7]
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
