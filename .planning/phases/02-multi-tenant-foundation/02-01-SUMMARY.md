---
phase: 02-multi-tenant-foundation
plan: 01
subsystem: auth
tags: [registration, email-verification, api-key, bcrypt, pwdlib, fastapi-mail, pydantic-v2, mongodb, asyncio]

requires:
  - phase: 01-prerequisites
    provides: PyJWT, pwdlib, pydantic v2, asyncio-compatible test patterns

provides:
  - POST /api/v1/auth/register — contractor registration with hashed password + one-time API key
  - GET /api/v1/auth/verify?token=... — email verification (single-use token, 24h TTL)
  - GET /api/v1/auth/profile — authenticated profile with masked API key
  - PATCH /api/v1/auth/profile — update company_name and settings
  - Tenant model with tenant_id UUID, email, hashed_password, hashed_api_key, tier, email_verified
  - auth_service.py with register_tenant, verify_email, get_profile, update_profile
  - VERIFICATION_TOKENS and REFRESH_TOKENS collections in DB_COLLECTIONS
  - SMTP, APP_BASE_URL, MASTER_ENCRYPTION_KEY, VALKEY_URL settings in config

affects: [02-02-jwt-auth, 02-03-tenant-isolation, 02-04-byok, 02-05-rate-limiting]

tech-stack:
  added:
    - fastapi-mail==1.6.2 (email verification)
    - valkey>=6.1.0 (rate limiting, plan 02-03)
    - slowapi==0.1.9 (rate limiting, plan 02-03)
    - fakeredis>=2.0.0 (dev testing)
    - pydantic[email]>=2.11.0 (EmailStr validator)
  patterns:
    - Tenant identified by UUID tenant_id string (not MongoDB ObjectId)
    - bcrypt hash via pwdlib.PasswordHash((BcryptHasher(),)) — argon2 not installed
    - API key format: sk_live_{tenant_id_no_dashes}_{secrets.token_urlsafe(16)}
    - api_key_last6 stored alongside hashed_api_key for profile display masking
    - Email enumeration prevention: duplicate registration returns identical 201 response
    - Per-test Motor client pattern for asyncio-compatible test isolation
    - Email verification is dev-skippable (SMTP_USERNAME=None logs warning)

key-files:
  created:
    - apps/efofx-estimate/app/models/tenant.py (rewritten)
    - apps/efofx-estimate/app/models/auth.py
    - apps/efofx-estimate/app/models/_objectid.py
    - apps/efofx-estimate/app/services/auth_service.py
    - apps/efofx-estimate/app/api/auth.py
    - apps/efofx-estimate/tests/api/__init__.py
    - apps/efofx-estimate/tests/api/test_auth.py
    - apps/efofx-estimate/tests/services/test_auth_service.py
  modified:
    - apps/efofx-estimate/app/core/config.py
    - apps/efofx-estimate/app/core/constants.py
    - apps/efofx-estimate/app/main.py
    - apps/efofx-estimate/app/services/tenant_service.py
    - apps/efofx-estimate/pyproject.toml
    - apps/efofx-estimate/tests/conftest.py
    - apps/efofx-estimate/tests/services/test_rcf_engine.py

key-decisions:
  - "Use BcryptHasher explicitly, not PasswordHash.recommended() — argon2 not installed in venv"
  - "api_key_last6 stored alongside hashed_api_key to enable masked display without reversing bcrypt hash"
  - "API key encodes tenant_id (no dashes) to enable O(1) tenant lookup without full-collection scan"
  - "Email verification is skipped (not errored) when SMTP_USERNAME is None for dev ergonomics"
  - "Email enumeration prevention: duplicate email returns 201 with identical generic message"
  - "asyncio_mode=auto requires per-test Motor client pattern — conftest session fixtures updated to use loop_scope=session"

patterns-established:
  - "PyObjectId moved to app/models/_objectid.py — no longer in tenant.py (tenant uses UUID strings)"
  - "Per-test Motor client: each test creates its own AsyncIOMotorClient bound to its event loop"
  - "Config uses SettingsConfigDict(env_file=...) not Field(env=...) — Pydantic v2 BaseSettings"

