# Story 7.3: Apply YAGNI - Remove Unused Code and Features

Status: backlog

## Story

As a developer,
I want to identify and remove unused code, commented-out sections, and over-engineered features,
So that the codebase stays lean and maintainable.

## Acceptance Criteria

**Given** Epics 1-6 included some speculative code
**When** I audit for unused code
**Then** identify and remove:
- Backend: unused imports, commented-out code, unused API endpoints, over-engineered abstractions, dead code
- Frontend: unused React components, unused CSS classes, unused npm dependencies, commented-out console.log
- Configuration: unused environment variables, unused feature flags, unused database indexes

**And** document removal decisions

**And** run tests to confirm nothing broke after removal

**And** reduce overall codebase size by 10-15%

## Tasks / Subtasks

- [ ] Run autoflake to remove unused imports
- [ ] Remove commented-out code blocks
- [ ] Check access logs for unused endpoints
- [ ] Remove over-engineered abstractions
- [ ] Find and remove unreachable code
- [ ] Run PurgeCSS to remove unused CSS
- [ ] Run depcheck for unused npm deps
- [ ] Remove unused environment variables
- [ ] Document removal decisions
- [ ] Run all tests
- [ ] Verify codebase size reduction

## Dev Notes

### Prerequisites

Epics 1-6 complete

### Technical Notes

- Use coverage tools to find untested (potentially unused) code
- Be conservative - if unsure, leave it
- Comment out first, remove after 1 sprint with no issues
- Check git history to understand why code was added before removing

### References

- [Source: docs/epics.md#Story-7-3]
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
