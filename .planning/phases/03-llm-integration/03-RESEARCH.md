# Phase 3: LLM Integration - Research

**Researched:** 2026-02-27
**Domain:** OpenAI v2 streaming, FastAPI SSE, MongoDB TTL sessions, prompt versioning, response caching
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Conversation flow**
- Structured guided intake — system leads with a logical question sequence, not freeform
- 3-5 follow-up questions: project type, size, location, timeline, special conditions
- Quick path to estimate: optimize for speed over exhaustive detail gathering
- Auto-trigger with confirmation — when system has enough detail, it says "I have enough to generate an estimate. Ready?" and user confirms before generation starts
- User can also explicitly request estimate generation at any point

**Estimate narrative style**
- Plain language, no jargon — "You'll most likely spend between $45k-$52k, though it could reach $65k if conditions change"
- Summary first, then breakdown — lead with bottom-line P50/P80 total range, then stream detailed cost categories and adjustment factors
- The narrative should explain the reasoning behind ranges in terms a homeowner understands
- Each cost category gets a plain-English explanation of what it covers and why the range exists

**Error experience**
- Transparent with retry — "We're having trouble generating a response. Retrying..." then "Please try again in a moment" if retry fails
- Show what's happening, don't hide failures behind vague messages
- Conversation is preserved across errors — user doesn't lose their progress
- Differentiate between transient failures (retry) and permanent failures (key exhausted → 402 with clear message to contractor)

**Prompt versioning**
- System-managed only — prompts are versioned JSON files in git, controlled by the platform team
- Contractors cannot edit or customize prompts
- Each estimate records which prompt_version was used for traceability
- Prompt versions are immutable once published — new version for any change

### Claude's Discretion
- Exact prompt content and structure (system prompts, follow-up generation prompts, narrative generation prompts)
- Cache key strategy for response deduplication
- SSE event format and reconnection behavior
- Chat session TTL duration
- Specific error retry timing and backoff strategy

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| LLM-01 | OpenAI client instantiated per-request with tenant's decrypted BYOK key | `decrypt_tenant_openai_key()` already exists in `byok_service.py`; LLMService already accepts `api_key` param; fallback to `settings.OPENAI_API_KEY` must be removed in production paths |
| LLM-02 | LLM responses streamed to client via Server-Sent Events (SSE) | `FastAPI StreamingResponse` with `media_type="text/event-stream"` + async generator yielding `data: ...\n\n` chunks; `client.chat.completions.create(stream=True)` with `async for chunk in stream` |
| LLM-03 | Graceful handling of OpenAI API failures (timeouts, rate limits, key exhaustion) | `AuthenticationError` (401 = invalid key), `RateLimitError` (429 = quota exhausted), `APITimeoutError`, `APIConnectionError` — all already importable from `openai`; error classification determines retry vs 402 |
| LLM-04 | LLM response caching by content hash for repeated identical queries | `hashlib.sha256` over `json.dumps(payload, sort_keys=True)` for deterministic cache key; in-memory `dict` or Valkey (already in stack) for cache storage |
| PRMT-01 | Prompts stored as versioned JSON files in git-tracked `config/prompts/` directory | `config/prompts/` directory already exists (README only); JSON files with schema `{version, name, created_at, system_prompt, user_prompt_template}` |
| PRMT-02 | Each estimate records which `prompt_version` was used for traceability | `EstimationSession` model needs `prompt_version: str` field; populate at generation time by reading `version` from loaded prompt JSON |
| PRMT-03 | Prompt versions are immutable once published (new version for changes) | File-per-version naming convention (`v1.0.0.json`, `v1.1.0.json`); loader raises error if existing version content changed |
| CHAT-01 | User can describe their project through multi-turn chat conversation | `chat_service.py` exists but has stub logic; needs full rewrite with structured question sequence and conversation state machine |
| CHAT-02 | Chat session persists conversation history within active session (MongoDB with TTL) | `chat_sessions` collection already has compound index; needs TTL index on `expires_at` field using Motor `create_index("expires_at", expireAfterSeconds=0)` |
| CHAT-03 | System determines when sufficient detail exists to generate estimate | LLM-driven readiness check OR rule-based check against collected fields (project_type, size, location, timeline present); LLM approach via structured output with `ReadinessCheck` Pydantic model |
| CHAT-04 | System asks targeted follow-up questions to gather missing project details | Prompt-driven: system prompt defines question sequence; LLM returns next question as structured output |
| CHAT-05 | Estimate generation triggered automatically or by user when ready | `is_ready` flag in `ChatResponse`; auto-trigger sends confirmation message; explicit `/estimate` trigger or user confirmation message |
| NARR-01 | LLM generates human-readable narrative explaining estimate ranges and assumptions | `client.chat.completions.create(stream=True)` with narrative-generation system prompt; stream tokens to SSE endpoint |
| NARR-02 | Narrative includes P50/P80 cost and timeline ranges with plain-language explanation | `EstimationOutput` model already has `total_cost_p50`, `total_cost_p80`, `timeline_weeks_p50`, `timeline_weeks_p80` — narrative prompt must reference these in plain language |
| NARR-03 | Narrative references specific cost breakdown categories and adjustment factors | `cost_breakdown: List[CostCategoryEstimate]` and `adjustment_factors: List[AdjustmentFactor]` already in `EstimationOutput` — pass these as context to the narrative prompt |
| NARR-04 | "Thinking" state indicator shown while LLM generates narrative | SSE `event: thinking` emitted before first token; client shows spinner until `data:` tokens begin arriving |
</phase_requirements>