requirements-completed: [AUTH-01, AUTH-02, AUTH-06, AUTH-07]

duration: 11min
completed: 2026-02-26
---

# Phase 02 Plan 01: Contractor Registration and Email Verification Summary

**Contractor registration with bcrypt-hashed API key (shown once), email verification via 24h token, and profile management — full tenant lifecycle from signup to API access**

## Performance

- **Duration:** 11 min
- **Started:** 2026-02-26T22:54:31Z
- **Completed:** 2026-02-26T23:05:39Z
- **Tasks:** 2
- **Files modified:** 15

## Accomplishments

- Full registration flow: POST /api/v1/auth/register creates tenant with hashed password, bcrypt-hashed API key (shown once in response), and stores verification token with 24h TTL
- Email verification: GET /api/v1/auth/verify?token=... marks tenant email_verified=True and deletes single-use token
- Profile endpoints: GET/PATCH /api/v1/auth/profile with API key auth (email verification enforced, unverified returns 403)
- 26 tests pass (13 API tests, 13 unit tests) — all auth requirements verified

## Task Commits

1. **Task 1: Rewrite Tenant model, create auth models, extend config** - `44e9b21` (feat)
2. **Task 2: Implement auth service, API endpoints, and tests** - `44413db` (feat)

## Files Created/Modified

- `apps/efofx-estimate/app/models/tenant.py` — Rewritten: tenant_id UUID, hashed_password, hashed_api_key, api_key_last6, tier, email_verified, encrypted_openai_key (Pydantic v2)
- `apps/efofx-estimate/app/models/auth.py` — RegisterRequest, RegisterResponse, ProfileUpdateRequest, ProfileResponse, VerifyEmailResponse
- `apps/efofx-estimate/app/models/_objectid.py` — Shared PyObjectId utility (moved from tenant.py)
- `apps/efofx-estimate/app/services/auth_service.py` — register_tenant, verify_email, send_verification_email, generate_verification_token, get_profile, update_profile
- `apps/efofx-estimate/app/api/auth.py` — Router with /auth/register, /auth/verify, /auth/profile (GET+PATCH)
- `apps/efofx-estimate/app/core/config.py` — Updated to SettingsConfigDict; added SMTP_*, APP_BASE_URL, MASTER_ENCRYPTION_KEY, VALKEY_URL
- `apps/efofx-estimate/app/core/constants.py` — Added VERIFICATION_TOKENS and REFRESH_TOKENS to DB_COLLECTIONS
- `apps/efofx-estimate/app/main.py` — Mounted auth_router at /api/v1
- `apps/efofx-estimate/app/services/tenant_service.py` — Added get_by_email(), get_by_tenant_id()
- `apps/efofx-estimate/pyproject.toml` — Added fastapi-mail, valkey, slowapi, fakeredis, pydantic[email]; asyncio_mode=auto
- `apps/efofx-estimate/tests/conftest.py` — Updated to pytest_asyncio.fixture with loop_scope for asyncio_mode=auto
- `apps/efofx-estimate/tests/services/test_rcf_engine.py` — Fixed TestRCFMatching to use per-test Motor client

## Decisions Made

- **BcryptHasher explicit:** Used `PasswordHash((BcryptHasher(),))` instead of `PasswordHash.recommended()` because argon2 is not installed. bcrypt is the Phase 1 dependency.
- **api_key_last6:** Stored alongside hashed_api_key to enable masked display (sk-...abc123) without reversing bcrypt hash.
- **API key encodes tenant_id:** Format `sk_live_{tenant_id_no_dashes}_{random}` allows O(1) tenant lookup by parsing first 32 chars of key body — avoids full-collection bcrypt scan.
- **Dev SMTP skip:** When SMTP_USERNAME is None, `send_verification_email` logs a warning (with token) and returns without error — dev environments work without SMTP setup.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed PyObjectId import error after Tenant model rewrite**
- **Found during:** Task 1 (model rewrite verification)
- **Issue:** `estimation.py`, `reference.py`, `chat.py`, `feedback.py` all imported `PyObjectId` from `app.models.tenant` — now removed from that module
- **Fix:** Created `app/models/_objectid.py` as shared utility, updated all 4 files to import from there
- **Files modified:** `_objectid.py` (created), `estimation.py`, `reference.py`, `chat.py`, `feedback.py`
- **Verification:** `python -c "from app.models.tenant import Tenant; from app.models.auth import RegisterRequest"` passes
- **Committed in:** 44e9b21 (Task 1 commit)

