---
phase: 06-valkey-infrastructure
plan: 01
subsystem: infra
tags: [valkey, redis, caching, llm, distributed-cache, fakeredis, tenant-isolation]

requires:
  - phase: 05-tech-debt-foundation-cleanup
    provides: LLMService with BYOK key injection and rate limiting via slowapi+Valkey

provides:
  - ValkeyCache service at app/services/valkey_cache.py with distributed LLM response caching
  - Tenant-scoped cache keys (efofx:llm:{tenant_id}:{input_hash})
  - Graceful Valkey fallback — connection errors never surface as 500s
  - LLMService wired with ValkeyCache and tenant_id parameter
  - Lifespan shutdown closes Valkey connection cleanly
  - 29 unit tests covering cache behavior, tenant isolation, fallback, and warning cooldown

affects: [07-magic-link-feedback, 08-calibration-service]

tech-stack:
  added: [valkey.asyncio (Valkey.from_url), fakeredis (FakeAsyncRedis server_type=valkey)]
  patterns:
    - "Lazy async client init: _get_client() creates client on first use, not at import time"
    - "Module-level singleton (_cache = ValkeyCache()) shared across requests"
    - "Warning cooldown: module-level _last_warn_at float throttles log flood on outages"
    - "Test isolation: inject FakeAsyncRedis via cache._client = fakeredis.FakeAsyncRedis(...)"

key-files:
  created:
    - apps/efofx-estimate/app/services/valkey_cache.py
    - apps/efofx-estimate/tests/services/test_valkey_cache.py
  modified:
    - apps/efofx-estimate/app/core/config.py
    - apps/efofx-estimate/app/services/llm_service.py
    - apps/efofx-estimate/app/main.py
    - apps/efofx-estimate/app/core/rate_limit.py
    - apps/efofx-estimate/tests/services/test_llm_service.py

key-decisions:
  - "Cache key format locked as efofx:llm:{tenant_id}:{input_hash} — tenant prefix before input hash for tenant-scoped Redis SCAN patterns"
  - "ValkeyCache._get_client() uses lazy init (not at module import) to avoid connection overhead during test collection and to support clean lifespan close"
  - "Warning cooldown via module-level _last_warn_at float (not per-instance) to throttle across all requests sharing the singleton"
  - "fakeredis.FakeAsyncRedis(server_type='valkey') used for tests — confirmed supported by installed fakeredis>=2.0.0"
  - "Pre-existing test failures (test_performance_requirement timing test, test_auth.py Redis connection) are out-of-scope and unrelated to this plan"

patterns-established:
  - "Graceful fallback pattern: catch (ValkeyConnectionError, ValkeyTimeoutError), call _maybe_warn(), return None / no-op — never re-raise"
  - "Test fixture pattern: inject FakeAsyncRedis directly via cache._client rather than patching from_url"
  - "Autouse mock fixture: mock_valkey_cache patches app.services.llm_service.valkey_cache for entire test module"

requirements-completed: [INFR-01, INFR-02, INFR-03]

duration: 4min
completed: 2026-03-01
---

# Phase 6 Plan 01: Valkey Distributed LLM Cache Summary

**ValkeyCache service replacing per-process dict cache with distributed Valkey storage, tenant-scoped keys (efofx:llm:{tenant_id}:{input_hash}), and graceful connection error fallback via caught ValkeyConnectionError/ValkeyTimeoutError**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-01T19:16:21Z
- **Completed:** 2026-03-01T19:20:40Z
- **Tasks:** 3
- **Files modified:** 7

## Accomplishments

- Created ValkeyCache service with lazy async client, tenant-scoped keys, graceful fallback, and warning cooldown — fulfills INFR-01, INFR-02, INFR-03
- Wired ValkeyCache into LLMService.generate_estimation() and lifespan shutdown, removing the per-process _response_cache dict entirely
- Added 29 unit tests (12 for ValkeyCache, 17 updated for LLMService) using FakeAsyncRedis for isolation

## Task Commits

Each task was committed atomically:

