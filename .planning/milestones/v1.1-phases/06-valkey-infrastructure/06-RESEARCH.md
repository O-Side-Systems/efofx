# Phase 6: Valkey Infrastructure - Research

**Researched:** 2026-03-01
**Domain:** Distributed caching with Valkey (valkey-py), DigitalOcean Managed Valkey provisioning, graceful fallback
**Confidence:** HIGH (core stack), MEDIUM (DO provisioning specifics, limits TLS workaround)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **24-hour TTL** on all cached entries
- **Cache both** the raw LLM response and the parsed/structured estimation result
- **TTL-only expiration** — no version-tagged invalidation, no prompt-version cache busting
- **No manual flush endpoint or CLI command** — 24h TTL handles staleness
- **Fallback**: When Valkey is unreachable, fall back to direct LLM calls with no caching (no per-process dict fallback)
- **Log a warning on first Valkey failure**, suppress repeated warnings for a cooldown period (avoid log flood)
- **Lazy reconnect** — each incoming request tries Valkey; if it's back, cache resumes. No background health check
- **Service starts in degraded mode** if Valkey is unavailable at startup — logs warning, serves via live LLM
- **DigitalOcean Managed Valkey**, smallest available instance (1 GB)
- **Same region** as the application servers
- **No persistence** (ephemeral) — cache-only use case, data loss on restart is acceptable
- **Connection string via `VALKEY_URL` environment variable**
- **Key composition**: `tenant_id` + hash of estimation input parameters
- **Namespaced keys**: `efofx:llm:{tenant_id}:{input_hash}`
- **JSON serialization** for cached values (human-readable, debuggable via Valkey CLI)
- **Logging only** for observability — cache hits/misses at debug level, no structured metrics

### Claude's Discretion

- Valkey client library choice (valkey-py, redis-py compatibility, etc.)
- Connection pooling configuration
- Exact hash algorithm for input parameters
- Error handling details beyond the fallback policy
- How to structure the ValkeyCache service class

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| INFR-01 | Replace per-process LLM dict cache with distributed Valkey cache | valkey-py asyncio client; ValkeyCache service wraps `_response_cache` dict in `llm_service.py`; cross-worker cache via single Valkey instance |
| INFR-02 | Valkey cache keys prefixed with tenant_id to prevent cross-tenant collisions | Key format `efofx:llm:{tenant_id}:{input_hash}` using hashlib.sha256; tenant_id injected at ValkeyCache call site in llm_service |
| INFR-03 | Graceful Valkey fallback — cache outage falls back to live LLM call, not 500 | Catch `valkey.exceptions.ConnectionError`, `TimeoutError`; log warning with cooldown suppression; proceed with live LLM call |

</phase_requirements>

## Summary

Phase 6 replaces the module-level `_response_cache: dict[str, str]` in `app/services/llm_service.py` with a distributed `ValkeyCache` service backed by DigitalOcean Managed Valkey. The current per-process dict is invisible across Gunicorn workers — Worker A caches a result that Worker B never sees, making caching effectively useless in production.

The implementation is a two-part effort: (1) provision a DigitalOcean Managed Valkey 1 GB instance and wire `VALKEY_URL` into the app environment, and (2) implement a `ValkeyCache` service class with tenant-scoped keys (`efofx:llm:{tenant_id}:{input_hash}`), 24-hour TTL, and graceful fallback. The existing project already depends on `valkey>=6.1.0` (pinned in both `pyproject.toml` and `requirements.txt`) and uses `valkey.asyncio` via `slowapi`/`limits`. The `ValkeyCache` class uses `valkey.asyncio.from_url()` with the `VALKEY_URL` env var, and catches `ConnectionError`/`TimeoutError` to fall back to a live LLM call.

A critical integration concern is the `slowapi`/`limits` library that already uses `VALKEY_URL` for rate limiting. The `limits` library (which powers `slowapi`) supports `valkey://` and `async+valkey://` schemes but does **not** document `valkeys://` (TLS). DigitalOcean Managed Valkey requires TLS. The workaround is to use `rediss://` scheme for the `slowapi` `storage_uri` — the limits library supports `rediss://` and Valkey is wire-compatible with Redis. The `ValkeyCache` service (for LLM caching) uses `valkey.asyncio.from_url()` directly, which fully supports `valkeys://`.

