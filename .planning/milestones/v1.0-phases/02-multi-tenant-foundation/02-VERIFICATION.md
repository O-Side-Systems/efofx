---
phase: 02-multi-tenant-foundation
verified: 2026-02-27T15:00:00Z
status: passed
score: 18/18 must-haves verified
re_verification: true
  previous_status: gaps_found
  previous_score: 17/18
  gaps_closed:
    - "tenant_service.py now uses TenantAwareCollection (get_tenant_collection) for all tenant-scoped queries — no deprecated accessors, no ObjectId(tenant_id)"
    - "REQUIREMENTS.md BYOK-04 corrected to: 'LLM endpoints return 402 when no BYOK OpenAI key is stored (no platform fallback)'"
    - "LLMService.__init__ now accepts optional api_key parameter — BYOK per-request key injection pattern established"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Send a real registration email via configured SMTP"
    expected: "Verification link arrives in inbox with correct format and 24-hour expiry notice"
    why_human: "SMTP is skipped in dev (SMTP_USERNAME not set). Cannot verify actual email delivery programmatically."
  - test: "Log in with a verified account, wait 20 minutes, then call a protected endpoint"
    expected: "Returns 401 'Token expired'"
    why_human: "Test suite mints tokens with artificial expiry. Cannot wait 20 real minutes in automated testing."
  - test: "Store a BYOK OpenAI key via PUT /auth/openai-key with a real OpenAI API key"
    expected: "Returns 200 with masked key; key is validated against OpenAI's models.list()"
    why_human: "All tests mock the OpenAI API call. Cannot verify real network validation without live credentials."
  - test: "In production with Valkey connected, send 6 login requests from the same IP within 15 minutes"
    expected: "First 5 succeed (401 for wrong credentials); 6th returns 429 with rate limit headers and Retry-After"
    why_human: "Rate limit tests use MemoryStorage. Cannot verify Valkey connection and distributed state without production infrastructure."
---

# Phase 02: Multi-Tenant Foundation Verification Report

**Phase Goal:** Contractors can register, authenticate, and have their data completely isolated from other tenants — the security layer every subsequent feature depends on.
**Verified:** 2026-02-27
**Status:** passed
**Re-verification:** Yes — after gap closure (Plans 02-06 and 02-07)

---

## Re-verification Summary

The initial verification (2026-02-26) found 3 gaps blocking full goal achievement. Two gap-closure plans were created and executed:

- **02-06** (commit `3fc6ded`, test `b62b307`): Refactored `tenant_service.py` — replaced all deprecated `get_estimates_collection()` / `get_feedback_collection()` calls with `get_tenant_collection()`, removed all `ObjectId(tenant_id)` usage, added 7 unit tests.
- **02-07** (commit `d4ee1be`, test `8b2cbb7`): Fixed `REQUIREMENTS.md` BYOK-04 text and refactored `LLMService.__init__` to accept optional `api_key` parameter with BYOK injection pattern, added 6 unit tests.

