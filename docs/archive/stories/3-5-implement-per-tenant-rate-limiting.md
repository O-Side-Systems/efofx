# Story 3.5: Implement Per-Tenant Rate Limiting

Status: backlog

## Story

As a system,
I want to enforce rate limits based on tenant tier,
So that free tiers don't consume excessive resources and paid tiers get higher quotas.

## Acceptance Criteria

**Given** FR-3.4 specifies tier-based rate limits
**When** I implement rate limiting
**Then** `app/middleware/rate_limiting.py` uses slowapi:
- Trial: 10 requests/hour
- Pro: 100 requests/hour
- Enterprise: 1000 requests/hour

**And** rate limit key function extracts tenant_id from JWT

**And** endpoints are decorated with dynamic limits

**And** when rate limit exceeded
**Then** API returns 429 Too Many Requests with:
- Current limit and window
- Retry-After header (seconds until reset)
- Upgrade URL for higher tier

**And** rate limit headers included in all responses:
- X-RateLimit-Limit
- X-RateLimit-Remaining
- X-RateLimit-Reset

## Tasks / Subtasks

- [ ] Implement rate limiting middleware using slowapi
- [ ] Configure tier-based rate limits (trial/pro/enterprise)
- [ ] Extract tenant_id from JWT for rate limit key
- [ ] Decorate endpoints with dynamic rate limits
- [ ] Return 429 error with helpful information
- [ ] Add rate limit headers to all responses
- [ ] Test rate limiting for all tiers
- [ ] Test rate limit reset behavior

## Dev Notes

### Prerequisites

Story 3.1 (tenant tier field exists)

### Technical Notes

- Use Redis for rate limit counters (optional for MVP, in-memory acceptable)
- Rate limit check adds <5ms latency per request
- Global rate limit: 1000 concurrent requests across platform
- Widget sessions: 50 concurrent per tenant (separate limit)

### References

- [Source: docs/epics.md#Story-3-5]
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