**Primary recommendation:** Use `valkey.asyncio.Valkey.from_url(settings.VALKEY_URL)` in `ValkeyCache`, catch `(valkey.exceptions.ConnectionError, valkey.exceptions.TimeoutError)` for graceful fallback, and handle the `VALKEY_URL` scheme split between `slowapi` (needs `rediss://`) and `ValkeyCache` (can use `valkeys://` natively).

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `valkey` | `>=6.1.0` (6.1.1 current) | Python client for Valkey server | Already in project dependencies; fork of redis-py with Valkey protocol support; async-first |
| `valkey.asyncio` | same | Async Valkey client (sub-module) | FastAPI is async; avoids blocking event loop on I/O |
| `hashlib` (stdlib) | stdlib | SHA-256 for input hashing | Already used in `llm_service._make_cache_key`; STATE.md mandates hashlib.sha256 (never hash()) |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `fakeredis` | `>=2.0.0` (already in dev deps) | In-memory Valkey fake for tests | All unit tests of ValkeyCache — avoid real Valkey in CI |
| `json` (stdlib) | stdlib | Serialize EstimationOutput for Valkey storage | Locked decision: JSON not pickle |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `valkey.asyncio` | `redis.asyncio` | redis-py works (wire-compatible) but valkey-py is the project's stated dependency; STATE.md notes valkey as the chosen library |
| `fakeredis.FakeAsyncValkey` | `fakeredis.FakeAsyncRedis(server_type="valkey")` | Both work; `FakeAsyncValkey` is the direct class if available; `FakeAsyncRedis` with server_type is the fallback |
| SHA-256 of full params | CRC32 or MD5 | STATE.md explicitly: "use hashlib.sha256 (never hash())"; existing `_make_cache_key` already does this |

**Installation:** Already installed via `pyproject.toml` and `requirements.txt`. No new packages needed for production. `fakeredis>=2.0.0` is already in `[project.optional-dependencies] dev`.

## Architecture Patterns

### Recommended Project Structure

```
app/
├── services/
│   ├── llm_service.py          # MODIFY: replace _response_cache dict with ValkeyCache calls
│   └── valkey_cache.py         # NEW: ValkeyCache service class
├── core/
│   └── config.py               # MODIFY: add VALKEY_CACHE_TTL setting (86400 = 24h)
```

### Pattern 1: ValkeyCache Service Class

**What:** A standalone service class wrapping `valkey.asyncio` with graceful fallback, warning cooldown, and tenant-scoped key construction.

**When to use:** Injected into (or called from) `LLMService.generate_estimation()` as a replacement for the `_response_cache` dict.