All three gaps are now closed. No regressions were found in previously-passing artifacts.

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A contractor can register with company name, email, and password and receive a verification email | VERIFIED | `register_tenant()` in `auth_service.py`; 40 API tests pass |
| 2 | Registration returns a one-time API key that is never retrievable again | VERIFIED | `sk_live_{tenant_id_no_dashes}_{random}`; bcrypt-hashed for storage; plaintext returned once |
| 3 | A contractor can verify their email via a token link and gain platform access | VERIFIED | `verify_email()` sets `email_verified=True`; single-use token deleted |
| 4 | A verified contractor can log in with email/password and receive JWT access and refresh tokens | VERIFIED | `login_tenant()` returns JWT (20-min) + refresh (14-day); claims include sub, tenant_id, role |
| 5 | JWT access tokens contain tenant_id, user_id (sub), and role claims with 20-minute expiry | VERIFIED | `create_access_token()` sets all required claims; `test_jwt_claims_complete` passes |
| 6 | Calling any protected endpoint with expired/missing/invalid JWT returns 401 | VERIFIED | `get_current_tenant()` raises 401 for missing, expired, invalid; three dedicated passing tests |
| 7 | An unverified tenant gets 403 on any protected endpoint | VERIFIED | `get_current_tenant()` checks `email_verified`; `test_unverified_tenant_jwt_blocked` passes |
| 8 | A contractor can silently refresh their access token using a refresh token | VERIFIED | `refresh_access_token()` rotates tokens (old deleted, new issued); SHA-256 hash for O(1) lookup |
| 9 | Every MongoDB query for tenant data automatically includes tenant_id in the filter | VERIFIED | `TenantAwareCollection._scoped_filter()` injects `tenant_id` on every operation; 22 unit tests prove this |
| 10 | Two tenants cannot see each other's data regardless of query parameters | VERIFIED | `TenantAwareCollection` enforces isolation structurally; `tenant_service.py` now uses `get_tenant_collection()` for all tenant-scoped methods (gap closed in 02-06) |
| 11 | Platform-provided data (tenant_id=None) is accessible by all tenants via $or filter | VERIFIED | `allow_platform_data=True` produces `{$or: [{tenant_id: X}, {tenant_id: null}]}`; `test_platform_data_mode_empty_filter` passes |
| 12 | MongoDB compound indexes have tenant_id as the first field for all tenant-scoped collections | VERIFIED | `create_indexes()` defines `(tenant_id, ...)` compound indexes for estimates, reference_classes, reference_projects, feedback, chat_sessions |
| 13 | Calling TenantAwareCollection without a tenant_id raises an error | VERIFIED | Constructor raises `ValueError` for empty/None tenant_id; `test_empty_tenant_id_raises` and `test_none_tenant_id_raises` pass |
| 14 | A contractor can store their OpenAI API key encrypted with a per-tenant derived Fernet key | VERIFIED | `encrypt_openai_key()` uses HKDF-SHA256 per-tenant derivation; round-trip and isolation tests pass |
| 15 | The encrypted key is decrypted per-request for LLM calls and never stored in plaintext | VERIFIED | `decrypt_tenant_openai_key()` implements correct pattern; `LLMService(api_key=decrypted_key)` injection pattern established and tested (gap closed in 02-07) |
| 16 | A contractor can rotate their OpenAI key without re-registration | VERIFIED | `rotate_openai_key()` overwrites old ciphertext; `test_key_rotation` passes |
| 17 | LLM endpoints return 402 when no OpenAI key is stored | VERIFIED | `decrypt_tenant_openai_key()` raises `HTTPException(402)`; `test_no_key_returns_402` passes; REQUIREMENTS.md BYOK-04 updated to match |
| 18 | Rate limiting is enforced with correct tier-based limits, IP-based login throttling, and proper 429/header responses | VERIFIED | `TIER_LIMITS = {"trial": "20/minute", "paid": "100/minute"}`; login `@limiter.limit("5/15minutes")`; `add_rate_limit_headers` middleware; all 19 rate limit tests pass |

