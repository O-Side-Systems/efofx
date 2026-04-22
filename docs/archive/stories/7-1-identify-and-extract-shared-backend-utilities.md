# Story 7.1: Identify and Extract Shared Backend Utilities

Status: backlog

## Story

As a developer,
I want common backend utilities consolidated into shared modules,
So that code isn't duplicated and changes propagate consistently.

## Acceptance Criteria

**Given** Epics 1-6 are complete
**When** I audit the codebase for duplication
**Then** identify and extract to `app/utils/`:
- common.py (format_currency, calculate_variance_pct, format_date_range, sanitize_input)
- validators.py (validate_email, validate_phone, validate_api_key_format, validate_cost_range)
- mongodb_helpers.py (get_tenant_filter, paginate_query, ensure_indexes)

**And** update all existing code to use shared utilities instead of inline logic

**And** remove duplicated code (DRY compliance)

**And** add unit tests for all shared utilities (>90% coverage)

**And** document each utility with docstrings and usage examples

## Tasks / Subtasks

- [ ] Audit codebase for duplicate patterns
- [ ] Create `app/utils/common.py`
- [ ] Create `app/utils/validators.py`
- [ ] Create `app/utils/mongodb_helpers.py`
- [ ] Extract shared functions
- [ ] Update existing code to use utilities
- [ ] Remove duplicated code
- [ ] Write unit tests (>90% coverage)
- [ ] Add docstrings to all utilities
- [ ] Test refactored code

## Dev Notes

### Prerequisites

Epics 1-6 complete

### Technical Notes

- Use ripgrep/grep to find duplicate patterns
- Maintain backward compatibility (don't break existing code)
- Consider edge cases in shared functions
- Shared code must have zero dependencies on specific features

### References

- [Source: docs/epics.md#Story-7-1]
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