**Example:**
```python
# app/services/valkey_cache.py
import hashlib
import json
import logging
import time
from typing import Optional

import valkey.asyncio as valkey
from valkey.exceptions import ConnectionError as ValkeyConnectionError, TimeoutError as ValkeyTimeoutError

from app.core.config import settings

logger = logging.getLogger(__name__)

# Cooldown: suppress repeated Valkey error logs for N seconds after first failure
_WARN_COOLDOWN_SECONDS = 60
_last_warn_at: float = 0.0


class ValkeyCache:
    """Distributed LLM response cache backed by Valkey.

    Keys: efofx:llm:{tenant_id}:{input_hash}
    Values: JSON-serialized EstimationOutput string
    TTL: 24 hours (VALKEY_CACHE_TTL setting)

    On any ConnectionError or TimeoutError, logs a warning (with cooldown
    suppression) and returns None — callers fall back to live LLM.
    """

    def __init__(self) -> None:
        self._client: Optional[valkey.Valkey] = None
        self._connected: bool = False

    def _get_client(self) -> valkey.Valkey:
        if self._client is None:
            # Lazy init: from_url supports valkeys:// for TLS
            self._client = valkey.Valkey.from_url(
                settings.VALKEY_URL,
                decode_responses=True,
                socket_timeout=2.0,
                socket_connect_timeout=2.0,
            )
        return self._client

    def _make_key(self, tenant_id: str, input_hash: str) -> str:
        return f"efofx:llm:{tenant_id}:{input_hash}"

    @staticmethod
    def make_input_hash(messages: list[dict], model: str) -> str:
        """SHA-256 hash of LLM input (messages + model) for cache key."""
        payload = {"messages": messages, "model": model}
        serialized = json.dumps(payload, sort_keys=True, ensure_ascii=True)
        return hashlib.sha256(serialized.encode("utf-8")).hexdigest()

    def _maybe_warn(self, exc: Exception) -> None:
        global _last_warn_at
        now = time.monotonic()
        if now - _last_warn_at > _WARN_COOLDOWN_SECONDS:
            logger.warning("Valkey unavailable — falling back to live LLM call: %s", exc)
            _last_warn_at = now

    async def get(self, tenant_id: str, input_hash: str) -> Optional[str]:
        """Return cached JSON string or None if miss/unreachable."""
        try:
            client = self._get_client()
            key = self._make_key(tenant_id, input_hash)
            value = await client.get(key)
            if value is not None:
                logger.debug("Valkey cache HIT: %s", key[:32])
            else:
                logger.debug("Valkey cache MISS: %s", key[:32])
            return value
        except (ValkeyConnectionError, ValkeyTimeoutError) as exc:
            self._maybe_warn(exc)
            return None

    async def set(self, tenant_id: str, input_hash: str, value: str) -> None:
        """Store JSON string with 24-hour TTL. Silently no-ops on error."""
        try:
            client = self._get_client()
            key = self._make_key(tenant_id, input_hash)
            await client.set(key, value, ex=settings.VALKEY_CACHE_TTL)
        except (ValkeyConnectionError, ValkeyTimeoutError) as exc:
            self._maybe_warn(exc)

    async def close(self) -> None:
        if self._client is not None:
            await self._client.aclose()
            self._client = None
```

### Pattern 2: Module-Level Singleton (lifecycle via lifespan)

**What:** A single `ValkeyCache` instance at module level, initialized lazily (no startup dependency), closed in the FastAPI lifespan shutdown.

**When to use:** This mirrors the existing `_response_cache` dict pattern — module-level — without requiring dependency injection into every call site.

```python
# app/services/valkey_cache.py (bottom of file)
_cache = ValkeyCache()  # module-level singleton

# app/main.py lifespan — add to shutdown block:
from app.services.valkey_cache import _cache as valkey_cache
yield
# ...shutdown...
await valkey_cache.close()
```

**Callers in `llm_service.py`:**
```python
# In generate_estimation(), replace _response_cache dict with:
from app.services.valkey_cache import _cache as valkey_cache

input_hash = _make_cache_key(messages, settings.OPENAI_MODEL)  # existing helper

if use_cache:
    cached = await valkey_cache.get(tenant_id, input_hash)
    if cached is not None:
        return EstimationOutput.model_validate_json(cached)

# ... LLM call ...

if use_cache:
    await valkey_cache.set(tenant_id, input_hash, result.model_dump_json())
```

### Pattern 3: TLS Connection (DigitalOcean Managed Valkey)

**What:** DigitalOcean Managed Valkey requires TLS. The `valkeys://` scheme in `valkey.asyncio.from_url()` enables SSL. DigitalOcean uses Let's Encrypt, so no CA cert download is required.

```python
# VALKEY_URL in production .env:
VALKEY_URL=valkeys://default:PASSWORD@host.db.ondigitalocean.com:25061/0?ssl_cert_reqs=required

# For local dev (plain, no TLS):
VALKEY_URL=redis://localhost:6379
```

**Note on slowapi:** The `limits` library backing `slowapi` supports `valkey://` but NOT `valkeys://` for TLS. For `slowapi`'s rate limiter in production, use `rediss://` (Redis TLS scheme) — Valkey is wire-compatible and DigitalOcean Managed Valkey accepts `rediss://` connections.

```python
# If VALKEY_URL is valkeys://, slowapi needs a different URL:
# Separate env vars, or derive: RATE_LIMIT_URL = VALKEY_URL.replace("valkeys://", "rediss://")
# Or simply use rediss:// for VALKEY_URL and the ValkeyCache client uses ssl=True
```

The simplest solution: set `VALKEY_URL=rediss://...` (which `limits` library understands), and the `ValkeyCache` service uses `valkey.asyncio.from_url()` which also accepts `rediss://` with `ssl=True`. Both work.