---

## Summary

Phase 3 builds the backend intelligence layer on top of the fully established Phase 2 multi-tenant foundation. The project already has the structural scaffolding: `LLMService` accepts a BYOK `api_key` parameter (wired in Phase 2), `ChatService` exists with stub logic, `EstimationOutput` is a typed Pydantic model, and `config/prompts/` directory already exists. The main work is: removing the dev-only settings fallback in production LLM paths, replacing stub chat logic with a real conversation state machine, adding SSE streaming endpoints, building the prompt registry, and wiring caching.

The critical architectural discovery is that **`beta.chat.completions.parse()` does not support streaming**. For streaming narrative generation, use `client.chat.completions.create(stream=True)` with `async for chunk in stream: chunk.choices[0].delta.content`. For the final structured estimation output (non-streaming, called after chat scoping), continue using `beta.chat.completions.parse()` with `EstimationOutput`. This two-path approach — stream the narrative, structured-parse the estimate — aligns perfectly with the UX requirements.

The conversation state machine is the most novel engineering challenge. The system must track which of the 3-5 scoping questions have been answered, detect readiness, and avoid asking redundant questions. The cleanest approach is an LLM-driven state check via a lightweight `ScopingState` structured output call, or a simpler rule-based approach that checks which context fields are populated. Both patterns are validated; the rule-based approach is recommended for speed and cost.

**Primary recommendation:** Implement in four sequential plans exactly as the roadmap describes — BYOK client hardening → prompt registry → conversation engine → streaming SSE + narrative. Each plan has clear boundaries and can be tested in isolation.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `openai` | `>=2.20.0` (already installed) | AsyncOpenAI client, streaming, structured outputs | Project-mandated; `beta.chat.completions.parse()` established in Phase 1 |
| `fastapi` | `0.116.1` (already installed) | `StreamingResponse` for SSE endpoints | Already the web framework; native ASGI streaming support |
| `motor` | `3.3.2` (already installed) | Async MongoDB driver for chat session persistence | Project-mandated; TTL index already established for other collections |
| `pydantic` | `2.11.7` (already installed) | `EstimationOutput`, new `ScopingState` models | Project-mandated; used for all structured OpenAI outputs |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `hashlib` | stdlib | SHA-256 cache key generation | LLM-04: content-hash cache keys |
| `json` | stdlib | `json.dumps(sort_keys=True)` for deterministic serialization | Cache key stability across dict ordering variations |
| `valkey` | `>=6.1.0` (already installed) | Optional distributed cache backend | If in-memory cache proves insufficient; Valkey already in stack for rate limiting |
| `asyncio` | stdlib | `asyncio.CancelledError` handling for client disconnect | SSE endpoint cleanup |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| In-memory dict cache | Valkey/Redis | In-memory is simpler and sufficient for single-instance dev; Valkey is better for multi-instance production — use in-memory first, Valkey as upgrade path |
| Rule-based readiness check | LLM structured output readiness | Rule-based is cheaper, faster, more predictable; LLM-based is more nuanced but burns tokens on meta-decisions — use rule-based |
| `beta.chat.completions.stream()` for narrative | `chat.completions.create(stream=True)` | `stream()` is designed for streaming structured outputs; `create(stream=True)` is simpler for free-form narrative text — use `create(stream=True)` for narrative streaming |

