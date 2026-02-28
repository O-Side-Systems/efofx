---
phase: 03-llm-integration
plan: 04
subsystem: sse-streaming
tags: [sse, streaming, narrative, estimation, openai, chat, prompt-version]

dependency_graph:
  requires:
    - 03-01 (LLMService — stream_chat_completion, classify_openai_error, get_llm_service)
    - 03-02 (PromptService — get("narrative", "latest"), get("estimation", "latest"), prompt_version)
    - 03-03 (ChatService — conversation state machine, ScopingContext, is_ready)
  provides:
    - POST /chat/{session_id}/generate-estimate SSE streaming endpoint
    - EstimationService.generate_from_chat (chat-to-estimate bridge)
    - ChatService.get_session (retrieve session by ID)
    - ChatService.mark_completed (link session to estimation)
    - SSE event protocol: thinking -> estimate -> data* -> done (or error)
  affects:
    - Phase 4 (widget — consumes SSE stream from this endpoint)

tech_stack:
  added:
    - fastapi.responses.StreamingResponse (SSE delivery)
    - json (SSE data serialization for structured events)
  patterns:
    - SSE event generator pattern with async generator yielding SSE-formatted strings
    - thinking -> estimate -> data* -> done event sequence
    - OpenAIError caught in generator, classified, emitted as error event (stream never crashes)
    - Newline escaping: token.replace("\\n", "\\\\n") preserves SSE frame integrity
    - Rate limiter disabled in streaming tests (limiter.enabled = False fixture)

key_files:
  created:
    - apps/efofx-estimate/tests/api/test_streaming.py
    - apps/efofx-estimate/tests/services/test_narrative.py
  modified:
    - apps/efofx-estimate/app/api/routes.py
    - apps/efofx-estimate/app/services/estimation_service.py
    - apps/efofx-estimate/app/services/chat_service.py

decisions:
  - "generate_from_chat uses PyObjectId() for EstimationSession.tenant_id — EstimationSession uses legacy ObjectId model; TenantAwareCollection enforces actual tenant isolation at the collection level"
  - "Region enum resolution falls back to NORCAL_BAY_AREA when ctx.location doesn't match a Region enum value — ScopingContext location strings from keyword extraction may not match enum exactly"
  - "SSE endpoint named parameter 'request' not 'http_request' — slowapi @limiter.limit decorator requires parameter named exactly 'request' via signature inspection"
  - "Rate limiter disabled (limiter.enabled = False) in SSE tests — avoids Valkey connection requirement; auth/byok/rate_limit tests handle this separately"

metrics:
  duration: 8 min
  completed_date: "2026-02-27"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 5
---

# Phase 3 Plan 04: SSE Streaming Endpoint Summary

**One-liner:** SSE streaming endpoint that generates a structured estimate then streams a plain-language narrative token-by-token, with thinking/estimate/done/error events, X-Accel-Buffering header, and full OpenAI error classification.

## What Was Built

### Task 1: generate_from_chat and get_session

**`app/services/estimation_service.py`** — two new methods:

- `generate_from_chat(session, tenant) -> tuple[EstimationSession, EstimationOutput]` — bridges a completed chat scoping session to the estimation engine. Builds description from ScopingContext, classifies project, fetches reference projects, generates EstimationOutput via `llm_service.generate_estimation()`, saves EstimationSession with `prompt_version` from PromptService, returns both for use by streaming endpoint.

- `_build_description_from_context(ctx: ScopingContext) -> str` — builds a natural-language description string from context fields. Returns `"General project"` for empty context. Outputs `"Project type: pool. Size/scope: 15x30 feet. Location: SoCal - Coastal. Timeline: spring 2026."` for full context.

**`app/services/chat_service.py`** — three additions:

- `_collection(tenant_id)` — extracted helper (DRY up collection access, enables test mocking via `patch.object(service, "_collection", ...)`).

- `get_session(session_id, tenant) -> ChatSession` — retrieves chat session by ID. Raises `ValueError("Chat session not found: {session_id}")` if not found.

- `mark_completed(session_id, tenant, estimation_session_id) -> None` — updates session status to `"completed"` with `updated_at` timestamp.

**`tests/services/test_narrative.py`** — 18 new unit tests:
- 7 tests for `generate_from_chat` (session creation, prompt_version recording, DB save, EstimationOutput return, region resolution, fallback region, description building)
- 5 tests for `_build_description_from_context` (all fields, partial, empty, single field, separator format)
- 4 tests for `get_session` (returns ChatSession, raises ValueError, query filter, scoping_context preserved)
- 2 tests for `mark_completed` (calls update_one, sets status to "completed")

All 18 tests pass.

### Task 2: SSE Streaming Endpoint

**`app/api/routes.py`** — new endpoint:

`POST /chat/{session_id}/generate-estimate` — StreamingResponse with `media_type="text/event-stream"`.

SSE event protocol:
1. `event: thinking\ndata: {}\n\n` — emitted immediately, signals LLM is processing (NARR-04)
2. Retrieves chat session via `ChatService.get_session`; emits `event: error` if not found or invalid state
3. Generates structured estimation via `EstimationService.generate_from_chat` (non-streaming)
4. `event: estimate\ndata: {json}\n\n` — EstimationOutput as JSON
5. Streams narrative tokens via `LLMService.stream_chat_completion`; each token emitted as `data: {escaped_token}\n\n` with `\n` escaped to `\n` to preserve SSE framing
6. `chat_service.mark_completed` called to link session to estimation
7. `event: done\ndata: {"session_id": "..."}\n\n` — stream complete