### Pattern 4: Tenant-ID Injection at Call Site

**What:** `LLMService.generate_estimation()` already receives `tenant_id` indirectly via the FastAPI dependency (`get_llm_service` scopes to a tenant). The tenant_id must be threaded into the cache key.

**Option A:** Pass `tenant_id` as a parameter to `generate_estimation()`.

**Option B:** Store `tenant_id` on the `LLMService` instance at construction (via `get_llm_service` dependency which already has `tenant.tenant_id`).

Option B is cleaner — `get_llm_service` already has the `Tenant` object, so `LLMService.__init__` can accept `tenant_id: str` alongside `api_key: str`.

```python
class LLMService:
    def __init__(self, api_key: str, tenant_id: str) -> None:
        self.api_key = api_key
        self.tenant_id = tenant_id
        self.client = AsyncOpenAI(api_key=self.api_key)
        # ...

async def get_llm_service(tenant: Tenant = Depends(get_current_tenant)) -> LLMService:
    api_key = await decrypt_tenant_openai_key(tenant.tenant_id)
    return LLMService(api_key=api_key, tenant_id=tenant.tenant_id)
```

### Anti-Patterns to Avoid

- **Keeping the per-process dict as fallback**: The locked decision explicitly says "no per-process dict fallback". Remove it entirely.
- **pickle serialization**: STATE.md is explicit: "serialized as JSON (never pickle)". `EstimationOutput.model_dump_json()` / `model_validate_json()` is correct.
- **hash() builtin for cache key**: STATE.md: "use hashlib.sha256 (never hash())". The existing `_make_cache_key()` already uses SHA-256 — keep it.
- **Using `VALKEY_URL` with `valkeys://` for slowapi `storage_uri`**: The `limits` library only supports `valkey://` (not `valkeys://`). Use `rediss://` for the rate limiter storage URI.
- **Background health-check loop**: Locked decision is lazy reconnect — each request tries; no background polling.
- **Blocking Valkey call in async context**: Use `valkey.asyncio` (not `valkey.Valkey` sync client). A blocking call would deadlock the FastAPI event loop.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Connection pooling | Custom pool manager | `valkey.asyncio.from_url()` default pool | valkey-py creates and manages a pool automatically per client instance |
| TLS negotiation | Manual SSL context | `valkeys://` or `ssl=True` in `from_url()` | valkey-py wraps ssl module; DigitalOcean Let's Encrypt accepted without custom CA |
| Key expiry enforcement | Background eviction thread | `await client.set(key, value, ex=86400)` | Valkey server handles TTL atomically; EX parameter is the standard approach |
| Test isolation | Real Valkey in CI | `fakeredis.FakeAsyncValkey` or `FakeAsyncRedis(server_type="valkey")` | fakeredis is already in dev deps; zero network required for unit tests |

**Key insight:** valkey-py mirrors redis-py's API almost exactly; the transition is mostly import path changes (`import valkey.asyncio as valkey` vs `import redis.asyncio as redis`).

## Common Pitfalls

### Pitfall 1: slowapi TLS URL Scheme Mismatch

**What goes wrong:** Setting `VALKEY_URL=valkeys://...` for DigitalOcean TLS causes `slowapi`'s Limiter to fail at startup because the `limits` library does not support `valkeys://`. The rate limiter may raise an unrecognized scheme error or silently fall back to in-memory, breaking distributed rate limiting.

**Why it happens:** `slowapi` uses the `limits` library under the hood. `limits` added `valkey://` support in v4.3, but did not add `valkeys://` (TLS). It does support `rediss://` (Redis TLS).

**How to avoid:** Use `rediss://user:pass@host:port/0` for `VALKEY_URL` instead of `valkeys://`. Both the `limits` library rate limiter AND `valkey.asyncio.from_url()` (ValkeyCache) accept `rediss://` — it still connects to the Valkey instance over TLS. Note: STATE.md acknowledges this as a known concern to verify: "Verify whether slowapi accepts valkeys:// TLS URL scheme before provisioning."

**Warning signs:** `slowapi` Limiter constructor raises `ConfigurationError` or logs "unsupported scheme" at startup.

### Pitfall 2: Sync Valkey Client in Async FastAPI Handler

