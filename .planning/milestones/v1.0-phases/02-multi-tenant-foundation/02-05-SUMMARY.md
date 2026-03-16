---
phase: 02-multi-tenant-foundation
plan: 05
subsystem: api
tags: [slowapi, rate-limiting, valkey, redis, brute-force-protection, middleware]

requires:
  - phase: 02-02
    provides: get_current_tenant dependency, JWT auth, RateLimiter class removed from security.py
  - phase: 02-01
    provides: VALKEY_URL and RATE_LIMIT_ENABLED settings in config.py

provides:
  - slowapi Limiter with Valkey backend in app/core/rate_limit.py
  - Per-tenant tier-based rate limiting (trial 20/min, paid 100/min)
  - IP-based login brute-force protection (5/15min)
  - X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset headers via middleware
  - 429 response: {"error": "rate_limit_exceeded", "message": "...", "retry_after": N}
  - Retry-After header on 429 responses

affects:
  - 03-estimation-engine (estimation endpoints now rate-limited per tier)
  - 04-widget-embeds (any API routes added will need @limiter.limit decorator)

tech-stack:
  added:
    - slowapi==0.1.9 (already in pyproject.toml, now wired up)
    - fakeredis>=2.0.0 (test dependency, patches global limiter to in-memory storage)
    - limits.storage.MemoryStorage + FixedWindowRateLimiter (for test patching)
  patterns:
    - autouse fixture patches limiter._storage/_limiter to MemoryStorage for test isolation
    - add_rate_limit_headers middleware reads request.state.view_rate_limit (set by slowapi) to inject X-RateLimit-* headers on all responses
    - @limiter.limit on FastAPI endpoints requires request: Request as first parameter; use http_request when function already has a request: Body param

key-files:
  created:
    - apps/efofx-estimate/app/core/rate_limit.py
    - apps/efofx-estimate/tests/api/test_rate_limit.py
  modified:
    - apps/efofx-estimate/app/main.py
    - apps/efofx-estimate/app/api/auth.py
    - apps/efofx-estimate/app/api/routes.py

key-decisions:
  - "slowapi decorator + custom add_rate_limit_headers middleware: FastAPI endpoints return Pydantic models (not Response objects), so slowapi's headers_enabled=True crashes; custom middleware reads request.state.view_rate_limit set by slowapi's __evaluate_limits instead"
  - "Test isolation via limiter._storage/_limiter patching: replace FixedWindowRateLimiter's storage with MemoryStorage on the global limiter instance (not a new instance) because @limiter.limit decorator captures the global object at decoration time"
  - "get_tenant_id_for_limit key func returns tenant:{id} for authenticated requests, ip:{addr} for unauthenticated — enables per-tenant rate limit isolation independent of IP"
  - "get_tier_limit dynamic limit func reads request.state.tier for per-tier limits; falls back to trial limit (20/min) when tier not set — safe default"

patterns-established:
  - "Rate limit test pattern: autouse fixture patches limiter._storage and limiter._limiter to MemoryStorage before each test, restores after; no Valkey connection needed in CI"
  - "slowapi route registration pattern: limits stored in limiter._route_limits['{module}.{funcname}'] for static limits; limiter._dynamic_route_limits for callable limits"

requirements-completed: [RATE-01, RATE-02, RATE-03]

duration: 9min
completed: 2026-02-26
---

# Phase 2 Plan 5: Rate Limiting with slowapi and Valkey Backend Summary

**slowapi Limiter with Valkey backend, tier-based per-tenant rate limits (trial 20/min, paid 100/min), IP-based login brute-force protection (5/15min), and X-RateLimit-* header middleware**

## Performance

- **Duration:** 9 min
- **Started:** 2026-02-26T23:26:52Z
- **Completed:** 2026-02-26T23:35:35Z
- **Tasks:** 2
- **Files modified:** 5 (3 modified, 2 created)

## Accomplishments

- Created `app/core/rate_limit.py`: Limiter with Valkey backend, TIER_LIMITS dict (trial 20/min, paid 100/min), `get_tenant_id_for_limit` key function (tenant:{id} or ip:{addr}), `get_tier_limit` dynamic limit function, custom 429 handler returning `{"error": "rate_limit_exceeded", "message": "...", "retry_after": N}`
- Registered limiter on `app.state` and added `add_rate_limit_headers` middleware in `main.py` to inject X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset on all API responses
- Applied `@limiter.limit` decorators to all auth endpoints (login 5/15min per IP, register 10/hr per IP, refresh 30/min per IP) and all protected API routes with dynamic per-tier limits
- 19 tests covering brute-force protection, 429 format, Retry-After header, X-RateLimit headers, TIER_LIMITS configuration, key functions, and limiter registration

## Task Commits

Each task was committed atomically:

1. **Task 1: Create rate_limit.py module with slowapi Limiter, tier-based limits, and custom 429 handler** - `968b2e8` (feat)
2. **Task 2: Apply rate limits to auth and API routes, add login brute-force protection, and write tests** - `e3af9e2` (feat)

