# Story 3.3: Implement BYOK Encryption for OpenAI Keys

Status: backlog

## Story

As a contractor (tenant),
I want to store my OpenAI API key securely encrypted,
So that LLM requests use my key and I control my usage costs.

## Acceptance Criteria

**Given** the architecture specifies AES-256 Fernet encryption for BYOK
**When** I implement BYOK storage
**Then** `app/core/security.py` contains:
- `encrypt_openai_key(plaintext_key, tenant_id)` function using Fernet
- `decrypt_openai_key(tenant_id)` function (decrypts at request time only)
- Encryption key loaded from environment variable (ENCRYPTION_KEY)
- Keys never logged or transmitted in plaintext

**And** `POST /api/v1/tenants/{id}/openai-key` endpoint:
- Accepts OpenAI API key in request body
- Validates key format (starts with "sk-")
- Tests key with simple OpenAI API call (validation)
- Encrypts key with Fernet
- Stores encrypted key in tenant document
- Returns success (does not echo key back)

**And** when LLM service needs key
**Then** it calls `decrypt_openai_key(tenant_id)` to get plaintext in memory

**And** when key is invalid/expired
**Then** API returns 402 Payment Required with helpful error message

## Tasks / Subtasks

- [ ] Implement `encrypt_openai_key()` function using Fernet
- [ ] Implement `decrypt_openai_key()` function
- [ ] Create `POST /api/v1/tenants/{id}/openai-key` endpoint
- [ ] Add OpenAI key format validation
- [ ] Add OpenAI key validation test (API call)
- [ ] Store encrypted key in tenant document
- [ ] Test encryption/decryption cycle
- [ ] Test error handling for invalid keys
- [ ] Audit log all key changes

## Dev Notes

### Prerequisites

Story 3.1 (tenant model has openai_api_key_encrypted field)

### Technical Notes

- Generate Fernet key: `python -c "from cryptography.fernet import Fernet; print(Fernet.generate_key().decode())"`
- Store ENCRYPTION_KEY in DO environment variables (encrypted at rest)
- Trial tier can use platform key (fallback), Pro/Enterprise require BYOK
- Audit log all key changes (encrypt, update, remove)

### References

- [Source: docs/epics.md#Story-3-3]
- [Source: docs/PRD.md] (for requirements context)

## Dev Agent Record

### Context Reference

<!-- Path(s) to story context XML will be added here by context workflow -->

### Agent Model Used

<!-- To be filled by dev agent -->

### Debug Log References

<!-- To be filled by dev agent during implementation -->

### Completion Notes List

<!-- To be filled by dev agent upon completion -->

### File List

<!-- NEW/MODIFIED/DELETED files will be listed here by dev agent -->