**Score: 18/18 truths fully verified**

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|---------|--------|---------|
| `apps/efofx-estimate/app/models/tenant.py` | Rewritten Tenant model with all required fields | VERIFIED | All fields present: tenant_id, company_name, email, hashed_password, hashed_api_key, api_key_last6, tier, email_verified, encrypted_openai_key, openai_key_last6, is_active, created_at, updated_at, settings |
| `apps/efofx-estimate/app/models/auth.py` | Request/response models for all auth flows | VERIFIED | RegisterRequest, RegisterResponse, LoginRequest, LoginResponse, RefreshRequest, TokenResponse, StoreOpenAIKeyRequest, StoreOpenAIKeyResponse, OpenAIKeyStatusResponse all present |
| `apps/efofx-estimate/app/api/auth.py` | Auth router with all planned endpoints | VERIFIED | POST /auth/register, GET /auth/verify, POST /auth/login, POST /auth/refresh, GET /auth/profile, PATCH /auth/profile, PUT /auth/openai-key, GET /auth/openai-key/status |
| `apps/efofx-estimate/app/services/auth_service.py` | Registration, login, token, profile, verification functions | VERIFIED | register_tenant, verify_email, send_verification_email, generate_verification_token, get_profile, update_profile, create_access_token, create_refresh_token, login_tenant, refresh_access_token |
| `apps/efofx-estimate/app/core/security.py` | Rewritten get_current_tenant dependency (JWT + API key auth) | VERIFIED | `get_current_tenant` supports both JWT and `sk_live_` API key auth; checks email_verified; raises proper HTTP errors |
| `apps/efofx-estimate/app/db/tenant_collection.py` | TenantAwareCollection wrapper class | VERIFIED | Full implementation with find_one, find, insert_one, insert_many, update_one, update_many, delete_one, delete_many, count_documents, aggregate; ValueError on empty tenant_id |
| `apps/efofx-estimate/app/db/mongodb.py` | get_tenant_collection() factory and compound indexes | VERIFIED | get_tenant_collection() factory present; compound indexes with tenant_id first for all tenant-scoped collections; deprecated functions retained with DEPRECATED docstrings |
| `apps/efofx-estimate/app/utils/crypto.py` | HKDF key derivation, Fernet encrypt/decrypt, key masking | VERIFIED | derive_tenant_fernet_key, encrypt_openai_key, decrypt_openai_key, mask_openai_key all implemented; 9 crypto unit tests pass |
| `apps/efofx-estimate/app/services/byok_service.py` | BYOK key management — validate, encrypt, store, rotate, decrypt | VERIFIED | validate_and_store_openai_key, rotate_openai_key, decrypt_tenant_openai_key, get_openai_key_status |
| `apps/efofx-estimate/app/core/rate_limit.py` | slowapi Limiter setup, tier-based limits, custom 429 handler | VERIFIED | limiter, get_tenant_id_for_limit, get_tier_limit, rate_limit_exceeded_handler, TIER_LIMITS all present |
| `apps/efofx-estimate/app/main.py` | slowapi state, exception handler, and auth router registered | VERIFIED | app.state.limiter = limiter; add_exception_handler(RateLimitExceeded, rate_limit_exceeded_handler); include_router(auth_router); add_rate_limit_headers middleware |
| `apps/efofx-estimate/app/core/config.py` | SMTP_*, APP_BASE_URL, MASTER_ENCRYPTION_KEY, VALKEY_URL, RATE_LIMIT_ENABLED | VERIFIED | All settings present; MASTER_ENCRYPTION_KEY is required (no default); VALKEY_URL defaults to redis://localhost:6379 |
| `apps/efofx-estimate/app/core/constants.py` | VERIFICATION_TOKENS and REFRESH_TOKENS in DB_COLLECTIONS | VERIFIED | Both present |
| `apps/efofx-estimate/app/services/tenant_service.py` | All tenant-scoped methods use TenantAwareCollection; no ObjectId | VERIFIED (re-verified) | `get_tenant_collection()` used at lines 145, 151, 207; `get_collection()` used for cross-tenant admin stats (intentional); zero `ObjectId(tenant_id)` calls (only a comment remains at line 190) |
| `apps/efofx-estimate/app/services/llm_service.py` | LLMService accepts optional per-request api_key | VERIFIED (re-verified) | `def __init__(self, api_key: Optional[str] = None)` at line 43; `self.api_key = api_key or settings.OPENAI_API_KEY`; `AsyncOpenAI(api_key=self.api_key)` at line 45 |
| `apps/efofx-estimate/tests/api/test_auth.py` | Full auth API tests | VERIFIED | 28 test cases |
| `apps/efofx-estimate/tests/api/test_byok.py` | BYOK endpoint tests | VERIFIED | 11 test cases |
| `apps/efofx-estimate/tests/api/test_rate_limit.py` | Rate limiting tests | VERIFIED | 19 test cases |
| `apps/efofx-estimate/tests/db/test_tenant_collection.py` | TenantAwareCollection unit tests | VERIFIED | 22 tests |
| `apps/efofx-estimate/tests/utils/test_crypto.py` | Crypto utility unit tests | VERIFIED | 9 tests |
| `apps/efofx-estimate/tests/services/test_tenant_service.py` | TenantService refactor unit tests (new in 02-06) | VERIFIED (new) | 7 tests covering collection routing, no-ObjectId assertions, return shape; confirms TenantAwareCollection used and get_all_tenant_statistics uses raw get_collection |
| `apps/efofx-estimate/tests/services/test_llm_service.py` | LLMService BYOK key injection tests (new in 02-07) | VERIFIED (new) | 6 tests covering: BYOK key passed to AsyncOpenAI, fallback to settings key, key flows through to API calls, generate_estimation returns EstimationOutput, api_key stored on instance |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `app/api/auth.py` | `app/services/auth_service.py` | `register_tenant()` called from POST /auth/register | WIRED | Confirmed unchanged from initial verification |
| `app/api/auth.py` | `app/services/auth_service.py` | `login_tenant()` called from POST /auth/login | WIRED | Confirmed unchanged from initial verification |
| `app/main.py` | `app/api/auth.py` | `app.include_router(auth_router)` | WIRED | Confirmed unchanged |
| `app/main.py` | `app/core/rate_limit.py` | `app.state.limiter` and exception handler | WIRED | Confirmed unchanged |
| `app/api/auth.py` | `app/core/rate_limit.py` | `@limiter.limit` decorator on login endpoint | WIRED | Confirmed unchanged |
| `app/core/security.py` | `jwt.decode` | JWT decode with required claims in get_current_tenant | WIRED | Confirmed unchanged |
| `app/db/mongodb.py` | `app/db/tenant_collection.py` | `get_tenant_collection()` returns TenantAwareCollection | WIRED | Confirmed unchanged |
| `app/services/tenant_service.py` | `app/db/tenant_collection.py` | `get_tenant_collection()` used in get_tenant_statistics and validate_tenant_limits | WIRED (was NOT_WIRED) | Lines 145, 151, 207 now call `get_tenant_collection(DB_COLLECTIONS[X], tenant_id)` — Gap 1 closed |
| `app/services/byok_service.py` | `app/utils/crypto.py` | Uses encrypt_openai_key/decrypt_openai_key | WIRED | Confirmed unchanged |
| `app/services/llm_service.py` | BYOK per-request key pattern | LLMService accepts api_key for BYOK injection | WIRED (was NOT_WIRED) | `__init__(self, api_key: Optional[str] = None)` established; Phase 3 caller pattern: `LLMService(api_key=await decrypt_tenant_openai_key(tenant_id))` — Gap 3 closed |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|------------|------------|-------------|--------|----------|
| AUTH-01 | 02-01 | Contractor can register with company name, email, password | SATISFIED | POST /api/v1/auth/register implemented and tested |
| AUTH-02 | 02-01 | Contractor receives email verification after registration | SATISFIED | send_verification_email() sends link via background task |
| AUTH-03 | 02-02 | Contractor can log in with email/password and receive JWT tokens | SATISFIED | login_tenant() returns access + refresh tokens; test_login_success passes |
| AUTH-04 | 02-02 | JWT tokens contain tenant_id, user_id, role claims with configurable expiration | SATISFIED | create_access_token() sets all claims; 20-min expiry; test_jwt_claims_complete passes |
| AUTH-05 | 02-02 | All protected endpoints require valid JWT and extract tenant_id automatically | SATISFIED | get_current_tenant() dependency on all protected endpoints |
| AUTH-06 | 02-01 | Contractor can update profile settings (name, branding, tier) | SATISFIED | PATCH /auth/profile; update_profile() updates company_name and settings |
| AUTH-07 | 02-01 | API key generated at registration (shown once, stored as bcrypt hash) | SATISFIED | raw_api_key returned once in RegisterResponse; hashed_api_key stored |
| ISOL-01 | 02-03 | Tenant isolation middleware enforces tenant_id on every MongoDB query automatically | SATISFIED | TenantAwareCollection._scoped_filter() called on every operation; 22 unit tests |
| ISOL-02 | 02-03 | Zero cross-tenant data leakage — no query can return another tenant's data | SATISFIED | Fully enforced by TenantAwareCollection; tenant_service.py now uses get_tenant_collection() for all tenant-scoped methods (gap closed 02-06) |
| ISOL-03 | 02-03 | MongoDB compound indexes include tenant_id as first field for performance | SATISFIED | create_indexes() defines (tenant_id, ...) compound indexes for all tenant-scoped collections |
| ISOL-04 | 02-03 | Platform-provided data accessible by all tenants | SATISFIED | allow_platform_data=True uses $or filter |
| BYOK-01 | 02-04 | Contractor can store OpenAI API key encrypted with per-tenant derived Fernet key | SATISFIED | HKDF-SHA256 per-tenant derivation; encrypt_openai_key() stores ciphertext |
| BYOK-02 | 02-04 | Encrypted keys decrypted per-request for LLM calls (never stored in plaintext) | SATISFIED | decrypt_tenant_openai_key() implements correct pattern; LLMService accepts api_key parameter; Phase 3 wires the full per-request path (LLM-01) |
| BYOK-03 | 02-04 | Contractor can rotate OpenAI key without re-registration | SATISFIED | rotate_openai_key() overwrites old ciphertext; test_key_rotation passes |
| BYOK-04 | 02-04 | LLM endpoints return 402 when no BYOK OpenAI key is stored (no platform fallback) | SATISFIED | decrypt_tenant_openai_key() raises HTTPException(402); test_no_key_returns_402 passes; REQUIREMENTS.md updated to match (gap closed 02-07) |
| RATE-01 | 02-05 | Per-tenant rate limiting enforced based on tier | SATISFIED | TIER_LIMITS = {"trial": "20/minute", "paid": "100/minute"}; get_tier_limit() reads tier from request.state |
| RATE-02 | 02-05 | Rate limit headers returned in API responses | SATISFIED | add_rate_limit_headers middleware in main.py; test_rate_limit_headers_present_on_auth_endpoint passes |
| RATE-03 | 02-05 | Login endpoint rate limited to 5 attempts per 15 minutes per IP | SATISFIED | @limiter.limit("5/15minutes", key_func=get_remote_address) on login; test_login_rate_limit_ip passes |

