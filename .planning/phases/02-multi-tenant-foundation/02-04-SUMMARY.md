---
phase: 02-multi-tenant-foundation
plan: "04"
subsystem: auth
tags: [byok, fernet, hkdf, encryption, openai, cryptography]

requires:
  - phase: 02-multi-tenant-foundation/02-01
    provides: Tenant model with encrypted_openai_key field, MASTER_ENCRYPTION_KEY in Settings
  - phase: 02-multi-tenant-foundation/02-02
    provides: get_current_tenant dependency for JWT + API key auth
  - phase: 02-multi-tenant-foundation/02-03
    provides: get_database() for MongoDB access

provides:
  - app/utils/crypto.py with HKDF-SHA256 per-tenant Fernet key derivation and encrypt/decrypt/mask utilities
  - app/services/byok_service.py with validate_and_store, rotate, decrypt, and status functions
  - PUT /auth/openai-key endpoint for key storage and rotation (validates against OpenAI before storing)
  - GET /auth/openai-key/status endpoint for masked key display without decryption
  - 402 gate via decrypt_tenant_openai_key when no key is stored (no platform key fallback)

affects:
  - llm-service
  - estimation-service
  - any endpoint calling LLM features (must call decrypt_tenant_openai_key to get plaintext key)

tech-stack:
  added:
    - cryptography>=41.0.0 (already in requirements.txt): Fernet, HKDF-SHA256
  patterns:
    - HKDF-SHA256 per-tenant key derivation with info="efofx-byok-{tenant_id}"
    - Fernet authenticated encryption for at-rest storage; plaintext only in request scope
    - openai_key_last6 stored alongside ciphertext for O(0) masked display (no decryption needed)
    - OpenAI key validation via models.list() before any storage (fail fast)
    - 402 Payment Required as the gate for missing BYOK key (no platform fallback)

key-files:
  created:
    - apps/efofx-estimate/app/utils/crypto.py
    - apps/efofx-estimate/app/services/byok_service.py
    - apps/efofx-estimate/tests/utils/__init__.py
    - apps/efofx-estimate/tests/utils/test_crypto.py
    - apps/efofx-estimate/tests/api/test_byok.py
  modified:
    - apps/efofx-estimate/app/models/tenant.py (added openai_key_last6 field)
    - apps/efofx-estimate/app/models/auth.py (added StoreOpenAIKeyRequest/Response, OpenAIKeyStatusResponse)
    - apps/efofx-estimate/app/api/auth.py (added PUT /auth/openai-key, GET /auth/openai-key/status)

key-decisions:
  - "HKDF info string scoped to efofx-byok-{tenant_id} — per-tenant key derivation limits blast radius if master key is compromised"
  - "openai_key_last6 stored alongside ciphertext for masked display without decryption"
  - "PUT /auth/openai-key handles both initial store and rotation (simple overwrite, no version history)"
  - "402 Payment Required is the correct gate for missing BYOK key — no platform key fallback per locked decision"
  - "OpenAI key validated via models.list() before encryption/storage — lightweight, no tokens burned"

patterns-established:
  - "Crypto pattern: derive_tenant_fernet_key(master_bytes, tenant_id) -> per-tenant Fernet instance (deterministic)"
  - "Service gate pattern: decrypt_tenant_openai_key raises 402 (not 404) when key absent — signals payment/setup action needed"
  - "Display pattern: openai_key_last6 field avoids any decryption for UI display — always read from stored field"
  - "Rotation pattern: validate_and_store_openai_key() handles both initial store and rotation — single function, immediate overwrite"

requirements-completed:
  - BYOK-01
  - BYOK-02
  - BYOK-03
  - BYOK-04

duration: 5min
completed: 2026-02-26
---

# Phase 02 Plan 04: BYOK Fernet Encryption with HKDF Per-Tenant Derivation Summary

