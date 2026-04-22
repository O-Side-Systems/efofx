# Story 3.1: Create Tenant Registration & Management

Status: backlog

## Story

As a contractor (tenant),
I want to register for an efOfX account with email verification,
So that I can access the estimation service and manage my settings.

## Acceptance Criteria

**Given** the PRD specifies tenant registration with tiers
**When** I implement tenant management
**Then** the Pydantic model `app/models/tenant.py` includes:
- `_id`: ObjectId
- `name`: str (company name)
- `email`: str (unique, validated)
- `password_hash`: str (bcrypt)
- `tier`: Enum[trial, pro, enterprise]
- `api_key_hash`: str
- `openai_api_key_encrypted`: str | None
- `branding`: dict (logo_url, primary_color, widget_button_text)
- `rate_limit_tier`: str
- `is_active`: bool
- `created_at`, `updated_at`: datetime

**And** `POST /api/v1/tenants` endpoint creates tenant:
- Validates email format and uniqueness
- Hashes password with bcrypt (cost factor 12)
- Generates unique API key (hashed with bcrypt for storage)
- Sends verification email via SendGrid
- Returns tenant_id and API key (plaintext, only shown once)

**And** `PATCH /api/v1/tenants/{id}` updates tenant settings (name, branding, tier)

**And** `GET /api/v1/tenants/{id}` returns tenant info (excluding password_hash and openai_api_key_encrypted)

## Tasks / Subtasks

- [ ] Create Pydantic tenant model with all fields
- [ ] Create `POST /api/v1/tenants` endpoint
- [ ] Implement email validation and uniqueness check
- [ ] Implement bcrypt password hashing
- [ ] Generate and hash API keys
- [ ] Integrate SendGrid for verification emails
- [ ] Create `PATCH /api/v1/tenants/{id}` endpoint
- [ ] Create `GET /api/v1/tenants/{id}` endpoint
- [ ] Test registration flow end-to-end
- [ ] Test email verification

## Dev Notes

### Prerequisites

Story 1.4 (MongoDB connected)

### Technical Notes

- Never return plaintext API key after initial creation
- Store API key hash only (bcrypt), not reversible
- Email verification token expires after 24 hours
- Trial tier is default, no credit card required

### References

- [Source: docs/epics.md#Story-3-1]
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