**Installation:** No new packages needed — all required libraries are already in `requirements.txt` / `pyproject.toml`.

---

## Architecture Patterns

### Recommended Project Structure Changes

```
apps/efofx-estimate/
├── app/
│   ├── api/
│   │   └── routes.py              # ADD: POST /chat/stream (SSE), POST /chat/estimate-ready
│   ├── services/
│   │   ├── llm_service.py         # MODIFY: remove settings fallback in production paths
│   │   ├── chat_service.py        # REWRITE: conversation state machine
│   │   └── prompt_service.py      # NEW: prompt registry loader + version resolution
│   └── models/
│       ├── estimation.py          # MODIFY: add prompt_version field to EstimationSession
│       └── chat.py                # MODIFY: add is_ready, readiness_context fields to ChatResponse
└── config/
    └── prompts/
        ├── v1.0.0-scoping.json    # NEW: scoping/follow-up question prompt
        ├── v1.0.0-narrative.json  # NEW: narrative generation prompt
        └── v1.0.0-estimation.json # NEW: structured estimation prompt
```

### Pattern 1: Per-Request BYOK Client Instantiation (LLM-01)

**What:** Every LLM call decrypts the tenant's key at request time and creates a new `AsyncOpenAI` instance. No sharing client across requests.

**When to use:** Any code path that calls OpenAI on behalf of a tenant.

```python
# Source: byok_service.py (already implemented), llm_service.py (already accepts api_key)
# Pattern established in Phase 2 — Phase 3 removes the dev fallback from production paths.

async def get_llm_service(tenant: Tenant = Depends(get_current_tenant)) -> LLMService:
    """FastAPI dependency: decrypt BYOK key and return scoped LLMService.
    Raises HTTP 402 if tenant has no stored OpenAI key.
    """
    api_key = await decrypt_tenant_openai_key(tenant.id)  # raises 402 if missing
    return LLMService(api_key=api_key)
```

### Pattern 2: SSE Streaming Endpoint (LLM-02, NARR-01, NARR-04)

**What:** FastAPI `StreamingResponse` with `media_type="text/event-stream"` wrapping an async generator that iterates OpenAI stream chunks.

**When to use:** Any endpoint that streams token-by-token LLM output.

```python
# Source: verified from fastapi StreamingResponse docs + openai AsyncOpenAI streaming docs
from fastapi import Request
from fastapi.responses import StreamingResponse

@api_router.post("/chat/{session_id}/stream")
async def stream_narrative(
    request: Request,
    session_id: str,
    tenant: Tenant = Depends(get_current_tenant),
    llm_service: LLMService = Depends(get_llm_service),
):
    async def event_generator():
        yield "event: thinking\ndata: [START]\n\n"  # NARR-04: thinking state
        try:
            stream = await llm_service.client.chat.completions.create(
                model="gpt-4o-mini",
                messages=[...],  # narrative prompt + context
                stream=True,
            )
            async for chunk in stream:
                content = chunk.choices[0].delta.content
                if content is not None:
                    # Escape newlines inside data field to preserve SSE framing
                    escaped = content.replace("\n", "\\n")
                    yield f"data: {escaped}\n\n"
        except RateLimitError:
            yield "event: error\ndata: quota_exhausted\n\n"
        except (APITimeoutError, APIConnectionError):
            yield "event: error\ndata: transient\n\n"
        finally:
            yield "event: end\ndata: [END]\n\n"

    return StreamingResponse(
        event_generator(),
        media_type="text/event-stream",
        headers={
            "Cache-Control": "no-cache",
            "X-Accel-Buffering": "no",   # disables nginx buffering
            "Connection": "keep-alive",
        },
    )
```