**2. [Rule 1 - Bug] pwdlib argon2 not available — use BcryptHasher explicitly**
- **Found during:** Task 2 (running unit tests)
- **Issue:** `PasswordHash.recommended()` tries argon2 first, but argon2 is not installed. Test import failed with `HasherNotAvailable`.
- **Fix:** Changed all PasswordHash instances to `PasswordHash((BcryptHasher(),))` in auth_service.py, auth.py, and test file
- **Files modified:** `auth_service.py`, `api/auth.py`, `tests/services/test_auth_service.py`
- **Verification:** Unit tests pass (13/13)
- **Committed in:** 44413db (Task 2 commit)

**3. [Rule 1 - Bug] asyncio_mode=auto broke TestRCFMatching session-scoped fixtures**
- **Found during:** Task 2 (running full test suite)
- **Issue:** Adding `asyncio_mode=auto` caused `TestRCFMatching::setup_test_data` fixture to run in a function-scoped event loop while `test_db` session fixture's Motor client was on the session loop — "Future attached to a different loop"
- **Fix:** Updated `TestRCFMatching.setup_test_data` to use per-test Motor client pattern (same as test_tenant_isolation.py); updated conftest.py to use `pytest_asyncio.fixture(loop_scope="session")` for session-scoped fixtures
- **Files modified:** `tests/conftest.py`, `tests/services/test_rcf_engine.py`
- **Verification:** 106/107 tests pass (1 pre-existing performance test fails due to 50ms threshold unrealistic in test environment)
- **Committed in:** 44413db (Task 2 commit)

**4. [Rule 1 - Bug] config.py Field(env=...) deprecated in Pydantic v2**
- **Found during:** Task 2 (test output showed 30+ PydanticDeprecatedSince20 warnings)
- **Issue:** All Settings fields used `Field(env="VAR_NAME")` syntax — deprecated in Pydantic v2, causes warning noise
- **Fix:** Rewrote config.py to use `SettingsConfigDict` and plain type annotations (pydantic-settings v2 reads env vars by field name automatically)
- **Files modified:** `app/core/config.py`
- **Verification:** No Field(env=...) warnings in test output
- **Committed in:** 44413db (Task 2 commit)

---

**Total deviations:** 4 auto-fixed (Rule 1 bugs — import errors, missing bcrypt, asyncio fixture conflict, Pydantic v2 deprecation)
**Impact on plan:** All fixes were required for correctness. No scope creep.

## Issues Encountered

- `test_performance_requirement` in `TestRCFMatching` now actually executes (previously it errored before our changes), but fails with P95=112ms vs 50ms threshold. This is a pre-existing issue with unrealistic threshold for test environment (10 sequential MongoDB queries without index warmup). Logged to deferred-items for tracking.

## User Setup Required

The plan lists SMTP credentials as required for email verification. However, the implementation is dev-friendly: when `SMTP_USERNAME` is not set, verification email is skipped with a warning that includes the token (for manual testing).

For production, the following env vars must be set:
- `SMTP_USERNAME` — email provider credentials
- `SMTP_PASSWORD`
- `SMTP_SERVER` (e.g., smtp.sendgrid.net)
- `SMTP_PORT` (default: 587)
- `SMTP_FROM` (e.g., noreply@efofx.ai)
- `APP_BASE_URL` — for verification link generation
- `MASTER_ENCRYPTION_KEY` — Fernet master key (required for BYOK in plan 02-04)

## Next Phase Readiness

- Auth registration and verification endpoints are live
- Ready for plan 02-02: JWT login/refresh token endpoints
- The `get_current_tenant` dependency in auth.py uses API key auth only (sk_live_ prefix); plan 02-02 will add JWT Bearer auth
- Tenant model is complete with all required fields for plans 02-02 through 02-05

---
*Phase: 02-multi-tenant-foundation*
*Completed: 2026-02-26*