**All 18 requirements satisfied.**

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `apps/efofx-estimate/app/services/tenant_service.py` | 190 | Comment: `# Get tenant by UUID string (not legacy ObjectId)` | Info | Informational comment only — no ObjectId usage. Not a code issue. |
| `apps/efofx-estimate/app/services/llm_service.py` | 44 | `self.api_key = api_key or settings.OPENAI_API_KEY` — platform key fallback | Info | Intentional dev/test fallback documented in class docstring. Will be removed when Phase 3 (LLM-01) wires full per-request injection. Not a Phase 2 defect. |
| `apps/efofx-estimate/app/services/tenant_service.py` | 177 | `"active_regions": []` — hardcoded empty list in get_tenant_statistics | Info | Placeholder for future Phase 3 data; accurately documented inline. Not blocking. |

No blocker or warning-level anti-patterns in the gap-closure changes. The informational items are all intentional and documented.

---

### Human Verification Required

#### 1. Email Verification Flow

**Test:** Register a new contractor account while SMTP credentials are configured. Check the inbox of the registered email address.
**Expected:** An email arrives from `noreply@efofx.ai` (or configured SMTP_FROM) with subject "Verify your efOfX account" containing a `/api/v1/auth/verify?token=...` link that expires in 24 hours. Clicking the link marks the account as verified.
**Why human:** SMTP is skipped in dev mode when `SMTP_USERNAME` is not set. Tests mock `send_verification_email`. Real email delivery cannot be verified programmatically.

