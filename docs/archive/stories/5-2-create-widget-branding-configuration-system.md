# Story 5.2: Create Widget Branding Configuration System

Status: backlog

## Story

As a contractor (tenant),
I want to customize the widget's appearance to match my brand,
So that customers see my branding, not efOfX's.

## Acceptance Criteria

**Given** FR-6.2 specifies branding customization
**When** I implement branding configuration
**Then** `GET /api/v1/widget/config?api_key={key}` endpoint returns branding config (logo_url, primary_color, button_text, welcome_message) and session_token

**And** widget initialization fetches branding and applies dynamically:
- Logo displayed in widget header
- Primary color used for buttons, chat bubbles, accents
- Button text customizable
- Welcome message personalized
- No efOfX branding visible anywhere

**And** tenant can preview branding changes in dashboard before publishing

**And** branding changes propagate to all widget instances within 5 minutes (CDN cache)

## Tasks / Subtasks

- [ ] Create `GET /api/v1/widget/config` endpoint
- [ ] Fetch tenant branding from database
- [ ] Generate session token
- [ ] Implement widget branding fetch on init
- [ ] Apply logo to widget header
- [ ] Apply primary color to UI elements
- [ ] Apply custom button text
- [ ] Apply custom welcome message
- [ ] Implement branding preview mode
- [ ] Test branding propagation

## Dev Notes

### Prerequisites

Story 3.1 (tenant branding stored), Story 5.1 (widget container ready)

### Technical Notes

- Cache branding config in widget localStorage (24 hour TTL)
- Validate color format (hex, rgb, hsl)
- Logo dimensions: max 200x60px, auto-scaled
- Fallback to default branding if fetch fails
- Preview mode: `?preview_branding=tenant_id` for testing

### References

- [Source: docs/epics.md#Story-5-2]
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