1. **Task 1: Create ValkeyCache service with tenant-scoped keys and graceful fallback** - `8b469db` (feat)
2. **Task 2: Wire ValkeyCache into LLMService, lifespan shutdown, and rate limiter URL handling** - `3a3acfa` (feat)
3. **Task 3: Add ValkeyCache unit tests and update existing LLMService test fixtures** - `015249c` (test)

**Plan metadata:** (docs commit, see below)

## Files Created/Modified

- `apps/efofx-estimate/app/services/valkey_cache.py` - ValkeyCache class with lazy client, tenant-scoped keys, graceful fallback, warning cooldown, _cache singleton
- `apps/efofx-estimate/app/core/config.py` - Added VALKEY_CACHE_TTL = 86400 (24h) setting
- `apps/efofx-estimate/app/services/llm_service.py` - Removed _response_cache dict and _make_cache_key function; added tenant_id param to LLMService.__init__; wired valkey_cache.get/set
- `apps/efofx-estimate/app/main.py` - Import _cache as valkey_cache; added await valkey_cache.close() in lifespan shutdown
- `apps/efofx-estimate/app/core/rate_limit.py` - Added TLS scheme (rediss://) comment above Limiter storage_uri
- `apps/efofx-estimate/tests/services/test_valkey_cache.py` - New: 12 tests for ValkeyCache (hash, get/set, tenant isolation, fallback, warning cooldown)
- `apps/efofx-estimate/tests/services/test_llm_service.py` - Updated: mock_valkey_cache autouse fixture; tenant_id in constructor tests; ValkeyCache.make_input_hash in key tests; side_effect-based caching tests; tenant_id assertion in get_llm_service test

## Decisions Made

- Cache key format: `efofx:llm:{tenant_id}:{input_hash}` — tenant prefix before input hash enables tenant-scoped Redis SCAN and exact-match gets
- Lazy client initialization in `_get_client()` avoids connection attempt at import time, which would break tests that don't need Valkey
- Warning cooldown uses module-level `_last_warn_at` (shared across requests via the singleton) rather than per-instance state — ensures throttle applies across the entire process
- `fakeredis.FakeAsyncRedis(server_type="valkey")` confirmed supported; used in all ValkeyCache tests for in-memory behavior without a live Valkey instance

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- `test_performance_requirement` and `test_auth.py::test_register_success` failures confirmed as pre-existing (verified by stashing changes and re-running). Both are unrelated to this plan (timing-dependent performance test and Redis connection test requiring a live Redis instance). Logged as out-of-scope.

## User Setup Required

**External services require manual configuration.** The `VALKEY_URL` environment variable must point to the DigitalOcean Managed Valkey cluster when deployed:

- **DigitalOcean Dashboard:** Databases -> Create Database Cluster -> Valkey (1 GB, same region as app servers, no persistence/ephemeral)
- **Environment variable to add:** `VALKEY_URL=rediss://default:PASSWORD@host.db.ondigitalocean.com:25061/0` (use `rediss://` scheme, not `valkeys://`)
- **Note:** The `rediss://` scheme is required for both ValkeyCache (valkey.asyncio) and slowapi/limits library TLS compatibility

See plan frontmatter `user_setup` section for full details.

## Next Phase Readiness

- INFR-01, INFR-02, INFR-03 all complete — distributed cache with tenant isolation and graceful fallback is production-ready
- DigitalOcean Managed Valkey cluster still needs provisioning (user_setup step in 06-01-PLAN.md frontmatter)
- Phase 6 Plan 02 can proceed once Valkey cluster is provisioned and VALKEY_URL is set

---
*Phase: 06-valkey-infrastructure*
*Completed: 2026-03-01*

## Self-Check: PASSED

- apps/efofx-estimate/app/services/valkey_cache.py: FOUND
- apps/efofx-estimate/tests/services/test_valkey_cache.py: FOUND
- .planning/phases/06-valkey-infrastructure/06-01-SUMMARY.md: FOUND
- Commit 8b469db (Task 1): FOUND
- Commit 3a3acfa (Task 2): FOUND
- Commit 015249c (Task 3): FOUND
