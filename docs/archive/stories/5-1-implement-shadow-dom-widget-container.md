# Story 5.1: Implement Shadow DOM Widget Container

Status: backlog

## Story

As a developer,
I want the widget to render in Shadow DOM with isolated styles,
So that it doesn't conflict with the host website's CSS and works reliably across all sites.

## Acceptance Criteria

**Given** the architecture specifies Shadow DOM for style isolation
**When** I implement the widget container
**Then** `apps/efofx-widget/src/components/WidgetContainer.tsx` contains:
- Shadow DOM root creation and attachment
- Style isolation (Tailwind styles don't leak to host page)
- Widget mount/unmount lifecycle
- Z-index management for overlay positioning
- Responsive container that adapts to screen size

**And** widget initialization creates Shadow DOM in `apps/efofx-widget/src/main.tsx`

**And** when embedded on any website
**Then** host page styles don't affect widget appearance
**And** widget styles don't affect host page

**And** widget renders correctly on mobile (320px+) and desktop (1920px+)

## Tasks / Subtasks

- [ ] Create WidgetContainer.tsx component
- [ ] Implement Shadow DOM creation and attachment
- [ ] Configure style isolation
- [ ] Implement mount/unmount lifecycle
- [ ] Add z-index management
- [ ] Make container responsive
- [ ] Test on various websites (Bootstrap, Material-UI, Tailwind)
- [ ] Test mobile and desktop rendering

## Dev Notes

### Prerequisites

Story 1.1 (widget project initialized)

### Technical Notes

- Follow architecture: Widget → Shadow DOM for Style Isolation
- Use `adoptedStyleSheets` for injecting Tailwind CSS into Shadow DOM
- Test on sites with Bootstrap, Material-UI, Tailwind to verify isolation
- Handle iframe edge cases if host uses CSP

### References

- [Source: docs/epics.md#Story-5-1]
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
