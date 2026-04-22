# Epic 2: RCF Engine & Synthetic Data - QA Test Guide

**Version:** 1.0
**Date:** 2025-11-16
**Epic:** 2 - Reference Class Engine & Synthetic Data
**Status:** Ready for QA Testing

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Test Cases](#test-cases)
3. [Example API Requests](#example-api-requests)
4. [Expected Results](#expected-results)
5. [Known Limitations](#known-limitations)
6. [Edge Cases & Error Handling](#edge-cases--error-handling)

---

## Prerequisites

Before beginning QA testing, ensure the following are in place:

### 1. Database Setup
- [ ] MongoDB Atlas connection is active
- [ ] Synthetic data has been generated and seeded
- [ ] Verify ~100 reference classes exist in database:
  ```bash
  # Run from apps/efofx-estimate
  python test_count.py
  ```
  Or use the one-liner:
  ```bash
  python -c "import asyncio; from app.db.mongodb import *; async def main(): await connect_to_mongo(); print(await get_reference_classes_collection().count_documents({'is_synthetic': True})); asyncio.run(main())"
  ```
- [ ] Expected: 96 reference classes (7 types × 4 regions × multiple variations)

### 2. Synthetic Data Generator
- [ ] Synthetic data generator is available at `apps/synthetic-data-generator/`
- [ ] Can re-seed database if needed:
  ```bash
  cd apps/synthetic-data-generator
  python seed_database.py
  ```

### 3. Test Environment
- [ ] Python virtual environment activated with dependencies installed
- [ ] Test framework (pytest) available
- [ ] Access to test files at `apps/efofx-estimate/tests/services/test_rcf_engine.py`

### 4. Sample Test Data
Sample project descriptions for testing:
- **High Specificity:** "I want to build a 400 sq ft concrete swimming pool with spa in Southern California coastal area"
- **Medium Specificity:** "Need a residential pool installation in SoCal"
- **Low Specificity:** "backyard water feature project"
- **Ambiguous:** "need something built"

---

## Test Cases

### TC2.1: High Confidence Match
**Objective:** Verify RCF engine matches specific descriptions with high confidence

**Test Steps:**
1. Call `find_matching_reference_class()` with:
   - Description: "I want to build a residential swimming pool with concrete finish in my backyard"
   - Category: "construction"
   - Region: "us-ca-south-coastal"
   - Tenant ID: None

**Expected Results:**
- ✓ Match found (no ValueError)
- ✓ Confidence score >= 0.8
- ✓ Reference class name contains "Pool"
- ✓ Region matches "us-ca-south-coastal"
- ✓ Processing time < 50ms

**Validation Command:**
```python
from app.services.rcf_engine import find_matching_reference_class
import asyncio

result = asyncio.run(find_matching_reference_class(
    "I want to build a residential swimming pool with concrete finish in my backyard",
    "construction",
    "us-ca-south-coastal",
    None
))
print(f"Confidence: {result['confidence']}")
print(f"Reference Class: {result['reference_class']['name']}")
```

---

### TC2.2: Low Confidence Match / Error Handling
**Objective:** Verify system returns error for ambiguous descriptions

**Test Steps:**
1. Call `find_matching_reference_class()` with:
   - Description: "something vague"
   - Category: "construction"
   - Region: "us-tx-houston"  # Different region
   - Tenant ID: None

**Expected Results:**
- ✓ Raises ValueError with message "Could not find a confident match"
- ✓ Error message suggests providing more details
- ✓ No partial/incomplete results returned

---

### TC2.3: Baseline Estimate Calculation
**Objective:** Verify baseline cost and timeline estimates are realistic

**Test Steps:**
1. Get a matched reference class from TC2.1
2. Call `calculate_baseline_estimate(reference_class)`

**Expected Results for Medium Pool (~$55k baseline):**
- ✓ P50 cost: $50k - $60k (±10% tolerance)
- ✓ P80 cost: $65k - $80k
- ✓ Variance: P80 - P50 (reasonable spread)
- ✓ P50 timeline: 45-55 days
- ✓ P80 timeline: 60-70 days
- ✓ Cost breakdown includes: materials, labor, equipment, permits, finishing
- ✓ Processing time < 10ms

**Validation:**
```python
from app.services.rcf_engine import calculate_baseline_estimate

baseline = calculate_baseline_estimate(result['reference_class'])
print(f"P50 Cost: ${baseline['p50_cost']:,.0f}")
print(f"P80 Cost: ${baseline['p80_cost']:,.0f}")
print(f"P50 Timeline: {baseline['p50_timeline_days']} days")
```

---

### TC2.4: Cost Breakdown Accuracy
**Objective:** Verify cost breakdown sums exactly to P50 cost

**Test Steps:**
1. Get baseline estimate from TC2.3
2. Sum all categories in cost_breakdown

**Expected Results:**
- ✓ Sum of breakdown categories == P50 cost (within $0.01)
- ✓ All categories have positive values
- ✓ Breakdown percentages are reasonable:
  - Materials: 35-50%
  - Labor: 25-40%
  - Equipment: 5-15%
  - Permits: 2-10%
  - Overhead/Finishing: 5-20%

**Validation:**
```python
breakdown = baseline['cost_breakdown']
total = sum(breakdown.values())
print(f"Breakdown Total: ${total:,.2f}")
print(f"P50 Cost: ${baseline['p50_cost']:,.2f}")
print(f"Difference: ${abs(total - baseline['p50_cost']):,.2f}")
assert abs(total - baseline['p50_cost']) <= 0.01, "Breakdown doesn't sum to P50!"
```

---

### TC2.5: Complexity Adjustment
**Objective:** Verify complexity multiplier is applied correctly

**Test Steps:**
1. Get baseline estimate
2. Apply adjustments with complexity="complex" (1.5x)

**Expected Results:**
- ✓ Complexity factor == 1.5
- ✓ Adjusted P50 == Baseline P50 × 1.5
- ✓ Adjusted P80 == Baseline P80 × 1.5
- ✓ Adjusted timeline == Baseline timeline × 1.5
- ✓ All breakdown categories multiplied by 1.5

**Validation:**
```python
from app.services.rcf_engine import apply_adjustments

adjusted = apply_adjustments(baseline, complexity="complex")
print(f"Baseline P50: ${baseline['p50_cost']:,.0f}")
print(f"Adjusted P50: ${adjusted['adjusted_p50']:,.0f}")
print(f"Expected: ${baseline['p50_cost'] * 1.5:,.0f}")
assert adjusted['adjusted_p50'] == baseline['p50_cost'] * 1.5
```

---

### TC2.6: Risk Adjustment
**Objective:** Verify risk multiplier is applied to cost but NOT timeline

**Test Steps:**
1. Apply adjustments with complexity="standard", risk_level="high" (1.3x)

**Expected Results:**
- ✓ Risk factor == 1.3
- ✓ Adjusted P50 == Baseline P50 × 1.0 × 1.3 (no complexity adjustment)
- ✓ Adjusted timeline == Baseline timeline (UNCHANGED - risk doesn't affect timeline)

**Validation:**
```python
adjusted = apply_adjustments(baseline, risk_level="high")
assert adjusted['risk_factor'] == 1.3
assert adjusted['adjusted_p50'] == baseline['p50_cost'] * 1.3
assert adjusted['adjusted_p50_timeline'] == baseline['p50_timeline_days']  # Unchanged!
```

---

### TC2.7: All Construction Types Available
**Objective:** Verify all 7 construction types have reference classes

**Test Steps:**
1. Query database for count by subcategory

**Expected Results:**
- ✓ Pool: 16 classes (4 sizes × 4 regions)
- ✓ ADU: 12 classes (3 configs × 4 regions)
- ✓ Kitchen: 12 classes (3 levels × 4 regions)
- ✓ Bathroom: 12 classes (3 levels × 4 regions)
- ✓ Landscaping: 12 classes (3 scopes × 4 regions)
- ✓ Roofing: 16 classes (4 materials × 4 regions)
- ✓ Flooring: 16 classes (4 materials × 4 regions)

**Total Expected:** ~96-100 classes

---

### TC2.8: Regional Coverage
**Objective:** Verify all 4 California regions represented for each type

**Expected Regions:**
- `us-ca-south-coastal` (SoCal Coastal - highest cost, baseline)
- `us-ca-south-inland` (SoCal Inland - 15% lower)
- `us-ca-north` (NorCal - 10% lower)
- `us-ca-central-coast` (Central Coast - 5% lower)

**Validation:**
```bash
cd apps/synthetic-data-generator
python validate_synthetic_data.py
```

Expected output: All costs within ±25% of HomeAdvisor 2024 averages

---

### TC2.9: Performance Requirements
**Objective:** Verify all operations complete within performance SLAs

**Test Steps:**
1. Run performance tests for all functions

**Expected Results:**
- ✓ RCF matching (p95): < 50ms
- ✓ Baseline calculation: < 10ms
- ✓ Adjustments: < 5ms
- ✓ End-to-end estimate: < 100ms

**Validation:**
Run pytest with performance tests:
```bash
cd apps/efofx-estimate
pytest tests/services/test_rcf_engine.py::TestRCFMatching::test_performance_requirement -v
pytest tests/services/test_rcf_engine.py::TestBaselineEstimateCalculation::test_performance_requirement -v
```

---

### TC2.10: Synthetic Data Validation
**Objective:** Verify synthetic costs match HomeAdvisor 2024 benchmarks

**HomeAdvisor 2024 Reference Costs (SoCal Coastal):**
- Pool Small: $35,000 ±25%
- Pool Medium: $55,000 ±25%
- Pool Large: $85,000 ±25%
- ADU Studio: $120,000 ±25%
- Kitchen Midrange: $30,000 ±25%
- Bathroom Midrange: $18,000 ±25%

**Test Steps:**
1. Run validation script
2. Verify all baseline (coastal) costs within tolerance

**Validation:**
```bash
cd apps/synthetic-data-generator
python validate_synthetic_data.py
```

**Expected Results:**
- ✓ 100% of reference classes pass validation
- ✓ P50 costs within ±25% of HomeAdvisor averages
- ✓ No validation failures reported

---

## Example API Requests

### Example 1: Pool Estimation
```bash
curl -X POST http://localhost:8000/api/v1/estimate \
  -H "Content-Type: application/json" \
  -d '{
    "description": "I want a 400 sq ft concrete swimming pool with spa",
    "category": "construction",
    "region": "us-ca-south-coastal",
    "complexity": "standard",
    "risk_level": "low"
  }'
```

### Example 2: ADU Estimation
```bash
curl -X POST http://localhost:8000/api/v1/estimate \
  -H "Content-Type: application/json" \
  -d '{
    "description": "I need a one bedroom accessory dwelling unit in my backyard",
    "category": "construction",
    "region": "us-ca-north",
    "complexity": "standard",
    "risk_level": "medium"
  }'
```

### Example 3: Kitchen Renovation
```bash
curl -X POST http://localhost:8000/api/v1/estimate \
  -H "Content-Type: application/json" \
  -d '{
    "description": "Full kitchen remodel with new cabinets and granite countertops",
    "category": "construction",
    "region": "us-ca-south-inland",
    "complexity": "complex",
    "risk_level": "low"
  }'
```

---

## Expected Results

### 1. Match Confidence Scores
- **High specificity** (detailed description with keywords): 0.8 - 1.0
- **Medium specificity** (some keywords, category clear): 0.7 - 0.8
- **Low specificity** (vague, few keywords): < 0.7 (error)

### 2. Cost Realism
- P50 costs should align with HomeAdvisor 2024 averages
- P80 costs should be ~20-40% higher than P50 (realistic variance)
- Cost breakdowns should be industry-standard:
  - Materials: 40-50% for construction
  - Labor: 25-40%
  - Overhead/other: 15-25%

### 3. Regional Adjustments
Already baked into reference classes:
- SoCal Coastal: Highest (baseline)
- SoCal Inland: -15%
- NorCal: -10%
- Central Coast: -5%

### 4. All Reference Classes Marked Synthetic
- Every reference class should have `is_synthetic: true`
- Every class should have `tenant_id: null` (platform-provided)
- Validation source should reference HomeAdvisor 2024

---

## Known Limitations

### Current MVP Limitations
1. **No Real Data:** All estimates based on synthetic data generated from HomeAdvisor 2024 averages
2. **Simple Keyword Matching:** Uses basic keyword overlap, no ML/NLP or semantic understanding
3. **No LLM Integration:** No natural language explanations or conversational scoping (coming in Epic 4)
4. **No Authentication:** No tenant isolation or API key management (coming in Epic 3)
5. **Limited Domains:** Only construction projects supported (pools, ADUs, renovations, etc.)

### Known Edge Cases
- Very short descriptions (<10 words) may have low confidence
- Descriptions without category-specific keywords will fail matching
- Regions outside California not supported
- Cost distributions are lognormal approximations, not actual project data

### Future Enhancements
- Real customer feedback integration (Epic 6)
- LLM-powered conversational scoping (Epic 4)
- Multi-tenant isolation and BYOK encryption (Epic 3)
- Additional domains beyond construction

---

## Edge Cases & Error Handling

### Test Edge Cases

#### 1. Empty Description
```python
# Should raise ValueError
find_matching_reference_class("", "construction", "us-ca-south-coastal", None)
# Expected: ValueError "provide more details"
```

#### 2. Unknown Category
```python
# Should raise ValueError
find_matching_reference_class("build something", "unknown_category", "us-ca-south-coastal", None)
# Expected: ValueError "No reference classes available"
```

#### 3. Unsupported Region
```python
# Should have lower confidence or fail
find_matching_reference_class("pool installation", "construction", "us-ny-nyc", None)
# Expected: Low confidence or error (region mismatch)
```

#### 4. Invalid Complexity/Risk
```python
# Should default to standard/low (1.0x)
apply_adjustments(baseline, complexity="invalid", risk_level="unknown")
# Expected: Both factors == 1.0, no error
```

#### 5. Rounding Edge Cases
```python
# Cost breakdown should still sum to P50 exactly
baseline = calculate_baseline_estimate(ref_class_with_awkward_cost)
total = sum(baseline['cost_breakdown'].values())
assert abs(total - baseline['p50_cost']) <= 0.01
```

---

## QA Sign-Off Checklist

- [ ] All 10 test cases (TC2.1 - TC2.10) pass
- [ ] Performance requirements met (<50ms matching, <10ms calculation)
- [ ] Synthetic data validates against HomeAdvisor 2024 benchmarks
- [ ] Cost breakdowns sum correctly (within $0.01)
- [ ] Regional adjustments are accurate
- [ ] Complexity and risk adjustments multiply correctly
- [ ] Error handling works for edge cases
- [ ] All 7 construction types have reference classes
- [ ] All 4 regions represented for each type
- [ ] Known limitations are acceptable for MVP

**QA Tester Name:** _______________
**Date:** _______________
**Approval:** ☐ PASS  ☐ FAIL
**Notes:** _______________

---

**End of QA Test Guide**