**Critical note:** `delta.content` can be `None` on the first and last chunks — always guard with `if content is not None`.

### Pattern 3: Prompt Registry (PRMT-01, PRMT-02, PRMT-03)

**What:** JSON files in `config/prompts/` named by version. A `PromptService` loads them at startup and serves them by name + version. Immutability enforced by checking file content hash on load.

**When to use:** Any LLM call that needs a versioned, trackable prompt.

```python
# Source: established pattern from prompt versioning best practices research
# config/prompts/v1.0.0-narrative.json
{
  "version": "1.0.0",
  "name": "narrative",
  "created_at": "2026-02-27",
  "system_prompt": "You are a plain-language construction estimator...",
  "user_prompt_template": "Project: {project_context}\nEstimate data: {estimate_data}\n..."
}

# app/services/prompt_service.py
class PromptService:
    _registry: dict[str, dict] = {}  # loaded once at startup

    @classmethod
    def load_all(cls, prompts_dir: str) -> None:
        """Load all prompt JSON files. Call from app lifespan."""
        for path in Path(prompts_dir).glob("*.json"):
            data = json.loads(path.read_text())
            key = f"{data['name']}:{data['version']}"
            cls._registry[key] = data

    @classmethod
    def get(cls, name: str, version: str = "latest") -> dict:
        """Return prompt dict. 'latest' resolves to highest semver."""
        ...
```

Populate `EstimationSession.prompt_version` at estimate generation time: `session.prompt_version = prompt["version"]`.

### Pattern 4: Response Caching by Content Hash (LLM-04)

**What:** SHA-256 hash of `json.dumps({"messages": messages, "model": model}, sort_keys=True)` as cache key. Store full response text in memory dict or Valkey.

**When to use:** Before every `generate_estimation()` call. Cache hit = skip OpenAI call entirely.

```python
# Source: hashlib stdlib + verified pattern from LLM caching research
import hashlib
import json
from typing import Optional

_cache: dict[str, str] = {}  # in-memory; upgrade to Valkey if multi-instance

def _cache_key(messages: list, model: str) -> str:
    payload = json.dumps({"messages": messages, "model": model}, sort_keys=True)
    return hashlib.sha256(payload.encode()).hexdigest()

async def generate_with_cache(llm_service: LLMService, messages: list) -> str:
    key = _cache_key(messages, llm_service.model)
    if key in _cache:
        return _cache[key]
    result = await llm_service.generate_estimation(...)
    _cache[key] = result
    return result
```

### Pattern 5: Conversation State Machine (CHAT-01 through CHAT-05)

**What:** `ChatService` tracks a `ScopingContext` (project_type, size, location, timeline, special_conditions). Each turn: (1) extract any new context from user message, (2) determine next question or trigger readiness, (3) save updated context to MongoDB chat session.

**When to use:** Every chat message in the scoping flow.

```python
# Rule-based readiness detection (recommended over LLM-based for cost/speed)
REQUIRED_FIELDS = {"project_type", "location"}
SUFFICIENT_FIELDS = {"project_type", "size", "location", "timeline"}

def _is_ready(context: dict) -> bool:
    """True when enough detail exists for a quality estimate."""
    populated = {k for k, v in context.items() if v}
    return SUFFICIENT_FIELDS.issubset(populated)
```