**What goes wrong:** Using `import valkey` (sync) instead of `import valkey.asyncio` blocks the event loop, stalling all concurrent requests.

**Why it happens:** valkey-py exports both sync and async clients; the import path differs by one segment.

**How to avoid:** Always `import valkey.asyncio as valkey` in the `ValkeyCache` service. The `from_url()` call on the async client returns an async client.

**Warning signs:** Request latency spikes; event loop "is blocked" warnings in uvicorn logs.

### Pitfall 3: Missing tenant_id in Cache Key Causes Cross-Tenant Pollution

**What goes wrong:** Two tenants with identical project descriptions get each other's estimation results. INFR-02 is violated.

**Why it happens:** If `tenant_id` is not injected into the cache key, the hash of identical inputs produces the same key regardless of tenant.

**How to avoid:** Always include `tenant_id` as the first segment of the key: `efofx:llm:{tenant_id}:{input_hash}`. The `ValkeyCache.make_input_hash()` computes input-only hash; `ValkeyCache.get/set()` prepend the tenant prefix. See Pattern 4 for tenant_id injection.

**Warning signs:** Tenants seeing wrong estimates; estimation content for Tenant A visible to Tenant B.

### Pitfall 4: Uncaught Exception Types Cause 500 on Valkey Outage

**What goes wrong:** Valkey outage raises `valkey.exceptions.TimeoutError` or `valkey.exceptions.MaxConnectionsError` — if only `ConnectionError` is caught, those propagate and crash the request with 500.

**Why it happens:** There are multiple exception types for connection problems; catching only one is insufficient.

**How to avoid:** Catch the tuple `(ValkeyConnectionError, ValkeyTimeoutError)` minimum. For safety, could also catch `Exception` with a log, but that's broader than needed. The `valkey.exceptions` module exports: `ConnectionError`, `TimeoutError`, `MaxConnectionsError`, `BusyLoadingError`.

**Warning signs:** 500 responses during Valkey maintenance windows; only some connection errors falling back correctly.

### Pitfall 5: Log Flood During Extended Valkey Outage

**What goes wrong:** Every request logs "Valkey unavailable" during an outage — if there are 100 req/s, that's 100 log lines per second.

**Why it happens:** Logging without cooldown in a high-throughput path.

**How to avoid:** Use a module-level `_last_warn_at: float` timestamp and only log if `time.monotonic() - _last_warn_at > _WARN_COOLDOWN_SECONDS` (e.g., 60 seconds). The locked decision specifies this pattern explicitly.

**Warning signs:** Log aggregation costs spike during Valkey maintenance; logs become unreadable.

### Pitfall 6: pytest-asyncio Event Loop Scope Mismatch with FakeAsyncValkey

**What goes wrong:** Using a session-scoped fixture for `FakeAsyncValkey` with function-scoped event loop in `pytest-asyncio==1.3.0` raises "tried to access function scoped fixture with session scoped request object."

**Why it happens:** The project uses `asyncio_default_fixture_loop_scope = "function"` in `pyproject.toml`. Session-scoped async fixtures conflict.

**How to avoid:** Scope `FakeAsyncValkey` fixtures at function level (default). Each test gets a fresh fake. See Code Examples below.

## Code Examples

Verified patterns from official sources:

### Async Client Initialization (from_url)

```python
# Source: https://valkey-py.readthedocs.io/en/latest/connections.html
import valkey.asyncio as valkey

# Local dev (no TLS):
client = valkey.Valkey.from_url("redis://localhost:6379", decode_responses=True)

# DigitalOcean Managed Valkey (TLS via rediss://):
client = valkey.Valkey.from_url(
    "rediss://default:PASSWORD@host.db.ondigitalocean.com:25061/0",
    decode_responses=True,
    socket_timeout=2.0,
    socket_connect_timeout=2.0,
)

# Alternatively with valkeys:// scheme (TLS, not compatible with limits/slowapi):
client = valkey.Valkey.from_url(
    "valkeys://default:PASSWORD@host.db.ondigitalocean.com:25061/0",
    decode_responses=True,
)
```

### SET with TTL (EX parameter)

