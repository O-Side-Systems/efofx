# Story 2.2: Implement RCF Matching Algorithm

Status: done

## Story

As a system,
I want to match user project descriptions to best reference class using keyword matching and confidence scoring,
So that estimates are based on the most relevant historical data.

## Acceptance Criteria

**Given** a user provides project description, category, and region
**When** the RCF matching algorithm runs
**Then** `app/services/rcf_engine.py` contains:
- `find_matching_reference_class(description, category, region, tenant_id)` function
- Keyword extraction from description (lowercase, tokenize)
- Scoring logic: keyword overlap + category exact match + region match
- Confidence score calculation (0.0 to 1.0)
- Returns top match with confidence >= 0.7, else None

**And** when confidence < 0.7
**Then** the system returns error suggesting more details needed

**And** when multiple matches have same score
**Then** prefer tenant-specific over platform-provided

**And** the matching algorithm completes in < 50ms (p95)

## Tasks / Subtasks

- [x] Create `app/services/rcf_engine.py`
- [x] Implement keyword extraction function
- [x] Implement scoring logic (keyword overlap + category + region)
- [x] Implement confidence score calculation
- [x] Add tenant-specific preference when scores tied
- [x] Handle case when confidence < 0.7
- [x] Add performance logging
- [x] Test with various project descriptions
- [x] Test performance meets <50ms requirement

## Dev Notes

### Prerequisites

Story 2.1 (schema created)

### Technical Notes

- Use simple keyword overlap for MVP (TF-IDF or ML models post-MVP)
- Scoring formula: (keyword_matches / total_keywords) * 0.6 + category_match * 0.3 + region_match * 0.1
- Cache matching results for identical queries (5 min TTL)
- Log all match attempts with confidence scores for analysis

### References

- [Source: docs/epics.md#Story-2-2]
- [Source: docs/PRD.md] (for requirements context)

## Dev Agent Record

### Context Reference

<!-- Path(s) to story context XML will be added here by context workflow -->

### Agent Model Used

claude-sonnet-4-5-20250929

### Debug Log References

Implementation completed in single session. All acceptance criteria met:
- ✓ Created rcf_engine.py with find_matching_reference_class function
- ✓ Implemented keyword extraction with stop word filtering
- ✓ Scoring formula: keyword_overlap * 0.6 + category_match * 0.3 + region_match * 0.1
- ✓ Confidence threshold of 0.7 with ValueError for low confidence
- ✓ Tenant-specific preference when scores are equal
- ✓ In-memory caching with 5-minute TTL
- ✓ Performance logging with elapsed time tracking
- ✓ Comprehensive test suite with 22 unit tests passing

### Completion Notes List

**Implementation Summary:**
Created domain-agnostic RCF matching engine in `app/services/rcf_engine.py` with the following features:

1. **Keyword Extraction**: Tokenizes and normalizes project descriptions, filters stop words and short words
2. **Scoring Algorithm**: Implements weighted scoring per requirements:
   - Keyword overlap: 60%
   - Category exact match: 30%
   - Region match: 10%
3. **Confidence Thresholding**: Returns matches with confidence >= 0.7, raises ValueError with helpful message otherwise
4. **Tenant Preference**: Sorts by (confidence, is_tenant_specific) to prefer tenant data when scores are equal
5. **Performance**: Includes timing logs and in-memory cache (5min TTL) for query optimization
6. **Error Handling**: Comprehensive error messages for empty descriptions, missing categories, low confidence

**Test Coverage:**
- 22 unit tests passing covering all core functionality
- Tests for keyword extraction, scoring, confidence calculation, region matching, caching
- Performance validation built into test suite

**Files Modified:**
- apps/efofx-estimate/app/services/rcf_engine.py (NEW)
- apps/efofx-estimate/tests/services/test_rcf_engine.py (NEW)

All acceptance criteria satisfied and ready for code review.

### File List

**NEW:**
- apps/efofx-estimate/app/services/rcf_engine.py
- apps/efofx-estimate/tests/services/test_rcf_engine.py

**MODIFIED:**
- None

**DELETED:**
- None
