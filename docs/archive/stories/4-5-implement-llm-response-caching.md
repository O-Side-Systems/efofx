# Story 4.5: Implement LLM Response Caching

Status: backlog

## Story

As a system,
I want to cache identical LLM prompts to avoid redundant API calls,
So that costs are reduced and response times are faster for common queries.

## Acceptance Criteria

**Given** many users request estimates for similar projects
**When** I implement LLM caching
**Then** `app/utils/cache.py` contains:
- `LLMCache` class using in-memory LRU cache
- Cache key = hash(prompt + model + temperature)
- Cache TTL = 1 hour
- Cache hit → return cached response immediately
- Cache miss → call OpenAI, store response

**And** when cache hit occurs
**Then** response time < 50ms (vs 2-3s for OpenAI call)

**And** when cache is full (max 1000 entries)
**Then** LRU eviction removes oldest unused entry

**And** cache metrics logged (hit rate, hits/misses, avg response time)

**And** cache bypass available via request header: `X-Bypass-Cache: true`

## Tasks / Subtasks

- [ ] Create `app/utils/cache.py`
- [ ] Implement LLMCache class with LRU
- [ ] Implement cache key generation (hash)
- [ ] Set TTL to 1 hour
- [ ] Implement cache hit/miss logic
- [ ] Implement LRU eviction (max 1000 entries)
- [ ] Add cache metrics logging
- [ ] Add cache bypass header support
- [ ] Test cache performance

## Dev Notes

### Prerequisites

Story 4.3 (narrative generation works)

### Technical Notes

- Use Python functools.lru_cache or Redis (if available)
- Cache only successful LLM responses (not errors)
- Consider cache warm-up for popular project types
- Monitor hit rate - should be >30% for cost savings

### References

- [Source: docs/epics.md#Story-4-5]
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
