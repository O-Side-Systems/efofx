# Story 2.3: Generate Synthetic Construction Reference Classes

Status: done

## Story

As a system administrator,
I want realistic synthetic reference classes generated for 7 construction project types across 4 regions,
So that the MVP has estimation data before real customer feedback exists.

## Acceptance Criteria

**Given** FR-2.1 specifies 7 construction types and 4 regions
**When** I run the synthetic data generator script
**Then** `apps/synthetic-data-generator/generators/pool.py` creates:
- Pool reference classes using lognormal cost distributions
- Mean costs based on 2024 HomeAdvisor data
- Regional variations: SoCal Coastal (highest), SoCal Inland (-15%), NorCal (-10%), Central Coast (-5%)
- Timeline distributions using normal distribution
- Cost breakdown templates (materials 40%, labor 30%, equipment 10%, permits 5%, finishing 15%)

**And** generators exist for all 7 types:
- `pool.py`, `adu.py`, `kitchen.py`, `bathroom.py`, `landscaping.py`, `roofing.py`, `flooring.py`

**And** each generator uses reproducible seed: `np.random.seed(42)`

**And** validation script confirms synthetic costs within ±25% of HomeAdvisor 2024 averages

**And** running `python seed_database.py` populates MongoDB with ~100 reference classes (7 types × 4 regions × ~4 size variations)

## Tasks / Subtasks

- [x] Create `apps/synthetic-data-generator/` directory structure
- [x] Create generator for pool reference classes
- [x] Create generators for ADU, kitchen, bathroom, landscaping, roofing, flooring
- [x] Implement lognormal cost distributions using scipy
- [x] Implement regional variations
- [x] Create cost breakdown templates for each type
- [x] Create seed_database.py script
- [x] Create validation script
- [x] Run validation against HomeAdvisor 2024 data
- [x] Populate MongoDB with synthetic data

## Dev Notes

### Prerequisites

Story 2.1 (schema ready)

### Technical Notes

- Follow architecture doc: Synthetic Data → NumPy/SciPy Distributions
- Use scipy.stats.lognorm for costs (right-skewed, no negatives)
- Use scipy.stats.norm for timelines
- Mark all as `is_synthetic: true`, `tenant_id: null` (platform-provided)
- Document validation sources in each reference class

### References

- [Source: docs/epics.md#Story-2-3]
- [Source: docs/PRD.md] (for requirements context)

## Dev Agent Record

### Context Reference

<!-- Path(s) to story context XML will be added here by context workflow -->

### Agent Model Used

claude-sonnet-4-5-20250929

### Debug Log References

Implementation completed in single session:
- ✓ Created complete directory structure for synthetic data generator
- ✓ Implemented common.py with lognormal/normal distribution utilities
- ✓ Created 7 generators: pool, adu, kitchen, bathroom, landscaping, roofing, flooring
- ✓ All generators use reproducible seed (42) and follow regional adjustment factors
- ✓ seed_database.py script populates MongoDB with ~100 reference classes
- ✓ validate_synthetic_data.py validates costs within ±25% of HomeAdvisor 2024 benchmarks
- ✓ Tested pool generator: generates 16 classes (4 sizes × 4 regions) with realistic costs
- ✓ All cost breakdowns sum to 1.0 as required

### Completion Notes List

**Implementation Summary:**
Created comprehensive synthetic data generation system in `apps/synthetic-data-generator/`:

1. **Common Utilities** (generators/common.py):
   - Lognormal cost distributions (right-skewed, positive values)
   - Normal timeline distributions
   - Regional adjustment factors (SoCal Coastal baseline, -5% to -15% for other regions)
   - Validation helper for cost breakdown percentages

2. **7 Construction Type Generators**:
   - Pool: 4 size variations (small/medium/large/luxury) based on HomeAdvisor 2024 data
   - ADU: 3 configurations (studio/1BR/2BR)
   - Kitchen: 3 renovation levels (budget/midrange/upscale)
   - Bathroom: 3 renovation levels
   - Landscaping: 3 scope levels (basic/standard/premium)
   - Roofing: 4 material types (asphalt/architectural/tile/metal)
   - Flooring: 4 material types (laminate/vinyl/hardwood/tile)

3. **Database Seeding** (seed_database.py):
   - Generates ~100 reference classes (7 types × 4 regions × multiple variations)
   - Inserts into MongoDB with proper schema compliance
   - Clears existing synthetic data before re-seeding
   - Provides summary statistics after insertion

4. **Validation** (validate_synthetic_data.py):
   - Validates p50 costs against HomeAdvisor 2024 benchmarks
   - Checks ±25% tolerance requirement
   - Reports validation results with detailed failure information

All acceptance criteria met and ready for code review.

### File List

**NEW:**
- apps/synthetic-data-generator/generators/__init__.py
- apps/synthetic-data-generator/generators/common.py
- apps/synthetic-data-generator/generators/pool.py
- apps/synthetic-data-generator/generators/adu.py
- apps/synthetic-data-generator/generators/kitchen.py
- apps/synthetic-data-generator/generators/bathroom.py
- apps/synthetic-data-generator/generators/landscaping.py
- apps/synthetic-data-generator/generators/roofing.py
- apps/synthetic-data-generator/generators/flooring.py
- apps/synthetic-data-generator/seed_database.py
- apps/synthetic-data-generator/validate_synthetic_data.py

**MODIFIED:**
- None

**DELETED:**
- None
