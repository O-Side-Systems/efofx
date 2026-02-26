# Story 2.5: Apply Regional & Complexity Adjustments

Status: done

## Story

As a system,
I want to apply regional multipliers and complexity/risk factors to baseline estimates,
So that estimates account for location and project-specific factors.

## Acceptance Criteria

**Given** a baseline estimate is calculated
**When** regional and complexity adjustments are applied
**Then** `app/services/rcf_engine.py::apply_adjustments()` modifies:
- Regional multiplier (already baked into reference class, verify it's applied)
- Complexity factor: 0.8x (simple) to 1.5x (complex) based on user input
- Risk factor: 1.0x (low) to 1.3x (high) based on project attributes

**And** final cost = baseline × complexity × risk

**And** final timeline = baseline_timeline × complexity (risk doesn't affect timeline)

**And** calculation breakdown is returned showing:
```python
{
  "baseline_p50": 75000,
  "complexity_factor": 1.2,
  "risk_factor": 1.1,
  "adjusted_p50": 99000  # 75000 * 1.2 * 1.1
}
```

**And** when complexity or risk is not provided
**Then** defaults to 1.0 (no adjustment)

## Tasks / Subtasks

- [x] Implement `apply_adjustments()` function
- [x] Map complexity input to multiplier (simple=0.8, standard=1.0, complex=1.5)
- [x] Implement risk factor calculation
- [x] Apply multiplicative adjustments to costs
- [x] Apply complexity adjustment to timeline
- [x] Return breakdown showing all factors
- [x] Handle missing complexity/risk (default to 1.0)
- [x] Store adjustment factors in estimate document
- [x] Test with various adjustment combinations

## Dev Notes

### Prerequisites

Story 2.4 (baseline calculation works)

### Technical Notes

- Complexity factor from user input: "simple" (0.8), "standard" (1.0), "complex" (1.5)
- Risk factor from heuristics: foundation issues, tight timeline, custom requirements
- Store adjustment factors in estimate document for transparency
- Apply multiplicatively: final = baseline × regional × complexity × risk

### References

- [Source: docs/epics.md#Story-2-5]
- [Source: docs/PRD.md] (for requirements context)

## Dev Agent Record

### Context Reference

<!-- Path(s) to story context XML will be added here by context workflow -->

### Agent Model Used

claude-sonnet-4-5-20250929

### Debug Log References

Implementation completed in single session. All acceptance criteria met:
- ✓ Implemented apply_adjustments() in app/services/rcf_engine.py
- ✓ Maps complexity to multipliers: simple (0.8), standard (1.0), complex (1.5)
- ✓ Maps risk to multipliers: low (1.0), medium (1.15), high (1.3)
- ✓ Applies multiplicative formula: final_cost = baseline × complexity × risk
- ✓ Applies complexity to timeline: final_timeline = baseline_timeline × complexity
- ✓ Risk does NOT affect timeline (as per requirements)
- ✓ Returns comprehensive breakdown with all factors and values
- ✓ Defaults to standard/low (1.0) when inputs not provided
- ✓ Handles case-insensitive inputs and invalid values
- ✓ 13 unit tests passing covering all adjustment combinations

### Completion Notes List

**Implementation Summary:**
Added adjustment function to `app/services/rcf_engine.py`:

1. **Function: apply_adjustments(baseline_estimate, complexity, risk_level)**
   - Takes baseline estimate and applies complexity/risk multipliers
   - Complexity: simple (0.8x), standard (1.0x), complex (1.5x)
   - Risk: low (1.0x), medium (1.15x), high (1.3x)

2. **Adjustment Formula:**
   - Cost: final = baseline × complexity × risk
   - Timeline: final = baseline × complexity (risk doesn't affect timeline)
   - Breakdown: Each category adjusted proportionally with rounding handling

3. **Response Structure:**
   - Preserves baseline values for comparison
   - Includes both factors (complexity_factor, risk_factor)
   - Provides adjusted P50/P80 costs and timelines
   - Includes human-readable adjustment summary
   - Adjusted cost breakdown sums exactly to adjusted P50

4. **Input Handling:**
   - Case-insensitive (COMPLEX, complex, Complex all work)
   - Defaults: complexity='standard', risk_level='low'
   - Invalid inputs default to 1.0 (no adjustment)

**Test Coverage:**
- 13 comprehensive unit tests
- Tests for all complexity levels, risk levels, and combinations
- Tests for breakdown adjustment, variance recalculation, edge cases
- All tests passing

All acceptance criteria satisfied and ready for code review.

### File List

**NEW:**
- None

**MODIFIED:**
- apps/efofx-estimate/app/services/rcf_engine.py (added apply_adjustments function and COMPLEXITY_MULTIPLIERS constant)
- apps/efofx-estimate/tests/services/test_rcf_engine.py (added TestAdjustments class with 13 tests)

**DELETED:**
- None