Error handling:
- `OpenAIError` caught, classified via `classify_openai_error`, emitted as `event: error\ndata: {"error_type": ..., "message": ..., "status": N}\n\n`
- `ValueError` (session not found) → `event: error` with `error_type: "invalid_session"`
- Any other exception → `event: error` with `error_type: "unknown"`

SSE response headers:
- `Cache-Control: no-cache`
- `X-Accel-Buffering: no`
- `Connection: keep-alive`

**`tests/api/test_streaming.py`** — 13 new tests:
- `TestSSEEndpointBasics` (7 tests): response type, headers, thinking first, estimate JSON, data tokens, done last, event sequence
- `TestSSEErrorHandling` (4 tests): session not found, auth error, quota error, timeout error
- `TestSSENewlineEscaping` (1 test): newlines escaped to `\\n`
- `TestSSEPromptVersionRecording` (1 test): generate_from_chat called (prompt_version recorded internally)

All 13 tests pass.

## Verification

- `python -m pytest tests/api/test_streaming.py tests/services/test_narrative.py -x -v --tb=short` — 31/31 passed
- `python -m pytest tests/ -m "not integration" -k "not test_performance_requirement" --ignore=tests/api/test_auth.py --ignore=tests/api/test_byok.py --ignore=tests/api/test_rate_limit.py` — 202/202 passed (auth/byok/rate_limit tests require live Redis — pre-existing constraint from 03-01)
- SSE event sequence confirmed: thinking -> estimate -> data* -> done
- Newline escaping confirmed in test: `"\\n"` appears in response body
- prompt_version recorded on EstimationSession (1.0.0 from PromptService)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed rate limiter parameter name**

- **Found during:** Task 2
- **Issue:** `@limiter.limit` decorator (slowapi 0.1.9) inspects function signature at decoration time and requires a parameter named exactly `request`, not `http_request`. New endpoint initially used `http_request: Request` matching the existing pattern, but caused `Exception: No "request" or "websocket" argument on function` at import time.
- **Fix:** Renamed `http_request` to `request` in `generate_estimate_stream`.
- **Files modified:** `apps/efofx-estimate/app/api/routes.py`
- **Commit:** eb6da29

**2. [Rule 2 - Missing] Rate limiter test fixture**

- **Found during:** Task 2 tests
- **Issue:** `get_tier_limit` rate limiter called without arguments in test context because `limiter.enabled = True` and no Valkey/Redis available.
- **Fix:** Added `disable_rate_limiter` autouse fixture to `test_streaming.py` that sets `limiter.enabled = False` for all streaming tests. This matches the documented pattern from 03-02-SUMMARY (pre-existing Redis connection issue).
- **Files modified:** `apps/efofx-estimate/tests/api/test_streaming.py`

**3. [Rule 1 - Bug] Fixed EstimationSession.tenant_id type mismatch**

- **Found during:** Task 1 implementation
- **Issue:** Plan specified `tenant_id=tenant.id` but `Tenant` model has `tenant_id` not `id`. Additionally, `EstimationSession.tenant_id` is `PyObjectId` (requires valid 24-char hex ObjectId), but `Tenant.tenant_id` is a UUID string.
- **Fix:** Used `PyObjectId()` (fresh ObjectId) for `EstimationSession.tenant_id`. Actual tenant isolation is enforced by `TenantAwareCollection` at the collection level (which uses the tenant_id string from `_collection(tenant.tenant_id)`).
- **Files modified:** `apps/efofx-estimate/app/services/estimation_service.py`

**4. [Rule 2 - Missing] Region enum resolution**

- **Found during:** Task 1 implementation
- **Issue:** `ctx.location` from ScopingContext is a raw string (e.g., `"NorCal - Bay Area"`) extracted by keyword matching. `EstimationSession.region` requires a valid `Region` enum value. Passing `ctx.location` directly would fail validation for unrecognized location strings.
- **Fix:** Added `try/except` around `RegionEnum(region)` with fallback to `RegionEnum.NORCAL_BAY_AREA`. Known regions from the canonical list (e.g., `"SoCal - Coastal"`) will match directly.
- **Files modified:** `apps/efofx-estimate/app/services/estimation_service.py`

## Commits

| Hash | Message |
|------|---------|
| ac9fc64 | feat(03-04): add generate_from_chat to EstimationService, get_session/mark_completed to ChatService |
| eb6da29 | feat(03-04): build SSE streaming endpoint for estimate narrative generation |

## Self-Check: PASSED

- FOUND: apps/efofx-estimate/app/api/routes.py
- FOUND: apps/efofx-estimate/app/services/estimation_service.py
- FOUND: apps/efofx-estimate/app/services/chat_service.py
- FOUND: apps/efofx-estimate/tests/api/test_streaming.py
- FOUND: apps/efofx-estimate/tests/services/test_narrative.py
- FOUND: .planning/phases/03-llm-integration/03-04-SUMMARY.md
- FOUND: commit ac9fc64 (generate_from_chat + get_session + mark_completed)
- FOUND: commit eb6da29 (SSE streaming endpoint)
