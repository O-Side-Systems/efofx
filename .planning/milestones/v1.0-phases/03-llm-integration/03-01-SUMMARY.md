---
phase: 03-llm-integration
plan: 01
subsystem: llm-service
tags: [byok, llm, error-handling, caching, dependency-injection]
requirements: [LLM-01, LLM-03, LLM-04]

dependency_graph:
  requires:
    - 02-04 (BYOK service — decrypt_tenant_openai_key)
    - 02-01 (security.py — get_current_tenant)
  provides:
    - get_llm_service FastAPI dependency (per-request BYOK injection)
    - classify_openai_error (error classification for 402/503/500)
    - LLMService with caching (content-hash cache key)
    - stream_chat_completion async generator
  affects:
    - 03-02 (structured estimation — uses get_llm_service)
    - 03-03 (chat service — ChatService now receives LLMService via DI)
    - 03-04 (streaming — uses stream_chat_completion)

tech_stack:
  added:
    - hashlib (SHA-256 for cache keys)
    - json (deterministic serialization for cache keys)
    - openai.AuthenticationError, openai.APIConnectionError (new imports)
  patterns:
    - FastAPI Depends() chain: get_current_tenant -> decrypt_tenant_openai_key -> LLMService
    - In-memory dict cache with SHA-256 content hash keys (upgrade to Valkey in multi-instance)
    - Module-level functions (classify_openai_error, _make_cache_key) for unit testability
    - Constructor injection: EstimationService(llm_service), ChatService(llm_service)

key_files:
  created: []
  modified:
    - apps/efofx-estimate/app/services/llm_service.py
    - apps/efofx-estimate/app/core/constants.py
    - apps/efofx-estimate/app/services/estimation_service.py
    - apps/efofx-estimate/app/services/chat_service.py
    - apps/efofx-estimate/app/api/routes.py
    - apps/efofx-estimate/tests/services/test_llm_service.py

decisions:
  - "LLMService.api_key is now required str with no default — settings.OPENAI_API_KEY fallback fully removed from production code paths"
  - "In-memory _response_cache dict is per-process; upgrade to Valkey for multi-instance/multi-worker deployments (tracked as known limitation)"
  - "classify_openai_error is module-level function (not method) to enable direct unit test import without LLMService instantiation"
  - "Cache key uses sort_keys=True JSON + SHA-256 — ensures deterministic hashing regardless of dict insertion order"
  - "use_cache=False parameter on generate_estimation enables forced refresh without cache invalidation complexity"

metrics:
  duration: 3 min
  completed_date: "2026-02-27"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 6
---

# Phase 3 Plan 01: LLM Service Hardening Summary

**One-liner:** BYOK-only LLMService with SHA-256 caching, OpenAI error classification to 402/503/500, and constructor injection replacing internal instantiation in EstimationService and ChatService.

## What Was Built

### Task 1: Harden LLMService

**`app/services/llm_service.py`** — complete rewrite of the module:

- `LLMService.__init__(self, api_key: str)` — `api_key` is now a required positional str. No default. No fallback to `settings.OPENAI_API_KEY`. Calling `LLMService()` raises `TypeError`.

- `classify_openai_error(exc: OpenAIError) -> tuple[str, int]` — module-level function mapping:
  - `AuthenticationError` → `("invalid_key", 402)`
  - `RateLimitError` with "insufficient_quota" → `("quota_exhausted", 402)`
  - `RateLimitError` (other) → `("transient", 503)`
  - `APITimeoutError`, `APIConnectionError` → `("transient", 503)`
  - Any other `OpenAIError` → `("unknown", 500)`

- `_make_cache_key(messages, model) -> str` — SHA-256 hash of JSON-serialized `{"messages": ..., "model": ...}` with `sort_keys=True`. Returns 64-char hex digest.

- `_response_cache: dict[str, str]` — module-level in-memory cache. Entries are `model_dump_json()` strings. Cache hits deserialize via `EstimationOutput.model_validate_json()`.

- `generate_estimation(...)` — updated with `use_cache: bool = True` parameter. Cache lookup precedes OpenAI call. Cache miss stores `result.model_dump_json()`.

- `stream_chat_completion(messages, temperature)` — async generator yielding content strings from streaming chat completions. Filters `None` chunks.

- `get_llm_service(tenant: Tenant = Depends(get_current_tenant)) -> LLMService` — FastAPI dependency. Calls `decrypt_tenant_openai_key(tenant.tenant_id)` and returns `LLMService(api_key=key)`. 402 propagates from byok_service if no key stored.

**`app/core/constants.py`** — added 4 message constants to `API_MESSAGES`:
- `BYOK_INVALID_KEY`: "Invalid OpenAI API key. Update your key in Settings."
- `BYOK_QUOTA_EXHAUSTED`: "OpenAI quota exhausted. Recharge your OpenAI account."
- `LLM_TRANSIENT_ERROR`: "We're having trouble generating a response. Please try again in a moment."
- `LLM_UNKNOWN_ERROR`: "An unexpected error occurred during AI processing."

**`tests/services/test_llm_service.py`** — rewrote with 16 tests:
- 2 constructor tests (requires api_key, creates AsyncOpenAI with correct key)
- 6 error classification tests (all OpenAI error types)
- 3 cache key tests (determinism, model variation, message variation)
- 2 caching behavior tests (cache hit, cache bypass)
- 3 get_llm_service tests (calls decrypt, returns correct LLMService, propagates 402)

All 16 tests pass.

### Task 2: Rewire Routes and Services

**`app/services/estimation_service.py`**:
- `EstimationService.__init__(self, llm_service: LLMService)` — `llm_service` is now a required parameter. `self.llm_service = LLMService()` removed.

**`app/services/chat_service.py`**:
- `ChatService.__init__(self, llm_service: LLMService)` — `llm_service` is now a required parameter. `self.llm_service = LLMService()` removed.

**`app/api/routes.py`**:
- Imported `LLMService, get_llm_service` from `app.services.llm_service`
- `get_estimation_service(llm_service: LLMService = Depends(get_llm_service)) -> EstimationService` — BYOK injection flows through automatically
- `get_chat_service(llm_service: LLMService = Depends(get_llm_service)) -> ChatService` — BYOK injection flows through automatically

All estimation and chat endpoints now enforce the BYOK 402 gate automatically via the dependency chain.

## Verification

- `python -m pytest tests/services/test_llm_service.py -x -v` — 16/16 passed
- `python -m pytest tests/ --ignore=tests/api -m "not integration"` — 125/125 passed (excluding pre-existing flaky performance test)
- `LLMService()` raises `TypeError: missing 1 required positional argument: 'api_key'` — confirmed
- API tests fail due to pre-existing Redis connection issues (unrelated to this plan)

## Deviations from Plan

None — plan executed exactly as written.

## Commits

| Hash | Message |
|------|---------|
| 174eda8 | feat(03-01): harden LLMService — BYOK-only, error classification, caching |
| 03c1195 | feat(03-01): rewire routes and services to use get_llm_service DI |

## Self-Check: PASSED

- FOUND: apps/efofx-estimate/app/services/llm_service.py
- FOUND: apps/efofx-estimate/tests/services/test_llm_service.py
- FOUND: .planning/phases/03-llm-integration/03-01-SUMMARY.md
- FOUND: commit 174eda8 (harden LLMService)
- FOUND: commit 03c1195 (rewire routes and services)
