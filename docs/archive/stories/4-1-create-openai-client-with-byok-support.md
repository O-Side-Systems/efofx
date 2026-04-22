# Story 4.1: Create OpenAI Client with BYOK Support

Status: backlog

## Story

As a developer,
I want an OpenAI client that uses tenant-specific API keys from BYOK storage,
So that LLM requests are billed to the tenant and respect their quota.

## Acceptance Criteria

**Given** the architecture specifies OpenAI BYOK with retry logic
**When** I implement the LLM service
**Then** `app/services/llm_service.py` contains:
- `LLMService` class with async OpenAI client
- `get_openai_client(tenant_id)` that decrypts tenant's key
- Fallback to platform key for Trial tier tenants
- Retry decorator with exponential backoff (max 3 retries)
- Timeout: 30 seconds per request

**And** when making LLM requests
**Then** the service includes:
- Tenant_id in request metadata (for tracking)
- Model selection from config (gpt-4o-mini for MVP)
- Structured output mode support (JSON)
- Error handling for rate limits (429), quota exceeded (402), invalid key (401)

**And** when tenant's OpenAI key fails
**Then** helpful error returned: "Your OpenAI API key quota exceeded. Please add credits or upgrade."

**And** LLM requests log: tenant_id, model, prompt_version, tokens used, latency

## Tasks / Subtasks

- [ ] Create `app/services/llm_service.py` with LLMService class
- [ ] Implement `get_openai_client()` with BYOK decryption
- [ ] Add fallback to platform key for Trial tier
- [ ] Implement retry logic with exponential backoff
- [ ] Add 30-second timeout for LLM requests
- [ ] Add error handling for all OpenAI error types
- [ ] Implement request logging
- [ ] Test with tenant BYOK key
- [ ] Test with platform key fallback
- [ ] Test error handling

## Dev Notes

### Prerequisites

Story 3.3 (BYOK encryption implemented)

### Technical Notes

- Use openai>=1.0.0 (latest SDK)
- Cache LLM responses for identical prompts (1 hour TTL)
- Monitor OpenAI API status and circuit break if down
- Consider streaming responses for long narratives (post-MVP)

### References

- [Source: docs/epics.md#Story-4-1]
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
