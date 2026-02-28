---
phase: 02-multi-tenant-foundation
plan: "02"
subsystem: auth
tags: [jwt, pyjwt, refresh-tokens, sha256, fastapi, mongodb, bcrypt, token-rotation]

# Dependency graph
requires:
  - phase: 02-01
    provides: Tenant model with hashed_password/hashed_api_key, register/verify flow, bcrypt hasher, JWT_SECRET_KEY config

provides:
  - JWT access tokens (20-min expiry, sub/tenant_id/role/iat/exp claims) via create_access_token()
  - Refresh token rotation (SHA-256-hashed, 14-day expiry, MongoDB TTL, one-time-use)
  - POST /auth/login endpoint — email+password -> JWT+refresh pair
  - POST /auth/refresh endpoint — rotates refresh token, issues new access token
  - Standalone get_current_tenant FastAPI dependency (replaces AuthService class)
  - Dual-mode auth: JWT bearer or API key (sk_live_...) on every protected endpoint
  - get_current_tenant_optional for unauthenticated-friendly endpoints
  - 29 passing auth API tests, 13 passing service unit tests

affects: [02-03, 02-04, 02-05, phase-03, phase-04, phase-05, phase-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - SHA-256 hash of raw refresh token stored in MongoDB for O(1) lookup (vs bcrypt which is unsearchable)
    - Token rotation: delete old refresh token before inserting new one — single atomic operation per refresh
    - Generic 401 "Invalid credentials" for all auth failures (wrong email, wrong password, inactive) — prevents email enumeration
    - JWT payload requires sub, tenant_id, role, iat, exp — validated via PyJWT options["require"]
    - get_current_tenant routes on sk_live_ prefix for API key vs JWT path

key-files:
  created: []
  modified:
    - apps/efofx-estimate/app/models/auth.py
    - apps/efofx-estimate/app/services/auth_service.py
    - apps/efofx-estimate/app/core/security.py
    - apps/efofx-estimate/app/api/auth.py
    - apps/efofx-estimate/app/api/routes.py
    - apps/efofx-estimate/tests/api/test_auth.py

key-decisions:
  - "SHA-256 (not bcrypt) for refresh token hashes — 384-bit entropy tokens are unguessable; SHA-256 enables O(1) MongoDB lookup"
  - "Refresh token rotation: old token deleted, new issued — any reuse of rotated token returns 401"
  - "user_id = tenant_id for single-user tenants (owner role) — simplifies single-tenant JWT claims"
  - "AuthService class fully removed from security.py — replaced by standalone get_current_tenant function"
  - "check_rate_limit removed from routes.py (dead code from removed RateLimiter) — slowapi replaces in plan 02-05"
  - "Test assertion for refresh token uniqueness changed from byte equality to JWT decode validation — same-second issuance produces identical tokens in sub-second test runs"

patterns-established:
  - "Refresh token storage: {token_hash: sha256hex, tenant_id, expires_at, created_at} with MongoDB TTL index on expires_at"
  - "create_access_token(tenant_id, user_id, role) -> str — pure function, no I/O"
  - "login_tenant raises 401 for any credential failure, 403 only for unverified email"
  - "All protected endpoints depend on get_current_tenant from app.core.security (not app.api.auth)"

requirements-completed: [AUTH-03, AUTH-04, AUTH-05]

# Metrics
duration: 7min
completed: 2026-02-26
---

# Phase 02 Plan 02: Auth — JWT Login and Refresh Token Flow Summary

**JWT login with refresh token rotation: SHA-256-hashed refresh tokens in MongoDB TTL collection, 20-min access tokens with required claims, dual-mode auth (JWT + API key) in standalone get_current_tenant dependency**

## Performance

- **Duration:** 7 min
- **Started:** 2026-02-26T23:08:53Z
- **Completed:** 2026-02-26T23:15:48Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments
- Implemented `login_tenant()` — email+password -> JWT access token + rotating refresh token; generic 401 prevents email enumeration
- Implemented `refresh_access_token()` — SHA-256 lookup O(1), delete-before-issue rotation, 14-day TTL with MongoDB expiry index
- Rewrote `get_current_tenant` as standalone FastAPI dependency supporting both JWT (with `require` claims validation) and API key (sk_live_ prefix fast lookup)
- Added POST /auth/login and POST /auth/refresh to auth router; updated /profile and /profile PATCH to use new Tenant-returning dependency
- 16 new tests covering all auth failure modes, JWT claim validation, token rotation, and API key auth; all 42 tests pass

## Task Commits

Each task was committed atomically:

1. **Task 1: JWT token service, get_current_tenant rewrite** - `14298c2` (feat)
2. **Task 2: Login/refresh endpoints and comprehensive tests** - `58f2d24` (feat)

## Files Created/Modified
- `apps/efofx-estimate/app/models/auth.py` — Added LoginRequest, LoginResponse, RefreshRequest, TokenResponse
- `apps/efofx-estimate/app/services/auth_service.py` — Added create_access_token, create_refresh_token, login_tenant, refresh_access_token
- `apps/efofx-estimate/app/core/security.py` — Removed AuthService/RateLimiter/check_rate_limit; new standalone get_current_tenant + get_current_tenant_optional
- `apps/efofx-estimate/app/api/auth.py` — Added POST /auth/login and POST /auth/refresh; updated profile endpoints to use Tenant type
- `apps/efofx-estimate/app/api/routes.py` — Removed dead check_rate_limit import and calls
- `apps/efofx-estimate/tests/api/test_auth.py` — 16 new tests; all 29 auth tests pass

## Decisions Made
- SHA-256 for refresh token hashes instead of bcrypt: refresh tokens have 384-bit entropy (token_urlsafe(48)) so they cannot be guessed; SHA-256 enables O(1) MongoDB lookup which bcrypt does not support
- Refresh token rotation: old token is deleted before new token is stored — any replay of a consumed token returns 401
- Generic 401 "Invalid credentials" for wrong email, wrong password, and inactive account — never reveals email existence
- user_id == tenant_id for single-user tenants — simplifies initial implementation; role = "owner"

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Removed dead `check_rate_limit` import from routes.py**
- **Found during:** Task 2 (running test suite)
- **Issue:** `app/api/routes.py` imported `check_rate_limit` from `app.core.security`, which was removed in Task 1 as part of the plan (RateLimiter class removal); this caused ImportError at test time
- **Fix:** Removed the `check_rate_limit` import and all four call sites in routes.py. The calls were no-ops for correctness (rate limiting delegated to slowapi in plan 02-05). Plan explicitly states removing `check_rate_limit` is safe
- **Files modified:** `apps/efofx-estimate/app/api/routes.py`
- **Verification:** Import error resolved; all 29 auth tests pass
- **Committed in:** `58f2d24` (Task 2 commit)

**2. [Rule 1 - Bug] Fixed test assertion for refresh token access token uniqueness**
- **Found during:** Task 2 (test run)
- **Issue:** `test_refresh_token_success` asserted new access token byte-differs from old access token. Since JWT iat/exp have second-level resolution and the refresh call completes in <1s, both tokens had identical timestamps and thus identical bytes
- **Fix:** Changed assertion from byte equality to JWT decode validation (verify token has tenant_id/sub/role claims); the real invariant is that the new token is valid and well-formed, not that it's byte-different
- **Files modified:** `apps/efofx-estimate/tests/api/test_auth.py`
- **Verification:** Test passes consistently
- **Committed in:** `58f2d24` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking import error, 1 test logic bug)
**Impact on plan:** Both auto-fixes necessary for correctness. No scope creep.

## Issues Encountered
None — all planned logic implemented without unexpected blockers.

## Next Phase Readiness
- JWT auth complete: every downstream endpoint can use `Depends(get_current_tenant)` from `app.core.security`
- Token refresh rotation is live: frontend can silently refresh before access token expiry
- Plan 02-04 (BYOK) can use `get_current_tenant` to get the authenticated Tenant for OpenAI key management
- Plan 02-05 (slowapi rate limiting) replaces the now-removed in-memory RateLimiter in routes.py

---
*Phase: 02-multi-tenant-foundation*
*Completed: 2026-02-26*
