# Story 2.1: Create Domain-Agnostic Reference Class MongoDB Schema

Status: done

## Story

As a developer,
I want MongoDB collections and Pydantic models for reference classes that work across any domain,
So that the same schema supports construction, IT/dev, and future domains.

## Acceptance Criteria

**Given** the architecture specifies domain-agnostic flexible schema
**When** I create the `reference_classes` collection schema
**Then** the Pydantic model in `app/models/reference_class.py` includes:
- `tenant_id`: ObjectId | None (null = platform-provided)
- `category`: str (e.g., "construction", "it_dev")
- `subcategory`: str (e.g., "pool", "api_development")
- `name`: str
- `description`: str
- `keywords`: List[str]
- `regions`: List[str]
- `attributes`: Dict[str, Any] (flexible, domain-specific)
- `cost_distribution`: {p50, p80, p95, currency}
- `timeline_distribution`: {p50_days, p80_days, p95_days}
- `cost_breakdown_template`: Dict[str, float] (percentages)
- `is_synthetic`: bool
- `validation_source`: str
- `created_at`: datetime

**And** MongoDB indexes are created via migration script:
```python
db.reference_classes.createIndex({"tenant_id": 1, "category": 1, "region": 1})
db.reference_classes.createIndex({"tenant_id": 1, "keywords": 1})
```

**And** the schema validates that cost_breakdown_template percentages sum to 1.0

## Tasks / Subtasks

- [x] Create Pydantic model `app/models/reference_class.py`
- [x] Add all required fields with proper types
- [x] Add Pydantic validator for cost_breakdown_template sum
- [x] Create MongoDB migration script for indexes
- [x] Test model validation with valid data
- [x] Test model validation with invalid data (percentages != 1.0)
- [x] Verify indexes created successfully

## Dev Notes

### Prerequisites

Story 1.4 (MongoDB connected)

### Technical Notes

- Follow architecture doc: Data Architecture → MongoDB Collections
- Use Pydantic validators for percentage sum validation
- Store attributes as flexible dict to support any domain
- Example construction attributes: {size_range, depth, includes_spa}
- Example IT attributes: {tech_stack, team_size, complexity}

### References

- [Source: docs/epics.md#Story-2-1]
- [Source: docs/PRD.md] (for requirements context)

## Dev Agent Record

### Context Reference

<!-- Path(s) to story context XML will be added here by context workflow -->

### Agent Model Used

<!-- To be filled by dev agent -->

### Debug Log References

<!-- To be filled by dev agent during implementation -->

### Completion Notes List

- Created domain-agnostic Pydantic model with flexible attributes dict
- Implemented cost_breakdown_template validator with 1% tolerance for floating point errors
- All 8 tests passing (valid data, invalid sums, tenant-specific, domain-specific attributes)
- MongoDB indexes created successfully for efficient querying
- Fixed MongoDB boolean check bug in get_database() function

### File List

**NEW:**
- `app/models/reference_class.py` - Pydantic models (ReferenceClass, CostDistribution, TimelineDistribution)
- `scripts/migrations/001_create_reference_class_indexes.py` - Index creation migration
- `tests/test_reference_class_model.py` - Comprehensive model validation tests

**MODIFIED:**
- `app/db/mongodb.py` - Fixed get_database() to use `is None` check instead of boolean check
