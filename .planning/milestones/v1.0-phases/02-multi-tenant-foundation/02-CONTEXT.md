# Phase 2: Multi-Tenant Foundation - Context

**Gathered:** 2026-02-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Tenant registration, JWT authentication, hard data isolation via TenantAwareCollection, BYOK encryption for OpenAI keys, and per-tenant rate limiting with Valkey backend. This is the security layer every subsequent feature depends on. No billing, no dashboard UI, no team/role management — just the core multi-tenant infrastructure.

</domain>

<decisions>
## Implementation Decisions

### Registration & Onboarding
- Minimal signup: company name, email, password only — collect more later in profile settings
- Email verification required before any platform access (verify-before-access)
- Every new registration starts on trial tier automatically — no plan selection at signup
- Single user per tenant — no team members or role management in Phase 2

### Auth Error Behavior
- Generic error messages for all security-sensitive operations — "Invalid credentials" for login failures, "If an account exists, we sent a link" for password reset
- Never reveal whether an email is registered (prevent enumeration)
- Silent JWT refresh: short-lived access tokens (15-30 min), long-lived refresh tokens (7-30 days) — user stays logged in seamlessly
- Time-based lockout recovery: locked out for 15 minutes after too many attempts, then can retry — password reset always available as escape hatch

### BYOK Key Management
- Block all LLM features until contractor stores their OpenAI API key — no platform key fallback, no trial usage
- Validate key on save: make lightweight OpenAI API call (e.g., list models) to confirm key is valid before storing
- Simple overwrite for key rotation: new key replaces old immediately, old encrypted blob deleted, no version history
- Show masked key in settings: last 6 characters visible (sk-...abc123) so contractor can confirm which key is stored

### Tenant Tiers & Rate Limits
- Two tiers only: trial and paid — simple structure, easy to extend later
- Primary rate limit: API calls per minute per tenant
- Login brute-force protection: 5 attempts per 15 minutes per IP (per success criteria)
- Rate limit headers on all responses: X-RateLimit-Limit, X-RateLimit-Remaining, Retry-After
- 429 response includes JSON body: { "error": "rate_limit_exceeded", "message": "...", "retry_after": N }
- Usage visibility through response headers only — no usage dashboard or endpoint in Phase 2

### Claude's Discretion
- Exact access token and refresh token lifetimes (within the 15-30 min / 7-30 day ranges)
- Specific rate limit thresholds for trial vs paid tiers
- Fernet encryption implementation details (HKDF derivation, key storage format)
- TenantAwareCollection wrapper internals (how compound indexes are structured)
- Valkey connection pooling and rate limiter algorithm choice (sliding window, token bucket, etc.)
- Email verification token format and expiry duration

</decisions>

<specifics>
## Specific Ideas

- Auth errors should never leak information — follow the principle of least information disclosure
- BYOK validation pattern: call OpenAI's list models endpoint as a lightweight check — don't burn tokens on a real completion
- Rate limit headers should follow the IETF draft standard (X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset)

</specifics>

<deferred>
## Deferred Ideas

- Team members and role management (owner, admin, viewer) — future phase
- Usage dashboard showing consumption against limits — future phase
- Billing and plan upgrade flow — future phase
- Estimate generation per-day caps — consider when LLM integration lands in Phase 3
- Graceful key rotation (old key for in-flight, new key for new requests) — only needed at scale

</deferred>

---

*Phase: 02-multi-tenant-foundation*
*Context gathered: 2026-02-26*