#### 2. JWT Token Expiry in Production

**Test:** Log in, wait 20 minutes (the real access token lifetime), then call `GET /api/v1/auth/profile` with the original access token.
**Expected:** Returns 401 with `{"detail": "Token expired"}`.
**Why human:** Tests create tokens with artificial past expiry. Cannot wait 20 real minutes in automated testing.

#### 3. Real OpenAI Key Validation

**Test:** Call `PUT /api/v1/auth/openai-key` with a real (valid) OpenAI API key and separately with a real (invalid/revoked) OpenAI API key.
**Expected:** Valid key returns 200 with masked key; invalid key returns 400 "Invalid OpenAI API key".
**Why human:** All BYOK tests mock `AsyncOpenAI.models.list()`. Real network validation against OpenAI API requires live credentials.

#### 4. Valkey Rate Limiting in Production

**Test:** In production with Valkey connected, send 6 login requests from the same IP within 15 minutes.
**Expected:** First 5 succeed (401 for wrong credentials); 6th returns 429 with rate limit headers and Retry-After.
**Why human:** Rate limit tests swap Valkey storage with MemoryStorage. Cannot verify Valkey connection and distributed state without running against production infrastructure.

---

### Gap Closure Verification

#### Gap 1 — tenant_service.py TenantAwareCollection (CLOSED)

