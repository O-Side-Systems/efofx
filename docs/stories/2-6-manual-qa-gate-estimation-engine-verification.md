# Story 2.6: Manual QA Gate - Estimation Engine Verification

Status: done

## Story

As a QA tester,
I want a test guide to verify the RCF engine produces accurate, explainable estimates,
So that I can approve Epic 2 before widget integration begins.

## Acceptance Criteria

**Given** Epic 2 implementation is complete
**When** I generate the QA test guide
**Then** the file `docs/test-guides/epic-2-rcf-engine-qa-guide.md` includes:

**Section 1: Prerequisites**
- MongoDB populated with synthetic data (100 reference classes)
- Test API client (Postman, curl, or Python requests)
- Sample project descriptions for testing

**Section 2: Test Cases**
- **TC2.1:** Match "pool installation in SoCal coastal" → confidence >= 0.8
- **TC2.2:** Match ambiguous "backyard project" → confidence < 0.7, error returned
- **TC2.3:** Baseline estimate for matched pool → P50 ~$75k, P80 ~$92k (±10% tolerance)
- **TC2.4:** Cost breakdown sums to P50 exactly
- **TC2.5:** Apply complexity=1.2 → adjusted cost = baseline * 1.2
- **TC2.6:** Apply risk=1.1 → adjusted cost = baseline * complexity * risk
- **TC2.7:** Verify all 7 construction types have reference classes
- **TC2.8:** Verify all 4 regions represented for each type
- **TC2.9:** Response time < 100ms for estimate calculation
- **TC2.10:** Synthetic data validation: costs within ±25% of HomeAdvisor 2024

**Section 3: Example API Requests**
```bash
curl -X POST http://localhost:8000/api/v1/estimate \
  -H "Content-Type: application/json" \
  -d '{
    "project_type": "Pool Installation",
    "region": "SoCal - Coastal",
    "description": "15x30 in-ground pool with spa",
    "complexity": "standard"
  }'
```

**Section 4: Expected Results**
- Match confidence scores make sense (higher for specific descriptions)
- P50/P80 values are realistic compared to industry averages
- Cost breakdowns are reasonable (materials typically 40-50% for construction)
- Adjustments multiply correctly
- All reference classes marked `is_synthetic: true`

**Section 5: Known Limitations**
- No real project data yet (all synthetic)
- Simple keyword matching (no ML/NLP)
- No LLM narrative generation yet (Epic 4)
- No user authentication yet (Epic 3)

## Tasks / Subtasks

- [x] Create comprehensive QA test guide
- [x] Document prerequisites
- [x] Create detailed test cases with expected results
- [x] Provide example API request samples
- [x] Document expected results
- [x] Document known limitations
- [x] Include HomeAdvisor 2024 validation data
- [x] Add edge case testing instructions

## Dev Notes

### Prerequisites

Stories 2.1-2.5

### Technical Notes

- Include HomeAdvisor 2024 pool cost range for validation: $50k-$120k (SoCal)
- Verify lognormal distribution shape (right-skewed, no negative costs)
- Test edge cases: empty description, unknown region, invalid category

### References

- [Source: docs/epics.md#Story-2-6]
- [Source: docs/PRD.md] (for requirements context)

## Dev Agent Record

### Context Reference

<!-- Path(s) to story context XML will be added here by context workflow -->

### Agent Model Used

claude-sonnet-4-5-20250929

### Debug Log References

QA test guide created in single session. All acceptance criteria met:
- ✓ Created comprehensive QA test guide at docs/test-guides/epic-2-rcf-engine-qa-guide.md
- ✓ Documented prerequisites (database setup, test environment, sample data)
- ✓ Created 10 detailed test cases (TC2.1 - TC2.10) with validation commands
- ✓ Provided example API requests for pool, ADU, and kitchen estimations
- ✓ Documented expected results with specific tolerances
- ✓ Included known limitations section
- ✓ Added edge case testing (empty description, unknown category, unsupported region, invalid inputs)
- ✓ Included HomeAdvisor 2024 validation benchmarks
- ✓ Added QA sign-off checklist

### Completion Notes List

**Implementation Summary:**
Created comprehensive QA test guide for Epic 2 verification:

1. **Prerequisites Section:**
   - Database setup checklist with verification commands
   - Synthetic data generator availability
   - Test environment requirements
   - Sample test data with varying specificity levels

2. **Test Cases (TC2.1 - TC2.10):**
   - TC2.1: High confidence match (>= 0.8, specific descriptions)
   - TC2.2: Low confidence/error handling (ambiguous descriptions)
   - TC2.3: Baseline estimate accuracy (P50/P80 within ±10% tolerance)
   - TC2.4: Cost breakdown sums exactly to P50 (within $0.01)
   - TC2.5: Complexity adjustment (simple=0.8x, standard=1.0x, complex=1.5x)
   - TC2.6: Risk adjustment (low=1.0x, medium=1.15x, high=1.3x, timeline unchanged)
   - TC2.7: All 7 construction types available (pool, ADU, kitchen, bathroom, landscaping, roofing, flooring)
   - TC2.8: All 4 CA regions represented (coastal, inland, north, central coast)
   - TC2.9: Performance requirements (<50ms matching, <10ms calculation, <100ms end-to-end)
   - TC2.10: Synthetic data validation (within ±25% of HomeAdvisor 2024)

3. **Example API Requests:**
   - Pool estimation with curl example
   - ADU estimation example
   - Kitchen renovation example

4. **Expected Results:**
   - Match confidence score guidelines
   - Cost realism benchmarks
   - Regional adjustment verification
   - Synthetic data marking validation

5. **Known Limitations:**
   - No real data (all synthetic from HomeAdvisor 2024)
   - Simple keyword matching (no ML/NLP)
   - No LLM integration yet (Epic 4)
   - No authentication (Epic 3)
   - Limited to construction domain

6. **Edge Cases & Error Handling:**
   - Empty descriptions
   - Unknown categories
   - Unsupported regions
   - Invalid complexity/risk values
   - Cost breakdown rounding edge cases

**HomeAdvisor 2024 Reference Costs Included:**
- Pool Small: $35,000 ±25%
- Pool Medium: $55,000 ±25%
- Pool Large: $85,000 ±25%
- ADU Studio: $120,000 ±25%
- Kitchen Midrange: $30,000 ±25%
- Bathroom Midrange: $18,000 ±25%

All acceptance criteria satisfied and ready for QA team to execute tests.

### File List

**NEW:**
- docs/test-guides/epic-2-rcf-engine-qa-guide.md (comprehensive QA test guide with 10 test cases)

**MODIFIED:**
- None

**DELETED:**
- None