```python
# Source: valkey-py API reference (EX = seconds)
await client.set("efofx:llm:tenant-abc:abc123", json_value, ex=86400)  # 24h TTL
```

### GET

```python
# Source: valkey-py API reference
value: Optional[str] = await client.get("efofx:llm:tenant-abc:abc123")
# Returns None on miss; returns str if decode_responses=True
```

### Connection Cleanup (lifespan)

```python
# Source: https://valkey-py.readthedocs.io/en/latest/examples/asyncio_examples.html
await client.aclose()  # closes internal connection pool
```

### Exception Catching for Fallback

```python
from valkey.exceptions import ConnectionError as ValkeyConnectionError
from valkey.exceptions import TimeoutError as ValkeyTimeoutError

try:
    value = await client.get(key)
except (ValkeyConnectionError, ValkeyTimeoutError) as exc:
    logger.warning("Valkey unavailable: %s", exc)
    value = None  # Fall back to live LLM
```

### FakeAsyncValkey in Tests

```python
# Source: fakeredis docs + valkey-py compatibility
import pytest
import pytest_asyncio
import fakeredis

@pytest_asyncio.fixture  # function-scoped (default) — matches project asyncio_default_fixture_loop_scope
async def fake_valkey():
    """In-memory fake Valkey client for unit tests — no real server needed."""
    async with fakeredis.FakeAsyncRedis(server_type="valkey") as client:
        yield client
```

### Key Format Verification

```python
# Conformance to efofx:llm:{tenant_id}:{input_hash} pattern
def _make_key(tenant_id: str, input_hash: str) -> str:
    return f"efofx:llm:{tenant_id}:{input_hash}"

# input_hash is SHA-256 hex of json.dumps({"messages": [...], "model": "..."}, sort_keys=True)
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `_response_cache: dict[str, str]` (module-level, per-process) | `ValkeyCache` backed by DigitalOcean Managed Valkey | Phase 6 | Cache hits work across all Gunicorn workers; tenant isolation enforced |
| `redis-py` only | `valkey-py` (fork of redis-py) | Valkey 1.0 release 2024 | Same API, same wire protocol, but Valkey-specific features going forward |
| Manual SSL context for TLS | `rediss://` or `valkeys://` scheme in `from_url()` | valkey-py >= 6.0 | One-liner TLS; DigitalOcean uses Let's Encrypt, no CA cert download needed |
| `limits` library pre-4.3 | `limits >= 4.3` with `valkey://` scheme | limits 4.3 (2024) | Native valkey-py used instead of redis-py for rate limit storage |

**Deprecated/outdated:**
- `_response_cache` dict: Per-process; invisible to other Gunicorn workers. Removed in Phase 6.
- `hash()` builtin for cache keys: Non-deterministic across Python restarts; STATE.md mandates `hashlib.sha256`.

## Open Questions

1. **Which URL scheme for unified `VALKEY_URL`?**
   - What we know: `limits` library supports `valkey://` and `rediss://` but NOT `valkeys://`. `valkey.asyncio.from_url()` supports both `valkeys://` and `rediss://`.
   - What's unclear: Whether the project should use one env var (`VALKEY_URL`) for both or two separate vars.
   - Recommendation: Use `VALKEY_URL=rediss://...` for both — it works with `limits` (slowapi) AND `valkey.asyncio` (ValkeyCache). Document this in the `.env.example`. This resolves the STATE.md concern: "Verify whether slowapi accepts valkeys:// TLS URL scheme."

2. **DigitalOcean Valkey provisioning port**
   - What we know: DigitalOcean Managed Valkey default port is 25061 (not 6379). Connection string uses `Let's Encrypt`, no CA cert download.
   - What's unclear: Whether the DigitalOcean control panel shows a one-click connection string with `rediss://` scheme for direct copy-paste.
   - Recommendation: Use `doctl databases connection <cluster-id> --user default` to retrieve full connection details; verify `VALKEY_URL` format before wiring into production `.env`.

3. **`ValkeyCache` as singleton vs. injected dependency**
   - What we know: The existing `_response_cache` dict is module-level; `LLMService` accesses it via module import.
   - What's unclear: Whether to inject `ValkeyCache` into `LLMService` constructor (testable) or keep module-level singleton.
   - Recommendation: Module-level singleton (`_cache = ValkeyCache()`) matches current pattern and avoids refactoring `get_llm_service`. For tests, inject via parameter override or patch.