Chat session stored in `chat_sessions` collection with `expires_at` TTL. MongoDB TTL index (`expireAfterSeconds=0` on `expires_at` field) auto-purges expired sessions. Session format stores full message history as embedded array for conversation continuity.

### Pattern 6: MongoDB TTL for Chat Sessions (CHAT-02)

**What:** `expires_at` datetime field on chat session documents. TTL index with `expireAfterSeconds=0` tells MongoDB to delete when `now > expires_at`.

**When to use:** Chat session creation — set `expires_at = utcnow() + timedelta(hours=TTL_HOURS)`.

```python
# Source: MongoDB TTL docs + existing pattern in refresh_tokens collection
# Already used in the codebase for verification_tokens and refresh_tokens.
# Add to create_indexes() in mongodb.py:
await db["chat_sessions"].create_index("expires_at", expireAfterSeconds=0)
```

Note: TTL cleanup runs every ~60 seconds, not instant. This is acceptable — sessions remain readable until purged.

### Anti-Patterns to Avoid

- **Using `beta.chat.completions.parse()` for streaming:** `parse()` blocks until completion — it does not stream. Use `client.chat.completions.create(stream=True)` for narrative streaming and reserve `parse()` for structured estimation output (non-streaming).
- **Sharing `LLMService` instance across requests:** Phase 2 already established per-request instantiation. The current `ChatService.__init__` creates `LLMService()` at construction time — this must be changed to accept the BYOK-keyed service as a dependency, not create its own.
- **Storing plaintext BYOK key:** `decrypt_tenant_openai_key()` contract says use within request scope and discard. Never persist to session context.
- **Falling back to `settings.OPENAI_API_KEY` in production:** The Phase 2 comment explicitly calls this out. Phase 3 plan 03-01 removes this fallback from all production code paths. The fallback may remain for test isolation only.
- **Prompts embedded as Python strings in code:** All prompts must be JSON files in `config/prompts/`. Inline strings cannot be versioned or traced back to a `prompt_version`.
- **Using `session.dict()` instead of `session.model_dump()`:** Pydantic v2 uses `model_dump()` — `dict()` still works but emits deprecation warnings. Existing code uses `dict(by_alias=True)` in places; Phase 3 additions should use `model_dump(by_alias=True)`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Streaming LLM responses | Custom chunked HTTP response | `FastAPI StreamingResponse` + OpenAI `stream=True` | Handles ASGI chunking, backpressure, and keep-alive correctly |
| JSON schema for structured output | Manual JSON schema dict | `beta.chat.completions.parse(response_format=EstimationOutput)` | Schema derived automatically from Pydantic model; guaranteed parse |
| Content-addressed cache key | Custom hashing function | `hashlib.sha256` + `json.dumps(sort_keys=True)` | Deterministic ordering, stdlib, zero dependencies |
| Document expiry | Cron job to delete expired sessions | MongoDB TTL index (`expireAfterSeconds=0`) | Already used in project for tokens; atomic, no application code |
| BYOK key decryption | Custom decryption in routes | `decrypt_tenant_openai_key()` from `byok_service.py` | Already implemented, tested, raises 402 correctly |

**Key insight:** Almost every non-trivial piece of infrastructure for this phase is already present — the Phase 3 work is wiring, not building from scratch.

---

## Common Pitfalls

### Pitfall 1: `delta.content` is `None` on First and Last Streaming Chunks

**What goes wrong:** Code does `content = chunk.choices[0].delta.content; yield f"data: {content}\n\n"` — the first chunk (role token) and last chunk (finish_reason) have `None` content, causing `data: None\n\n` to be sent to the client.

**Why it happens:** OpenAI streaming sends a partial structure on each chunk; the first chunk often carries only `role: "assistant"` and the last carries only `finish_reason`.

**How to avoid:** Always guard: `if content is not None: yield f"data: {content}\n\n"`

**Warning signs:** Client receives literal `"None"` text interspersed with real content.

### Pitfall 2: SSE Newlines Inside Data Payload Break Event Frame