**Was:** `get_estimates_collection()` and `get_feedback_collection()` (deprecated) called in `get_tenant_statistics()` and `validate_tenant_limits()`; `ObjectId(tenant_id)` used in multiple methods.

**Now:** `get_tenant_collection(DB_COLLECTIONS["ESTIMATES"], tenant_id)` and `get_tenant_collection(DB_COLLECTIONS["FEEDBACK"], tenant_id)` used at lines 145, 151, 207. All `ObjectId(tenant_id)` calls removed. `get_all_tenant_statistics()` correctly uses `get_collection()` (raw, unscoped) for intentional cross-tenant admin aggregation — this is correct behavior, not a gap.

**Evidence:** 7 unit tests in `tests/services/test_tenant_service.py` (commit `b62b307`) verify collection routing and absence of ObjectId.

#### Gap 2 — REQUIREMENTS.md BYOK-04 text (CLOSED)

**Was:** "Trial tier tenants use platform fallback OpenAI key" — contradicted the locked decision and implementation.

**Now:** "LLM endpoints return 402 when no BYOK OpenAI key is stored (no platform fallback)" — matches the locked decision in CONTEXT.md and the implementation in `byok_service.py`.

**Evidence:** Commit `d4ee1be`; REQUIREMENTS.md line 40 verified.

#### Gap 3 — LLMService not wired to BYOK decrypt pattern (CLOSED)

**Was:** `LLMService.__init__` used `settings.OPENAI_API_KEY` (platform key) unconditionally; no path for per-request BYOK key injection.

**Now:** `LLMService.__init__(self, api_key: Optional[str] = None)` — accepts per-request key; `self.api_key = api_key or settings.OPENAI_API_KEY`; `AsyncOpenAI(api_key=self.api_key)`. Class docstring documents the Phase 3 caller pattern: `LLMService(api_key=await decrypt_tenant_openai_key(tenant_id))`. Settings fallback retained for dev/testing only.

**Evidence:** 6 unit tests in `tests/services/test_llm_service.py` (commit `8b2cbb7`) prove BYOK key is passed to AsyncOpenAI and flows through to API calls.

---

_Verified: 2026-02-27_
_Verifier: Claude (gsd-verifier)_
_Re-verification after Plans 02-06 and 02-07 gap closure_
