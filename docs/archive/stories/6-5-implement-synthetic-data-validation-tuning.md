# Story 6.5: Implement Synthetic Data Validation & Tuning

Status: backlog

## Story

As a system,
I want to compare synthetic reference classes to real feedback data,
So that synthetic data stays calibrated and accurate.

## Acceptance Criteria

**Given** FR-2.3 specifies synthetic data validation
**When** I implement validation
**Then** nightly batch job runs `app/jobs/validate_synthetic_data.py`:
- For each synthetic reference class with >=5 feedback submissions
- Calculate average actual cost vs synthetic P50/P80
- Calculate variance percentage
- Flag if variance >25% (needs tuning)
- Generate tuning recommendation

**And** `GET /admin/synthetic/validate` endpoint shows validation status and pending tuning recommendations

**And** when admin approves tuning
**Then** reference class updated with new values, change logged in audit trail, future estimates use updated values

**And** validation report emailed to admin weekly

## Tasks / Subtasks

- [ ] Create `app/jobs/validate_synthetic_data.py` batch job
- [ ] Calculate variance for each synthetic reference class
- [ ] Flag classes needing tuning (>25% variance)
- [ ] Generate tuning recommendations
- [ ] Create `GET /admin/synthetic/validate` endpoint
- [ ] Implement admin approval workflow
- [ ] Update reference classes with new values
- [ ] Create audit trail
- [ ] Schedule weekly validation reports
- [ ] Test validation and tuning

## Dev Notes

### Prerequisites

Story 6.3 (feedback data aggregated)

### Technical Notes

- Require minimum 5 feedback submissions before tuning (statistical significance)
- Use median instead of mean (resistant to outliers)
- Track how many estimates used each reference class version (for rollback)
- Consider seasonal adjustments (construction costs vary by quarter)
- Post-MVP: Automate tuning for well-calibrated classes (>20 data points)

### References

- [Source: docs/epics.md#Story-6-5]
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