**What goes wrong:** If narrative content contains `\n`, the raw newline is sent in the `data:` field, creating a multi-line data field that breaks some SSE parsers.

**Why it happens:** SSE spec uses `\n\n` as the event terminator. A raw `\n` inside `data:` is interpreted as a field separator.

**How to avoid:** Escape newlines in content before yielding: `content.replace("\n", "\\n")`. The client unescapes on receipt.

**Warning signs:** SSE client fails to parse some events; event boundaries appear incorrect.

### Pitfall 3: Nginx Buffering Defeats Streaming

**What goes wrong:** SSE events arrive at the client in large batches, not token-by-token, despite correct server implementation.

**Why it happens:** Nginx (and other reverse proxies) buffer response bodies by default before forwarding.

**How to avoid:** Add `X-Accel-Buffering: no` header to the SSE response. In Nginx config: `proxy_buffering off;` for the relevant location.

**Warning signs:** Streaming works locally but batches in staging/production.

### Pitfall 4: `ChatService` Instantiates `LLMService` Without BYOK Key

**What goes wrong:** `ChatService.__init__(self): self.llm_service = LLMService()` creates an LLM client that falls back to `settings.OPENAI_API_KEY` — bypassing BYOK gate.

**Why it happens:** Existing `ChatService` was written as a stub. Phase 3 must inject the BYOK-keyed `LLMService` as a dependency rather than creating it internally.

**How to avoid:** Refactor `ChatService` constructor to accept `api_key: str` or `llm_service: LLMService`. Create `ChatService` via FastAPI dependency that calls `get_llm_service()` first.

**Warning signs:** LLM calls succeed even when tenant has no stored BYOK key.

### Pitfall 5: Caching Streaming Responses

**What goes wrong:** Attempting to cache an async generator — the generator is exhausted on first read and subsequent cache hits return empty.

**Why it happens:** Generators are stateful one-shot iterators.

**How to avoid:** Cache only applies to non-streaming paths (structured estimation output). For streaming narrative, do not cache the generator — cache the fully-assembled narrative string post-generation if desired, but do not cache the SSE stream itself.

**Warning signs:** Cache hit for a narrative returns no tokens streamed.

### Pitfall 6: `prompt_version` Missing from Estimate Record

**What goes wrong:** `EstimationSession` document is saved to MongoDB without `prompt_version` populated — traceability broken.

**Why it happens:** Developer forgets to propagate the loaded prompt's `version` field to the session before saving.

**How to avoid:** Add `prompt_version: str` to `EstimationSession` model. Make it required (no Optional). Load prompt first, then create session, then save. The model enforces presence.

**Warning signs:** `prompt_version` is `null` or absent in MongoDB documents.

### Pitfall 7: Authentication vs Quota Errors Look Similar

**What goes wrong:** Both `AuthenticationError` (401, invalid key) and `RateLimitError` (429, quota exhausted) result in a failed LLM call, but they require different responses: invalid key → `402 Payment Required` with "update your key" message; quota exhausted → also `402` but "your quota is exhausted, recharge your account".

**Why it happens:** Treating all LLM errors as generic failures.

**How to avoid:** Catch specifically:
```python
except AuthenticationError:
    raise HTTPException(402, "Invalid OpenAI API key. Update your key in Settings.")
except RateLimitError as e:
    if "insufficient_quota" in str(e):
        raise HTTPException(402, "OpenAI quota exhausted. Recharge your account.")
    # Otherwise it is a per-minute rate limit — retry after backoff
    raise  # let retry logic handle it
```

---

## Code Examples

Verified patterns from official sources and codebase inspection:

### Streaming Chat Completion (LLM-02, NARR-01)
```python
# Source: github.com/openai/openai-python README + verified async streaming pattern
async def stream_narrative_tokens(client: AsyncOpenAI, messages: list, model: str):
    """Async generator yielding SSE-formatted token strings."""
    stream = await client.chat.completions.create(
        model=model,
        messages=messages,
        stream=True,
    )
    async for chunk in stream:
        content = chunk.choices[0].delta.content
        if content is not None:
            yield f"data: {content.replace(chr(10), chr(92) + 'n')}\n\n"
```

