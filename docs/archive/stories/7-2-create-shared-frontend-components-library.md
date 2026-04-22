# Story 7.2: Create Shared Frontend Components Library

Status: backlog

## Story

As a developer,
I want common React components extracted to a shared library,
So that widget UI is consistent and reusable.

## Acceptance Criteria

**Given** widget is fully implemented (Epic 5)
**When** I extract shared components
**Then** create `apps/efofx-widget/src/components/shared/`:
- Button.tsx (primary/secondary/tertiary variants, loading/disabled states, icon support)
- Input.tsx (text/email/phone/number variants, validation display, label/help text)
- Card.tsx (header/body/footer sections, consistent padding)
- LoadingSpinner.tsx (branded with primary color, size variants)
- Modal.tsx (overlay with close, accessible, ESC to close)

**And** refactor existing components to use shared library

**And** ensure all shared components have TypeScript types, accessibility, mobile-responsive design, brand color theming

## Tasks / Subtasks

- [ ] Create `src/components/shared/` directory
- [ ] Create Button.tsx component
- [ ] Create Input.tsx component
- [ ] Create Card.tsx component
- [ ] Create LoadingSpinner.tsx component
- [ ] Create Modal.tsx component
- [ ] Refactor existing components to use shared library
- [ ] Add TypeScript types
- [ ] Add accessibility (ARIA labels)
- [ ] Test mobile responsiveness
- [ ] Test brand theming

## Dev Notes

### Prerequisites

Epic 5 complete (widget implemented)

### Technical Notes

- Consider headless UI libraries (Radix, Headless UI) for accessibility
- Use Tailwind's @apply directive for consistent styling
- Shared components should be pure (no business logic)
- Test shared components in isolation

### References

- [Source: docs/epics.md#Story-7-2]
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