**Per-tenant HKDF-SHA256 Fernet encryption for BYOK OpenAI API keys with validate-before-store, masked display, and 402 gate — 20 tests passing, OpenAI calls fully mocked**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-26T23:25:49Z
- **Completed:** 2026-02-26T23:30:19Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- Implemented HKDF-SHA256 per-tenant Fernet key derivation in `crypto.py` — deterministic, isolated per tenant, each tenant gets a unique derived key
- Built `byok_service.py` with validate-before-store pattern (OpenAI models.list() called before encryption), key rotation (immediate overwrite), decryption gate (402 when no key), and masked status (no decryption needed)
- Added PUT /auth/openai-key and GET /auth/openai-key/status endpoints with proper auth guards (401 unauthenticated, 403 unverified)
- 20 tests pass: 9 pure unit tests (crypto round-trip, per-tenant isolation, HKDF determinism, masking) + 11 endpoint tests (success, invalid key, OpenAI unavailable, rotation, 402 gate, status)

## Task Commits

1. **Task 1: TDD — Implement crypto utilities and BYOK service with tests** - `b245c68` (feat)
2. **Task 2: Add BYOK endpoint and write integration tests** - `2f4b231` (feat)

**Plan metadata:** _(final docs commit below)_

## Files Created/Modified

- `apps/efofx-estimate/app/utils/crypto.py` - HKDF-SHA256 per-tenant Fernet derivation, encrypt/decrypt/mask utilities
- `apps/efofx-estimate/app/services/byok_service.py` - BYOK service: validate, store, rotate, decrypt, status
- `apps/efofx-estimate/app/models/tenant.py` - Added `openai_key_last6: Optional[str]` field
- `apps/efofx-estimate/app/models/auth.py` - Added StoreOpenAIKeyRequest, StoreOpenAIKeyResponse, OpenAIKeyStatusResponse
- `apps/efofx-estimate/app/api/auth.py` - Added PUT /auth/openai-key and GET /auth/openai-key/status endpoints
- `apps/efofx-estimate/tests/utils/__init__.py` - New package init
- `apps/efofx-estimate/tests/utils/test_crypto.py` - 9 unit tests for crypto utilities
- `apps/efofx-estimate/tests/api/test_byok.py` - 11 endpoint tests for BYOK key management

## Decisions Made

- Used HKDF info string `"efofx-byok-{tenant_id}"` for per-tenant derivation — consistent with research plan pattern and limits blast radius
- Stored `openai_key_last6` alongside ciphertext in tenant document — avoids decryption on status/profile reads
- PUT endpoint handles both initial store and rotation (same code path — simple overwrite per locked decision)
- 402 Payment Required for missing BYOK key — signals contractors need to configure their key before using LLM features
- AuthenticationError mock required constructing minimal error object (httpx Response not needed for mocking)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added `openai_key_last6` field to Tenant and TenantCreate models**
- **Found during:** Task 1 (byok_service.py implementation)
- **Issue:** Plan noted this field was not in 02-01 and needed to be added for masked display without decryption
- **Fix:** Added `openai_key_last6: Optional[str] = None` to both `Tenant` and `TenantCreate` models
- **Files modified:** `apps/efofx-estimate/app/models/tenant.py`
- **Verification:** Field visible in tenant documents after key storage; GET /auth/openai-key/status reads it correctly
- **Committed in:** b245c68 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 2 — missing critical field)
**Impact on plan:** The field was explicitly noted as needing addition in the plan. No scope creep.

## Issues Encountered

- AuthenticationError from openai v2 requires special constructor handling for mocking — used `AuthenticationError.__new__(AuthenticationError)` with manual attribute assignment. All 11 endpoint tests pass.

## User Setup Required

None — no external service configuration required. `MASTER_ENCRYPTION_KEY` was already in the Settings model from plan 02-01.

## Next Phase Readiness

- BYOK key storage, validation, and 402 gate are complete
- LLM endpoints (estimation, chat) can now call `decrypt_tenant_openai_key(tenant_id)` to get the per-request plaintext key
- Next: 02-05 (Rate limiting with slowapi)
- Blockers: None

---
*Phase: 02-multi-tenant-foundation*
*Completed: 2026-02-26*