### Structured Estimation Output (remains non-streaming)
```python
# Source: existing llm_service.py — established in Phase 1, keep as-is
completion = await client.beta.chat.completions.parse(
    model="gpt-4o-mini",
    messages=[...],
    response_format=EstimationOutput,
)
result = completion.choices[0].message.parsed  # fully typed EstimationOutput
```

### Error Classification (LLM-03)
```python
# Source: openai Python SDK error hierarchy (verified via search + community docs)
from openai import (
    AuthenticationError,    # 401 - invalid/expired key
    RateLimitError,         # 429 - quota exhausted or per-minute rate limit
    APITimeoutError,        # timeout - transient, retry
    APIConnectionError,     # network - transient, retry
    OpenAIError,            # base class - catch-all
)

def classify_openai_error(exc: OpenAIError) -> tuple[str, int]:
    """Returns (error_type, http_status) for routing to correct user message."""
    if isinstance(exc, AuthenticationError):
        return ("invalid_key", 402)
    if isinstance(exc, RateLimitError) and "insufficient_quota" in str(exc):
        return ("quota_exhausted", 402)
    if isinstance(exc, (RateLimitError, APITimeoutError, APIConnectionError)):
        return ("transient", 503)  # retry
    return ("unknown", 500)
```

### Chat Session with TTL
```python
# Source: existing refresh_tokens TTL pattern in mongodb.py
session = {
    "session_id": session_id,
    "tenant_id": tenant_id,
    "messages": [],
    "context": {},
    "created_at": datetime.utcnow(),
    "expires_at": datetime.utcnow() + timedelta(hours=24),  # TTL_HOURS = Claude's discretion
}
# Index already exists: ("tenant_id", "session_id", unique=True)
# Add TTL index: await db["chat_sessions"].create_index("expires_at", expireAfterSeconds=0)
```

### Prompt Registry JSON Schema
```json
{
  "version": "1.0.0",
  "name": "narrative",
  "created_at": "2026-02-27",
  "description": "Generates plain-language estimate narrative for homeowners",
  "system_prompt": "You are a knowledgeable contractor explaining a project estimate in plain language...",
  "user_prompt_template": "Project: {project_type} in {location}...\nEstimate: P50={p50}, P80={p80}..."
}
```