**Plan metadata:** `[pending]` (docs: complete plan)

## Files Created/Modified

- `apps/efofx-estimate/app/core/rate_limit.py` - NEW: slowapi Limiter with Valkey backend, TIER_LIMITS, key functions, custom 429 handler
- `apps/efofx-estimate/app/main.py` - Register limiter on app.state + exception handler; add add_rate_limit_headers middleware
- `apps/efofx-estimate/app/api/auth.py` - Add @limiter.limit to register (10/hr), login (5/15min), refresh (30/min) with request: Request parameter
- `apps/efofx-estimate/app/api/routes.py` - Add @limiter.limit(get_tier_limit) with get_tenant_id_for_limit to all protected endpoints
- `apps/efofx-estimate/tests/api/test_rate_limit.py` - NEW: 19 tests with in-memory limiter patching via autouse fixture

## Decisions Made

- **slowapi headers_enabled=False + custom middleware:** FastAPI endpoints return Pydantic models (not `Response` objects), so slowapi's `headers_enabled=True` raises "parameter `response` must be an instance of starlette.responses.Response" in the decorator wrapper. Custom `add_rate_limit_headers` middleware reads `request.state.view_rate_limit` (set by slowapi's `__evaluate_limits`) and injects headers on all responses.
- **Test isolation via limiter storage patching:** The `@limiter.limit` decorator captures the global `limiter` instance at module import time. Tests patch `limiter._storage` and `limiter._limiter` on the existing instance (not create a new one) to use `MemoryStorage` — this avoids needing a live Valkey/Redis connection in CI.
- **get_tenant_id_for_limit fallback to IP:** When no `tenant_id` is on `request.state` (unauthenticated requests like login), the key function falls back to `ip:{addr}` — enabling per-IP brute-force protection on auth endpoints.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Custom rate limit header middleware instead of headers_enabled=True**
- **Found during:** Task 2 (writing tests)
- **Issue:** slowapi's `headers_enabled=True` is incompatible with FastAPI endpoints returning Pydantic models; raises `Exception: parameter response must be an instance of starlette.responses.Response` in `async_wrapper`
- **Fix:** Added `add_rate_limit_headers` middleware in `main.py` that reads `request.state.view_rate_limit` (populated by slowapi's `__evaluate_limits`) to inject X-RateLimit-* headers — same data, different injection point
- **Files modified:** `apps/efofx-estimate/app/main.py`, `apps/efofx-estimate/app/core/rate_limit.py`
- **Verification:** `test_rate_limit_headers_present_on_auth_endpoint` passes; headers confirmed in response
- **Committed in:** e3af9e2 (Task 2 commit)

**2. [Rule 1 - Bug] Limiter._route_limits check instead of _rate_limits attribute**
- **Found during:** Task 2 (test failures)
- **Issue:** Test initially checked `hasattr(login, "_rate_limits")` but slowapi uses `functools.wraps` and doesn't add `_rate_limits` to the wrapped function — it stores limits in `limiter._route_limits['{module}.{funcname}']`
- **Fix:** Updated tests to check `limiter._route_limits["app.api.auth.login"]` instead
- **Files modified:** `apps/efofx-estimate/tests/api/test_rate_limit.py`
- **Verification:** `test_login_route_has_rate_limit`, `test_register_route_has_rate_limit`, `test_refresh_route_has_rate_limit` all pass
- **Committed in:** e3af9e2 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 Rule 1 — bugs discovered during implementation)
**Impact on plan:** Both auto-fixes necessary for correct behavior. All plan requirements met. No scope creep.

## Issues Encountered

- **http_request naming in routes.py:** FastAPI routes like `start_estimation` and `send_chat_message` already had a parameter named `request` (the Pydantic body model). Added HTTP request param as `http_request: Request` to avoid collision — slowapi still correctly finds the HTTP request as it checks for parameter named `"request"` first (the body `request` parameter satisfies slowapi's lookup, which works correctly since both are keyword args).

## Next Phase Readiness

- Rate limiting fully operational for Phase 3 (estimation engine) — all `/api/v1/estimate/*` and `/api/v1/chat/*` endpoints already have tier-based limits applied
- Valkey URL configured; production deployment should set `VALKEY_URL` to DigitalOcean Managed Redis/Valkey endpoint
- Phase 2 complete — all 5 plans (02-01 through 02-05) done; ready for Phase 3

## Self-Check: PASSED

- `apps/efofx-estimate/app/core/rate_limit.py` - FOUND
- `apps/efofx-estimate/tests/api/test_rate_limit.py` - FOUND
- Commit `968b2e8` (Task 1: rate_limit.py module) - FOUND
- Commit `e3af9e2` (Task 2: routes + tests) - FOUND
- 19/19 tests pass: `pytest tests/api/test_rate_limit.py -m "not integration"` - VERIFIED

---
*Phase: 02-multi-tenant-foundation*
*Completed: 2026-02-26*