## Validation Architecture

Note: `workflow.nyquist_validation` is not set in `.planning/config.json` (field absent — treated as disabled). Skipping formal validation architecture section per instructions.

However, for planning purposes, the test infrastructure is documented:

**Test framework:** pytest 8.4.1 + pytest-asyncio 1.3.0 (configured in `pyproject.toml`)
**Test location:** `apps/efofx-estimate/tests/services/`
**Dev dependency:** `fakeredis>=2.0.0` (already in `pyproject.toml` dev extras)
**Quick run:** `pytest tests/services/test_valkey_cache.py -x`
**Full suite:** `pytest tests/ -x`

**Wave 0 gaps (files that don't yet exist):**
- `tests/services/test_valkey_cache.py` — covers INFR-01, INFR-02, INFR-03
  - Test: cache hit returns stored value without LLM call
  - Test: cache miss triggers LLM call and stores result
  - Test: tenant A key not readable under tenant B prefix (INFR-02)
  - Test: `ConnectionError` caught, returns None (INFR-03)
  - Test: `TimeoutError` caught, returns None (INFR-03)
  - Test: warning cooldown suppresses repeated logs
- Modify `tests/services/test_llm_service.py` — update `clear_response_cache` fixture to patch ValkeyCache instead of `_response_cache` dict

## Sources

### Primary (HIGH confidence)
- `https://valkey-py.readthedocs.io/en/latest/connections.html` — `from_url()`, TLS schemes (`valkeys://`), connection pool params
- `https://valkey-py.readthedocs.io/en/latest/exceptions.html` — `ConnectionError`, `TimeoutError`, `MaxConnectionsError` exception types
- `https://valkey-py.readthedocs.io/en/latest/examples/asyncio_examples.html` — async client usage, `aclose()` pattern
- `https://limits.readthedocs.io/en/stable/storage.html` — `valkey://` scheme (added v4.3), `rediss://` TLS, no `valkeys://` support confirmed
- Project `pyproject.toml` — `valkey>=6.1.0` already in dependencies; `fakeredis>=2.0.0` in dev deps
- Project `app/services/llm_service.py` — existing `_response_cache` dict, `_make_cache_key()` with SHA-256, `generate_estimation()` cache hit/miss logic
- Project `app/core/config.py` — `VALKEY_URL: str = "redis://localhost:6379"` already exists
- Project `app/core/rate_limit.py` — `slowapi` already using `VALKEY_URL` as `storage_uri`
- Project `.planning/STATE.md` — "Valkey cache keys: prefixed with tenant_id, use hashlib.sha256 (never hash()), serialized as JSON (never pickle)"; concern about `valkeys://` TLS scheme
- `https://github.com/valkey-io/valkey-py/releases/` — v6.1.1 is current (August 2025)

### Secondary (MEDIUM confidence)
- `https://docs.digitalocean.com/products/databases/valkey/how-to/connect/` — TLS required, Let's Encrypt (no CA cert needed), port 25061, connection string format
- `https://docs.digitalocean.com/products/databases/valkey/details/pricing/` — Single node 1 GiB starts at $15/month (confirmed smallest available plan)
- `https://pypi.org/project/fakeredis/` — `FakeAsyncRedis(server_type="valkey")` usage, `FakeAsyncValkey` class, version 2.34.0 current

### Tertiary (LOW confidence)
- `https://fakeredis.readthedocs.io/` — `FakeAsyncValkey` class name (specific class vs. server_type param) — documentation was incomplete; use `FakeAsyncRedis(server_type="valkey")` as the verified fallback

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — valkey-py is already in project deps; API verified against official docs
- Architecture: HIGH — ValkeyCache service pattern derived from existing llm_service.py structure; fallback pattern verified against exception docs
- Pitfalls: HIGH (slowapi scheme mismatch verified; log flood pattern well-known); MEDIUM (DO port 25061 from docs, verified)
- DigitalOcean provisioning: MEDIUM — pricing and TLS behavior verified against official DO docs; exact control panel steps not verified

**Research date:** 2026-03-01
**Valid until:** 2026-04-01 (stable libraries; DO pricing may change)