### Content Hash Cache Key
```python
# Source: hashlib stdlib docs + verified pattern
import hashlib
import json

def make_cache_key(messages: list[dict], model: str) -> str:
    payload = {"messages": messages, "model": model}
    serialized = json.dumps(payload, sort_keys=True, ensure_ascii=True)
    return hashlib.sha256(serialized.encode("utf-8")).hexdigest()
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `openai.ChatCompletion.create()` (v0) | `AsyncOpenAI().chat.completions.create()` (v2) | v1.0.0 (2023) | Already adopted in Phase 1 |
| Free-form LLM response + custom parser | `beta.chat.completions.parse(response_format=PydanticModel)` | 2024 | Already adopted in Phase 1; never parse free-form LLM text |
| `stream=True` with `parse()` | Cannot combine — use `create(stream=True)` for narrative, `parse()` for structured output | Always true | Two separate patterns: streaming narrative vs structured estimate |
| `session.dict()` (Pydantic v1) | `session.model_dump()` (Pydantic v2) | Pydantic 2.x | Existing code uses `dict()`; Phase 3 should use `model_dump()` |

**Deprecated/outdated:**
- `LLMService()` without `api_key` in production: Phase 2 documented this as temporary. Phase 3 plan 03-01 removes the `settings.OPENAI_API_KEY` fallback from all production code paths. It may be retained in test fixtures only with explicit documentation.
- `ChatService.__init__` creating its own `LLMService()`: Must change to dependency injection of a pre-keyed `LLMService`.

---

## Open Questions

1. **Chat session TTL duration**
   - What we know: Session TTL is in Claude's discretion; the project uses 30-minute SESSION_TIMEOUT_MINUTES for estimation sessions
   - What's unclear: Whether chat sessions should expire faster (user leaves browser), slower (user returns next day), or align with estimation session TTL
   - Recommendation: Use 24 hours as default (generous for UX); make it configurable via settings

2. **Cache storage: in-memory dict vs Valkey**
   - What we know: Valkey is already installed for rate limiting; in-memory cache does not survive server restarts or multi-instance deployments
   - What's unclear: Whether the deployment is single-instance or multi-instance
   - Recommendation: Use in-memory dict for Phase 3 (simple, no new wiring needed); add Valkey upgrade path as a comment in the cache implementation

3. **SSE reconnection on transient error**
   - What we know: SSE has built-in reconnection via `retry:` directive; browsers reconnect automatically after disconnect
   - What's unclear: Whether the Phase 4 widget will rely on browser SSE `EventSource` (which auto-reconnects) or fetch + ReadableStream (which does not)
   - Recommendation: Include `retry: 3000\n` in SSE headers to hint reconnection interval; document for Phase 4

4. **Structured estimation output still non-streaming?**
   - What we know: The current flow calls `generate_estimation()` which uses `beta.chat.completions.parse()` — blocking, not streamed
   - What's unclear: Whether the estimate data itself needs to stream or only the narrative
   - Recommendation: Keep estimate generation non-streaming (it's a structured object, not prose). Only the narrative text is streamed. The flow is: (1) generate structured EstimationOutput via parse() → (2) stream narrative referencing those numbers via create(stream=True).

---

## Sources

### Primary (HIGH confidence)
- `apps/efofx-estimate/app/services/llm_service.py` — current BYOK pattern, `generate_estimation()` implementation
- `apps/efofx-estimate/app/services/byok_service.py` — `decrypt_tenant_openai_key()`, error handling, 402 gate
- `apps/efofx-estimate/app/db/mongodb.py` — TTL index patterns, `TenantAwareCollection` usage
- `apps/efofx-estimate/app/models/estimation.py` — `EstimationOutput` Pydantic model, `EstimationSession`
- `apps/efofx-estimate/app/services/chat_service.py` — current stub state (full rewrite needed)
- github.com/openai/openai-python README — async streaming pattern: `async for chunk in stream: chunk.choices[0].delta.content`
- github.com/openai/openai-python helpers.md — `beta.chat.completions.stream()` vs `parse()` distinction

### Secondary (MEDIUM confidence)
- community.openai.com/t/is-is-possible-to-stream-structured-output-with-pydantic/1085193 — confirms `parse()` does not support streaming; use `stream()` or `create(stream=True)`
- sevalla.com/blog/real-time-openai-streaming-fastapi/ — FastAPI SSE generator pattern with `event: start/end` markers and `X-Accel-Buffering: no`
- MongoDB docs (mongodb.com/docs/manual/tutorial/expire-data/) — TTL index with `expireAfterSeconds=0` pattern
- jasoncameron.dev/posts/fastapi-cancel-on-disconnect — `cancel_on_disconnect` pattern for clean generator teardown
- platform.openai.com/docs/guides/error-codes — `AuthenticationError` (401), `RateLimitError` (429), error code taxonomy

### Tertiary (LOW confidence)
- python.useinstructor.com/blog/2023/11/26/python-caching-llm-optimization/ — SHA-256 cache key pattern (pattern is sound; article predates SDK v2)

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already installed and in use; no new dependencies
- Architecture: HIGH — patterns derived from existing codebase + official OpenAI docs
- Pitfalls: HIGH for items verified from codebase inspection (BYOK fallback, parse() vs stream()); MEDIUM for SSE-specific pitfalls (verified via community sources)

**Research date:** 2026-02-27
**Valid until:** 2026-03-29 (30 days — OpenAI SDK v2 is stable; FastAPI 0.116.x is stable)
