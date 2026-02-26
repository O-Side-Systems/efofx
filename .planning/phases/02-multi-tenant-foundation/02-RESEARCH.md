# Phase 2: Multi-Tenant Foundation - Research

**Researched:** 2026-02-26
**Domain:** FastAPI multi-tenant auth, JWT, Fernet BYOK encryption, MongoDB TenantAwareCollection, Valkey rate limiting
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Registration & Onboarding**
- Minimal signup: company name, email, password only — collect more later in profile settings
- Email verification required before any platform access (verify-before-access)
- Every new registration starts on trial tier automatically — no plan selection at signup
- Single user per tenant — no team members or role management in Phase 2

**Auth Error Behavior**
- Generic error messages for all security-sensitive operations — "Invalid credentials" for login failures, "If an account exists, we sent a link" for password reset
- Never reveal whether an email is registered (prevent enumeration)
- Silent JWT refresh: short-lived access tokens (15-30 min), long-lived refresh tokens (7-30 days) — user stays logged in seamlessly
- Time-based lockout recovery: locked out for 15 minutes after too many attempts, then can retry — password reset always available as escape hatch

**BYOK Key Management**
- Block all LLM features until contractor stores their OpenAI API key — no platform key fallback, no trial usage
- Validate key on save: make lightweight OpenAI API call (e.g., list models) to confirm key is valid before storing
- Simple overwrite for key rotation: new key replaces old immediately, old encrypted blob deleted, no version history
- Show masked key in settings: last 6 characters visible (sk-...abc123) so contractor can confirm which key is stored

**Tenant Tiers & Rate Limits**
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

### Deferred Ideas (OUT OF SCOPE)
- Team members and role management (owner, admin, viewer) — future phase
- Usage dashboard showing consumption against limits — future phase
- Billing and plan upgrade flow — future phase
- Estimate generation per-day caps — consider when LLM integration lands in Phase 3
- Graceful key rotation (old key for in-flight, new key for new requests) — only needed at scale
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| AUTH-01 | Contractor can register for an efOfX account with company name, email, and password | POST /auth/register endpoint; pwdlib[bcrypt] for password hashing; generate tenant_id (UUID) at registration; insert into tenants collection; trigger email verification flow |
| AUTH-02 | Contractor receives email verification after registration | fastapi-mail 1.6.2 for SMTP sending; store verification token (secrets.token_urlsafe(32)) with expiry in MongoDB; GET /auth/verify?token=... marks tenant as email_verified=True |
| AUTH-03 | Contractor can log in with email/password and receive JWT access + refresh tokens | POST /auth/login; pwdlib verify_password(); PyJWT 2.11.1 encode() with HS256; return {access_token, refresh_token, token_type: "bearer"} |
| AUTH-04 | JWT tokens contain tenant_id, user_id, and role claims with configurable expiration | jwt.encode({"sub": user_id, "tenant_id": tenant_id, "role": role, "exp": ..., "iat": ...}) — PyJWT 2.x encode() returns str directly, no .decode() |
| AUTH-05 | All protected API endpoints require valid JWT and extract tenant_id automatically | FastAPI Depends(get_current_tenant) pattern already established in security.py; extend to require email_verified=True; extract tenant_id from payload["tenant_id"] |
| AUTH-06 | Contractor can update profile settings (name, branding, tier) | PUT /auth/profile or PATCH /tenants/me endpoint; partial updates via TenantUpdate model; blocked for unverified tenants |
| AUTH-07 | API key generated at registration (shown once, stored as bcrypt hash) | secrets.token_urlsafe(32) for raw key; pwdlib hash for storage; return raw key once in registration response; subsequent requests only see "sk-...{last6}" mask |
| ISOL-01 | Tenant isolation middleware enforces tenant_id on every MongoDB query automatically | TenantAwareCollection wrapper class wrapping Motor collection; intercepts find/find_one/insert_one/update_one/delete_one; auto-injects {"tenant_id": tenant_id} into every filter |
| ISOL-02 | Zero cross-tenant data leakage — no query can return another tenant's data | TenantAwareCollection raises if tenant_id missing; compound filter always AND'd; existing rcf_engine.py $or patch covers transition until TenantAwareCollection rolls out |
| ISOL-03 | MongoDB compound indexes include tenant_id as first field for performance | await collection.create_index([("tenant_id", 1), ("other_field", 1)]) — Motor 3.x async index creation at app startup in lifespan |
| ISOL-04 | Platform-provided data (synthetic reference classes) accessible by all tenants | tenant_id=None marks platform data; TenantAwareCollection uses $or: [{tenant_id: tenant_id}, {tenant_id: None}] — existing pattern from rcf_engine fix |
| BYOK-01 | Contractor can store their OpenAI API key encrypted with per-tenant derived Fernet key | cryptography.fernet.Fernet; HKDF from cryptography.hazmat to derive per-tenant key from (MASTER_ENCRYPTION_KEY + tenant_id); store encrypted blob in tenant document |
| BYOK-02 | Encrypted keys are decrypted per-request for LLM calls (never stored in plaintext) | Fernet.decrypt() in request handler; pass decrypted key as api_key= to AsyncOpenAI(api_key=...) — instantiated per-request, not global; key lives only in local scope |
| BYOK-03 | Contractor can rotate their OpenAI key without re-registration | PUT /tenants/me/openai-key; validate new key (list models), encrypt, overwrite encrypted_openai_key field, delete old blob atomically |
| BYOK-04 | Trial tier tenants use platform fallback OpenAI key | CONTEXT.md decision: "Block all LLM features until contractor stores their OpenAI API key — no platform key fallback, no trial usage" — BYOK-04 is superseded by the locked decision; treat BYOK-04 as: LLM endpoints return 402 if no key stored |
| RATE-01 | Per-tenant rate limiting enforced based on tier (trial/pro/enterprise) | slowapi 0.1.9 with Valkey backend (valkey>=6.1.0); tier-based limits in config; key_func=lambda req: extract_tenant_id(req); trial: 20 req/min, paid: 100 req/min (Claude's discretion) |
| RATE-02 | Rate limit headers returned in API responses (remaining, reset time) | slowapi auto-adds X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset headers; add Retry-After via @app.exception_handler(RateLimitExceeded) returning 429 JSON body |
| RATE-03 | Login endpoint rate limited (5 attempts per 15 minutes per IP) | @limiter.limit("5/15minutes") on POST /auth/login; key_func=get_remote_address for IP-based limiting; separate from per-tenant limits |
</phase_requirements>

---

## Summary

Phase 2 builds the complete security and tenancy foundation that every subsequent phase depends on. The implementation targets the primary app at `apps/efofx-estimate/` — the codebase already has a significant head start: Motor MongoDB connection, Pydantic models, PyJWT, pwdlib, and `cryptography` are already declared in `pyproject.toml`. The existing `Tenant` model, `TenantService`, `AuthService`, and `security.py` provide working skeletons that need to be extended, not replaced.

The three highest-risk areas are: (1) the TenantAwareCollection wrapper, which must be bulletproof because every query in the app flows through it; (2) Fernet BYOK encryption, which requires correct HKDF key derivation to avoid cross-tenant decryption accidents; and (3) the rate limiting integration with Valkey, which has a known concern (Valkey SSL via `valkeys://` URI needs verification — noted in STATE.md). The email verification flow with `fastapi-mail` is new infrastructure but low-risk since it's a simple SMTP send.

The biggest implementation gap is that the existing `pyproject.toml` at `apps/efofx-estimate/` does not include `valkey`, `slowapi`, or `fastapi-mail` as dependencies — these must be added. Additionally, the existing `Settings` class uses HS256 with `JWT_SECRET_KEY` (symmetric) which aligns with the Phase 2 design; no need to switch to RS256 for single-service MVP.

**Primary recommendation:** Build the TenantAwareCollection wrapper first as the isolation foundation, then layer auth/registration on top, then BYOK encryption, then rate limiting. This order ensures isolation is in place before any new tenant data is created.

---

## Standard Stack

### Core (already in pyproject.toml at apps/efofx-estimate/)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `PyJWT` | 2.11.0 (pyproject) / 2.11.1 (latest) | JWT encode/decode with tenant claims | Already migrated from python-jose in Phase 1; confirmed working |
| `pwdlib[bcrypt]` | 0.3.0 | Password hashing for tenant accounts | Already migrated from passlib in Phase 1; confirmed working |
| `cryptography` | >=46.0.0 | Fernet encryption + HKDF key derivation | Listed as transitive dep; already importable; provides Fernet and HKDF primitives |
| `motor` | 3.3.2 | Async MongoDB driver | Already installed; TenantAwareCollection wraps Motor collections |
| `fastapi` | 0.116.1 | HTTP framework | Already installed |
| `pydantic` | 2.11.7 | Request/response models | Already installed |
| `pydantic-settings` | 2.2.1 | Settings from env vars | Already installed |

### New Dependencies (must be added to pyproject.toml)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `valkey` | >=6.1.0 | Valkey/Redis client for rate limiting | Required for slowapi Valkey backend; DigitalOcean deprecated Redis in favor of Valkey June 2025 |
| `slowapi` | 0.1.9 | Per-tenant + per-IP rate limiting middleware | Starlette/FastAPI-native; Valkey backend; decorator-based |
| `fastapi-mail` | 1.6.2 | Async SMTP email sending | Email verification and (future) password reset flows |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `slowapi` | `limits` + custom middleware | slowapi wraps limits library; custom gives more control but 3x more code |
| `fastapi-mail` | `smtplib` | smtplib is synchronous — blocks FastAPI event loop |
| `fastapi-mail` | Vendor SDKs (SendGrid, Postmark) | Vendor lock-in; fastapi-mail is SMTP-agnostic |
| `Fernet` symmetric encryption | AES-GCM manual | Fernet is authenticated encryption (HMAC-SHA256 + AES-128-CBC) — correct default; manual AES-GCM adds unauthenticated plaintext risk |
| `HKDF` per-tenant key derivation | Shared master key for all tenants | HKDF limits blast radius — compromise of one tenant key doesn't expose others |
| `slowapi` Valkey backend | MongoDB TTL counters (Phase 1 interim plan) | STATE.md shows "MongoDB-based rate limiting for Phase 2" was Phase 1 context; now Phase 2 should use Valkey as the final design |

**Installation:**
```bash
# From apps/efofx-estimate/
pip install "valkey>=6.1.0" "slowapi==0.1.9" "fastapi-mail==1.6.2"
# Then add to pyproject.toml dependencies section
```

---

## Architecture Patterns

### Recommended Project Structure (additions to existing app/)

```
apps/efofx-estimate/app/
├── api/
│   ├── routes.py              # existing — add auth routes here or new auth.py
│   └── auth.py                # NEW: /auth/register, /auth/login, /auth/verify, /auth/refresh
├── core/
│   ├── config.py              # extend Settings: VALKEY_URL, MASTER_ENCRYPTION_KEY, SMTP_*
│   ├── constants.py           # add: DB_COLLECTIONS["USERS"], ["VERIFICATION_TOKENS"], ["REFRESH_TOKENS"]
│   ├── security.py            # extend: TenantAwareCollection, BYOK encrypt/decrypt, get_current_tenant
│   └── rate_limit.py          # NEW: slowapi Limiter setup, tier-based limits, custom key functions
├── db/
│   ├── mongodb.py             # extend: create_indexes() for compound (tenant_id, ...) indexes
│   └── tenant_collection.py   # NEW: TenantAwareCollection wrapper class
├── middleware/
│   └── auth_middleware.py     # NEW (optional): middleware for auto-extracting tenant_id to request.state
├── models/
│   ├── tenant.py              # extend: add tier, email_verified, hashed_password, encrypted_openai_key, hashed_api_key
│   └── auth.py                # NEW: RegisterRequest, LoginRequest, LoginResponse, TokenResponse
├── services/
│   ├── auth_service.py        # NEW: register(), login(), verify_email(), refresh_token()
│   ├── byok_service.py        # NEW: encrypt_key(), decrypt_key(), validate_openai_key(), rotate_key()
│   └── tenant_service.py      # extend: get_by_email(), get_by_api_key_hash()
└── utils/
    └── crypto.py              # NEW: derive_tenant_fernet_key(master_key, tenant_id) using HKDF
```

### Pattern 1: TenantAwareCollection Wrapper

**What:** A class that wraps a Motor `AsyncIOMotorCollection` and automatically injects `tenant_id` into every query filter and insert document. Raises `RuntimeError` if called without a tenant context.

**When to use:** Every collection that holds per-tenant data. Platform data collections (reference_classes with `tenant_id=None`) use a special `allow_platform=True` mode that generates `$or: [{tenant_id: tid}, {tenant_id: None}]`.

**Example:**
```python
# apps/efofx-estimate/app/db/tenant_collection.py
from motor.motor_asyncio import AsyncIOMotorCollection
from typing import Any, Optional

class TenantAwareCollection:
    """Wraps Motor collection and auto-injects tenant_id on every operation."""

    def __init__(
        self,
        collection: AsyncIOMotorCollection,
        tenant_id: str,
        allow_platform_data: bool = False,
    ) -> None:
        self._col = collection
        self._tenant_id = tenant_id
        self._allow_platform = allow_platform_data

    def _scoped_filter(self, filter: dict[str, Any]) -> dict[str, Any]:
        """Build tenant-scoped filter. Never returns unscoped query."""
        if self._allow_platform:
            tenant_filter: dict[str, Any] = {
                "$or": [
                    {"tenant_id": self._tenant_id},
                    {"tenant_id": None},
                ]
            }
        else:
            tenant_filter = {"tenant_id": self._tenant_id}

        if filter:
            return {"$and": [tenant_filter, filter]}
        return tenant_filter

    async def find_one(self, filter: dict[str, Any], **kwargs: Any) -> Optional[dict]:
        return await self._col.find_one(self._scoped_filter(filter), **kwargs)

    def find(self, filter: dict[str, Any], **kwargs: Any):
        return self._col.find(self._scoped_filter(filter), **kwargs)

    async def insert_one(self, document: dict[str, Any], **kwargs: Any):
        document["tenant_id"] = self._tenant_id
        return await self._col.insert_one(document, **kwargs)

    async def update_one(self, filter: dict[str, Any], update: dict, **kwargs: Any):
        return await self._col.update_one(self._scoped_filter(filter), update, **kwargs)

    async def delete_one(self, filter: dict[str, Any], **kwargs: Any):
        return await self._col.delete_one(self._scoped_filter(filter), **kwargs)

    async def count_documents(self, filter: dict[str, Any], **kwargs: Any) -> int:
        return await self._col.count_documents(self._scoped_filter(filter), **kwargs)
```

### Pattern 2: BYOK Fernet Encryption with HKDF

**What:** Derive a per-tenant Fernet key from a shared master secret using HKDF. Encrypt OpenAI keys with the derived key. Decrypt per-request in a local scope — key never persists beyond the request.

**When to use:** Storing BYOK OpenAI keys in the `tenants` collection, and decrypting them when instantiating `AsyncOpenAI` for LLM calls.

**Example:**
```python
# apps/efofx-estimate/app/utils/crypto.py
import base64
from cryptography.fernet import Fernet
from cryptography.hazmat.primitives.kdf.hkdf import HKDF
from cryptography.hazmat.primitives import hashes

def derive_tenant_fernet_key(master_key: bytes, tenant_id: str) -> Fernet:
    """Derive a unique Fernet key for a tenant using HKDF."""
    hkdf = HKDF(
        algorithm=hashes.SHA256(),
        length=32,
        salt=None,  # master_key already high-entropy
        info=f"efofx-byok-{tenant_id}".encode(),
    )
    derived_key = hkdf.derive(master_key)
    # Fernet requires URL-safe base64-encoded 32-byte key
    fernet_key = base64.urlsafe_b64encode(derived_key)
    return Fernet(fernet_key)


def encrypt_openai_key(master_key: bytes, tenant_id: str, plaintext_key: str) -> str:
    """Encrypt tenant's OpenAI API key. Returns base64 ciphertext string."""
    fernet = derive_tenant_fernet_key(master_key, tenant_id)
    encrypted = fernet.encrypt(plaintext_key.encode())
    return encrypted.decode()  # Store as string in MongoDB


def decrypt_openai_key(master_key: bytes, tenant_id: str, encrypted_key: str) -> str:
    """Decrypt tenant's OpenAI API key. Only call in request scope."""
    fernet = derive_tenant_fernet_key(master_key, tenant_id)
    return fernet.decrypt(encrypted_key.encode()).decode()


def mask_openai_key(plaintext_key: str) -> str:
    """Return masked key showing last 6 chars: sk-...abc123"""
    if len(plaintext_key) < 6:
        return "sk-...******"
    return f"sk-...{plaintext_key[-6:]}"
```

### Pattern 3: JWT Registration + Refresh Token Flow

**What:** Issue short-lived access tokens (20 min) and long-lived refresh tokens (14 days). Store refresh tokens as hashed values in MongoDB with a TTL index. Silent refresh via `POST /auth/refresh`.

**When to use:** All POST /auth/login and POST /auth/refresh flows.

**Example:**
```python
# apps/efofx-estimate/app/services/auth_service.py
import secrets
import jwt
from datetime import datetime, timedelta, timezone
from pwdlib import PasswordHash

from app.core.config import settings

password_hash = PasswordHash.recommended()

ACCESS_TOKEN_MINUTES = 20
REFRESH_TOKEN_DAYS = 14

def create_access_token(tenant_id: str, user_id: str, role: str) -> str:
    """Create short-lived JWT access token with required claims."""
    now = datetime.now(timezone.utc)
    payload = {
        "sub": user_id,
        "tenant_id": tenant_id,
        "role": role,
        "iat": now,
        "exp": now + timedelta(minutes=ACCESS_TOKEN_MINUTES),
    }
    # PyJWT 2.x encode() returns str directly — no .decode() needed
    return jwt.encode(payload, settings.JWT_SECRET_KEY, algorithm="HS256")

def verify_access_token(token: str) -> dict:
    """Verify JWT and return payload. Raises jwt.InvalidTokenError on failure."""
    return jwt.decode(
        token,
        settings.JWT_SECRET_KEY,
        algorithms=["HS256"],
        options={"require": ["exp", "iat", "sub", "tenant_id", "role"]},
    )

def create_refresh_token() -> tuple[str, str]:
    """Return (raw_token, hashed_token). Store hash in DB, return raw to client."""
    raw = secrets.token_urlsafe(48)
    hashed = password_hash.hash(raw)
    return raw, hashed
```

### Pattern 4: slowapi Rate Limiting with Valkey

**What:** Use slowapi with a Valkey backend for distributed rate limiting. Separate limits for login (IP-based) and API calls (tenant-based).

**When to use:** All protected API endpoints and the login endpoint.

**Example:**
```python
# apps/efofx-estimate/app/core/rate_limit.py
from slowapi import Limiter, _rate_limit_exceeded_handler
from slowapi.errors import RateLimitExceeded
from slowapi.util import get_remote_address
from fastapi import Request
from fastapi.responses import JSONResponse

TIER_LIMITS = {
    "trial": "20/minute",
    "paid": "100/minute",
}

def get_tenant_id_for_limit(request: Request) -> str:
    """Key function: use tenant_id from token state for per-tenant limits."""
    tenant_id = getattr(request.state, "tenant_id", None)
    if tenant_id:
        return tenant_id
    return get_remote_address(request)  # fallback to IP

# Limiter with Valkey backend
limiter = Limiter(
    key_func=get_remote_address,  # default; override per-route
    storage_uri=settings.VALKEY_URL,  # e.g. "valkeys://user:pass@host:6379"
)

async def rate_limit_exceeded_handler(request: Request, exc: RateLimitExceeded) -> JSONResponse:
    """Return standardized 429 body with Retry-After."""
    retry_after = exc.retry_after if hasattr(exc, "retry_after") else 60
    return JSONResponse(
        status_code=429,
        content={
            "error": "rate_limit_exceeded",
            "message": f"Rate limit exceeded. Retry after {retry_after} seconds.",
            "retry_after": retry_after,
        },
        headers={"Retry-After": str(retry_after)},
    )
```

```python
# Usage in auth.py router — login rate limiting by IP
from app.core.rate_limit import limiter

@router.post("/auth/login")
@limiter.limit("5/15minutes", key_func=get_remote_address)
async def login(request: Request, credentials: LoginRequest) -> TokenResponse:
    ...
```

### Pattern 5: Email Verification with fastapi-mail

**What:** Send verification email on registration using fastapi-mail. Token stored in MongoDB with 24-hour TTL. Verification link: `GET /auth/verify?token={token}`.

**Example:**
```python
# apps/efofx-estimate/app/services/auth_service.py
import secrets
from datetime import datetime, timedelta, timezone
from fastapi_mail import FastMail, MessageSchema, ConnectionConfig

mail_config = ConnectionConfig(
    MAIL_USERNAME=settings.SMTP_USERNAME,
    MAIL_PASSWORD=settings.SMTP_PASSWORD,
    MAIL_FROM=settings.SMTP_FROM,
    MAIL_PORT=settings.SMTP_PORT,
    MAIL_SERVER=settings.SMTP_SERVER,
    MAIL_STARTTLS=True,
    MAIL_SSL_TLS=False,
)
fast_mail = FastMail(mail_config)

async def send_verification_email(email: str, token: str) -> None:
    """Send email verification link."""
    verify_url = f"{settings.APP_BASE_URL}/auth/verify?token={token}"
    message = MessageSchema(
        subject="Verify your efOfX account",
        recipients=[email],
        body=f"Click to verify your account: {verify_url}\n\nThis link expires in 24 hours.",
        subtype="plain",
    )
    await fast_mail.send_message(message)

def generate_verification_token() -> tuple[str, datetime]:
    """Return (token, expiry). Token is URL-safe random bytes."""
    token = secrets.token_urlsafe(32)
    expiry = datetime.now(timezone.utc) + timedelta(hours=24)
    return token, expiry
```

### Anti-Patterns to Avoid

- **Storing OpenAI keys in plaintext anywhere** — not in env vars (that would be the platform key), not in the tenant document, not in logs. The encrypted blob is the only form that touches MongoDB.
- **Global AsyncOpenAI client with a single key** — always instantiate per-request with the tenant's decrypted key. The existing `openai_client.py` in `estimator-project/` does this wrong.
- **Symmetric API key auth without a hash** — AUTH-07 requires showing the key once and storing only the bcrypt hash. Searching by hash requires iterating all tenants. Design: generate `sk_live_{tenant_id}_{random}` format so tenant_id is in the key; validate by: extract tenant_id, fetch tenant, verify hash.
- **In-memory rate limiter** — the existing `RateLimiter` class in `app/core/security.py` is a known bad pattern (see CONCERNS.md). Phase 2 replaces it with slowapi+Valkey entirely.
- **Leaking auth state in error messages** — "Invalid credentials" only, never "User not found" or "Wrong password".
- **Missing `email_verified` guard on protected routes** — Every `get_current_tenant()` dependency must check `tenant.email_verified == True` or return 403 with message "Email not verified".

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Password hashing | Custom bcrypt calls | `pwdlib[bcrypt]` already in pyproject.toml | Timing-safe comparison, correct cost factor, upgrade path to Argon2 |
| JWT encode/decode | Manual HMAC/base64 | `PyJWT` already in pyproject.toml | Handles exp, iat, algorithm negotiation, error types |
| Fernet encryption | Manual AES-CBC + HMAC | `cryptography.fernet.Fernet` already importable | Authenticated encryption; manual AES mistakes are common and catastrophic |
| Rate limiting state | Redis/Valkey INCR loops | `slowapi` + valkey-py | Handles sliding window, headers, exception mapping; 3 lines vs 100+ |
| Email sending | `smtplib` + `asyncio.to_thread` | `fastapi-mail` | Async-native; blocking smtplib in async context causes event loop stalls |
| HKDF key derivation | SHA-256(master + tenant_id) | `cryptography.hazmat.primitives.kdf.hkdf.HKDF` | Manual key derivation violates cryptographic assumptions; HKDF is the standard |
| Secure random tokens | `uuid4()` for verification tokens | `secrets.token_urlsafe(32)` | UUID4 has 122 bits of entropy; secrets provides 192 bits and is URL-safe by default |

**Key insight:** The cryptography library is the one area where hand-rolling almost always introduces subtle vulnerabilities. Fernet + HKDF from the `cryptography` package are the correct primitives and they're already available as a transitive dependency.

---

## Common Pitfalls

### Pitfall 1: TenantAwareCollection Not Wrapping All Collection Access Points
**What goes wrong:** Code that calls `get_collection("estimates")` directly bypasses TenantAwareCollection and returns raw Motor collection, allowing cross-tenant queries.
**Why it happens:** Existing `get_collection()` in `mongodb.py` returns plain Motor collection; developers forget to wrap it.
**How to avoid:** Provide a `get_tenant_collection(tenant_id, collection_name)` factory function that always returns `TenantAwareCollection`. Grep for direct `db[...]` and `get_collection(...)` calls during code review. The existing `get_estimates_collection()` etc. in `mongodb.py` must be refactored to require `tenant_id` or removed.
**Warning signs:** Any `await collection.find_one({})` without `"tenant_id"` in the filter dict.

### Pitfall 2: PyJWT datetime.utcnow() Deprecation Warning
**What goes wrong:** `datetime.utcnow()` used in JWT payload raises `DeprecationWarning` in Python 3.12+; some versions fail silently with wrong timestamps.
**Why it happens:** Existing `security.py` and `auth_service.py` skeletons use `datetime.utcnow()`.
**How to avoid:** Always use `datetime.now(timezone.utc)` for JWT timestamps. PyJWT 2.x accepts timezone-aware datetimes and handles UTC correctly.
**Warning signs:** `DeprecationWarning: datetime.utcnow() is deprecated` in logs.

### Pitfall 3: Fernet Key Must Be 32 URL-Safe Base64 Bytes
**What goes wrong:** `Fernet(key)` raises `ValueError: Fernet key must be 32 url-safe base64-encoded bytes` if you pass raw bytes or wrong-length key.
**Why it happens:** HKDF derives 32 raw bytes; Fernet needs those 32 bytes base64-encoded. Missing `base64.urlsafe_b64encode()` step.
**How to avoid:** Always: `fernet_key = base64.urlsafe_b64encode(hkdf_output)` before `Fernet(fernet_key)`.
**Warning signs:** `ValueError` during `Fernet()` initialization.

### Pitfall 4: slowapi Valkey SSL URI Format
**What goes wrong:** `valkeys://` (SSL) vs `redis://` format confusion; `Limiter(storage_uri=...)` rejects wrong scheme.
**Why it happens:** DigitalOcean Managed Valkey requires SSL (`valkeys://`); local dev uses `redis://localhost:6379` (works because valkey-py is redis-py fork with same protocol). Confusing the two causes connection errors in production.
**How to avoid:** Use `VALKEY_URL` env var; set to `redis://localhost:6379` in dev (plain), `valkeys://user:pass@host:25061` in production. slowapi inherits the storage from `limits` library which uses `redis` scheme even for Valkey. Confirm `valkeys://` works with limits library before deploying — if not, use `rediss://` (TLS Redis compatible, which Valkey serves).
**Warning signs:** `ConnectionRefusedError` or `AuthenticationError` in production but works locally.

### Pitfall 5: pytest-asyncio strict mode + Motor Client per Test
**What goes wrong:** Session-scoped Motor client from `conftest.py` fixture conflicts with per-test event loops in pytest-asyncio strict mode. Tests fail with `RuntimeError: Event loop is closed`.
**Why it happens:** pytest-asyncio 1.3.0 (installed) uses strict mode; session-scoped fixtures share the event loop but Motor creates async resources bound to a specific loop.
**How to avoid:** Follow the per-test Motor client pattern established in `test_tenant_isolation.py` — create a fresh `AsyncIOMotorClient` inside the fixture, yield, close it, reset `app.db.mongodb._client` and `_database` to None. Do not use session-scoped database fixtures. See existing pattern in `tests/services/test_tenant_isolation.py`.
**Warning signs:** Tests pass individually but fail in suite, or `RuntimeError: Event loop is closed`.

### Pitfall 6: Generic Auth Errors That Still Leak Information
**What goes wrong:** Returning different HTTP status codes for "user not found" (404) vs "wrong password" (401) reveals user existence via timing or status.
**Why it happens:** Natural FastAPI error handling pattern raises 404 for missing resource.
**How to avoid:** Auth endpoints must ALWAYS return 401 with the same message and same response time regardless of whether the user exists. Use constant-time comparison for API keys. Never use 404 in auth flows.
**Warning signs:** Login returning 404 for unknown email.

### Pitfall 7: BYOK-04 Contradiction with Locked Decision
**What goes wrong:** REQUIREMENTS.md BYOK-04 says "Trial tier tenants use platform fallback OpenAI key" but the locked decision in CONTEXT.md says "no platform key fallback, no trial usage." These contradict.
**Why it happens:** Requirements written before the user discussion; CONTEXT.md is the authoritative override.
**How to avoid:** BYOK-04 should be implemented as: if `encrypted_openai_key` is null, all LLM endpoints return 402 Payment Required with message "OpenAI API key required. Add your key in Settings." This satisfies the spirit of the requirement (trial management) without the platform fallback.
**Warning signs:** Building platform key fallback logic that was explicitly rejected.

---

## Code Examples

Verified patterns from official sources and existing codebase:

### Registration Endpoint Pattern

```python
# apps/efofx-estimate/app/api/auth.py
import uuid, secrets
from datetime import datetime, timezone, timedelta
from fastapi import APIRouter, HTTPException, status
from pwdlib import PasswordHash

router = APIRouter(prefix="/auth", tags=["auth"])
password_hash = PasswordHash.recommended()

@router.post("/register", status_code=status.HTTP_201_CREATED)
async def register(body: RegisterRequest) -> RegisterResponse:
    db = get_database()
    # Check email not already registered — silently (no enumeration)
    existing = await db["tenants"].find_one({"email": body.email})
    if existing:
        # Return 201 as if success to prevent email enumeration
        # In practice: still send verification email to registered addr
        return RegisterResponse(message="If this email is new, check your inbox.")

    tenant_id = str(uuid.uuid4())
    hashed_password = password_hash.hash(body.password)

    # Generate API key (show once, store hash)
    raw_api_key = f"sk_live_{tenant_id.replace('-', '')}_{secrets.token_urlsafe(16)}"
    hashed_api_key = password_hash.hash(raw_api_key)

    verification_token, token_expiry = generate_verification_token()

    tenant_doc = {
        "tenant_id": tenant_id,
        "company_name": body.company_name,
        "email": body.email,
        "hashed_password": hashed_password,
        "hashed_api_key": hashed_api_key,
        "tier": "trial",
        "email_verified": False,
        "encrypted_openai_key": None,
        "created_at": datetime.now(timezone.utc),
    }
    await db["tenants"].insert_one(tenant_doc)

    # Store verification token
    await db["verification_tokens"].insert_one({
        "token": verification_token,
        "tenant_id": tenant_id,
        "expires_at": token_expiry,
    })

    await send_verification_email(body.email, verification_token)

    return RegisterResponse(
        tenant_id=tenant_id,
        api_key=raw_api_key,  # shown ONCE only
        message="Account created. Check email to verify.",
    )
```

### get_current_tenant Dependency (Updated)

```python
# apps/efofx-estimate/app/core/security.py
async def get_current_tenant(
    credentials: HTTPAuthorizationCredentials = Depends(security)
) -> Tenant:
    """Extract and validate tenant from JWT. Requires email_verified=True."""
    if not credentials:
        raise HTTPException(status_code=401, detail="Authentication required")

    try:
        payload = jwt.decode(
            credentials.credentials,
            settings.JWT_SECRET_KEY,
            algorithms=["HS256"],
            options={"require": ["exp", "iat", "sub", "tenant_id", "role"]},
        )
    except jwt.ExpiredSignatureError:
        raise HTTPException(status_code=401, detail="Token expired")
    except jwt.InvalidTokenError:
        raise HTTPException(status_code=401, detail="Invalid token")

    db = get_database()
    tenant_doc = await db["tenants"].find_one({"tenant_id": payload["tenant_id"]})
    if not tenant_doc:
        raise HTTPException(status_code=401, detail="Invalid token")

    tenant = Tenant(**tenant_doc)
    if not tenant.email_verified:
        raise HTTPException(status_code=403, detail="Email not verified")

    return tenant
```

### Compound Index Creation (Motor 3.x)

```python
# apps/efofx-estimate/app/db/mongodb.py
async def create_indexes():
    db = get_database()

    # Tenants
    await db["tenants"].create_index("email", unique=True)
    await db["tenants"].create_index("tenant_id", unique=True)

    # Estimates — compound with tenant_id FIRST (ISOL-03)
    await db["estimates"].create_index(
        [("tenant_id", 1), ("created_at", -1)]
    )
    await db["estimates"].create_index(
        [("tenant_id", 1), ("session_id", 1)], unique=True
    )

    # Reference classes — tenant_id first, platform data (None) queryable
    await db["reference_classes"].create_index(
        [("tenant_id", 1), ("category", 1)]
    )

    # Verification tokens — TTL auto-delete after 24h
    await db["verification_tokens"].create_index(
        "expires_at", expireAfterSeconds=0
    )

    # Refresh tokens — TTL auto-delete after 14 days
    await db["refresh_tokens"].create_index(
        "expires_at", expireAfterSeconds=0
    )
```

### BYOK Validation Pattern

```python
# apps/efofx-estimate/app/services/byok_service.py
from openai import AsyncOpenAI, AuthenticationError

async def validate_and_store_openai_key(
    tenant_id: str, plaintext_key: str, master_key: bytes
) -> str:
    """Validate key against OpenAI, encrypt, return masked version."""
    # 1. Validate by calling list models — lightweight, no tokens burned
    try:
        client = AsyncOpenAI(api_key=plaintext_key)
        await client.models.list()
    except AuthenticationError:
        raise HTTPException(status_code=400, detail="Invalid OpenAI API key")
    except Exception:
        raise HTTPException(status_code=503, detail="Could not validate key")

    # 2. Encrypt and store
    encrypted = encrypt_openai_key(master_key, tenant_id, plaintext_key)

    db = get_database()
    await db["tenants"].update_one(
        {"tenant_id": tenant_id},
        {"$set": {"encrypted_openai_key": encrypted, "updated_at": datetime.now(timezone.utc)}}
    )

    # 3. Return masked version for UI display
    return mask_openai_key(plaintext_key)
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `python-jose` JWT | `PyJWT` 2.11.x | Phase 1 (complete) | Already migrated; no action needed |
| `passlib[bcrypt]` | `pwdlib[bcrypt]` | Phase 1 (complete) | Already migrated; no action needed |
| In-memory rate limiter dict | `slowapi` + Valkey | Phase 2 (this phase) | Replace `RateLimiter` class in security.py entirely |
| `redis` client | `valkey` client | Phase 1 resolved (remove), Phase 2 adds Valkey | STATE.md says Redis removed in Phase 1; add valkey now |
| Direct collection access | TenantAwareCollection wrapper | Phase 2 (this phase) | All collection access through wrapper; existing `get_*_collection()` functions must be refactored |
| `datetime.utcnow()` | `datetime.now(timezone.utc)` | Python 3.12 deprecation | Update all JWT and timestamp code to timezone-aware |

**Deprecated/outdated in this codebase:**
- `RateLimiter` class in `app/core/security.py` (lines 122-152): in-memory, not thread-safe, memory leak. Replace entirely.
- `get_estimates_collection()` etc. in `mongodb.py`: returns raw Motor collection. Refactor to accept `tenant_id` and return `TenantAwareCollection`.
- `Tenant.openai_api_key: Optional[str]` in `models/tenant.py`: stores plaintext. Must become `encrypted_openai_key: Optional[str]`.
- `settings.JWT_ALGORITHM` hardcoded as `"HS256"` in `AuthService.__init__`: already matches, but use the settings value.

---

## Open Questions

1. **BYOK-04 vs. CONTEXT.md locked decision**
   - What we know: REQUIREMENTS.md says "Trial tier tenants use platform fallback OpenAI key" but CONTEXT.md explicitly locks "no platform key fallback, no trial usage."
   - What's unclear: Was BYOK-04 intentionally superseded or is the CONTEXT.md decision about something different?
   - Recommendation: Treat CONTEXT.md as authoritative override. Implement BYOK-04 as "LLM endpoints return 402 if no key stored" rather than platform fallback. Flag for user confirmation before planning.

2. **Valkey SSL URI format with slowapi/limits library**
   - What we know: DigitalOcean Managed Valkey requires SSL; valkey-py uses `valkeys://`; slowapi delegates storage to `limits` library which historically used `redis://` schemes.
   - What's unclear: Whether `limits` library (used by slowapi 0.1.9) accepts `valkeys://` or requires `rediss://` (TLS Redis) with valkey-py as backend.
   - Recommendation: Test locally first with `redis://localhost:6379` (plain Valkey works the same protocol). For DO deployment, try `valkeys://` first; if limits rejects it, try `rediss://` with SSL cert verify disabled. Noted as blocker in STATE.md.

3. **Email verification token storage: MongoDB vs. in-token (JWT)**
   - What we know: CONTEXT.md says "Claude's discretion" for token format/expiry. MongoDB TTL index approach stores token in DB, allows revocation. JWT approach is stateless but can't be revoked.
   - Recommendation: Use MongoDB TTL index (secrets.token_urlsafe(32) stored in `verification_tokens` collection with `expireAfterSeconds=0` and TTL index on `expires_at`). Allows single-use enforcement and revocation. No JWT complexity for a one-time link.

4. **Existing `pyproject.toml` asyncio_mode setting**
   - What we know: `apps/efofx-estimate/pyproject.toml` has `pytest-asyncio==1.3.0` but does NOT have `asyncio_mode = "auto"` in `[tool.pytest.ini_options]` (unlike `apps/estimator-project/`). Tests use explicit `@pytest.mark.asyncio`.
   - What's unclear: Whether to add `asyncio_mode = "auto"` to efofx-estimate's pyproject.toml for consistency.
   - Recommendation: Add `asyncio_mode = "auto"` to `[tool.pytest.ini_options]` in `apps/efofx-estimate/pyproject.toml` to match the established pattern from Phase 1 (estimator-project); follow existing `test_tenant_isolation.py` per-test Motor client pattern for all new DB tests.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | pytest 8.4.1 + pytest-asyncio 1.3.0 |
| Config file | `apps/efofx-estimate/pyproject.toml` |
| Quick run command | `cd apps/efofx-estimate && pytest tests/ -x -v --tb=short` |
| Full suite command | `cd apps/efofx-estimate && pytest tests/ -v --cov=app --cov-report=term-missing` |
| Integration tests | `cd apps/efofx-estimate && pytest tests/ -m integration -v` (requires live MongoDB) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| AUTH-01 | POST /auth/register creates tenant, hashes password, returns api_key once | unit | `pytest tests/api/test_auth.py::test_register_success -x` | ❌ Wave 0 |
| AUTH-01 | Registration with duplicate email returns non-enumerating response | unit | `pytest tests/api/test_auth.py::test_register_duplicate_email -x` | ❌ Wave 0 |
| AUTH-02 | Verification token created and sent on registration | unit (mocked mail) | `pytest tests/api/test_auth.py::test_verification_email_sent -x` | ❌ Wave 0 |
| AUTH-02 | GET /auth/verify with valid token sets email_verified=True | integration | `pytest tests/api/test_auth.py::test_email_verification_success -m integration -x` | ❌ Wave 0 |
| AUTH-03 | POST /auth/login with correct credentials returns access+refresh tokens | unit | `pytest tests/api/test_auth.py::test_login_success -x` | ❌ Wave 0 |
| AUTH-03 | POST /auth/login with wrong password returns 401 generic error | unit | `pytest tests/api/test_auth.py::test_login_wrong_password -x` | ❌ Wave 0 |
| AUTH-04 | JWT payload contains tenant_id, user_id (sub), role, exp, iat | unit | `pytest tests/services/test_auth_service.py::test_jwt_claims -x` | ❌ Wave 0 |
| AUTH-05 | Protected endpoint with expired JWT returns 401 | unit | `pytest tests/api/test_auth.py::test_expired_token_returns_401 -x` | ❌ Wave 0 |
| AUTH-05 | Protected endpoint with missing JWT returns 401 | unit | `pytest tests/api/test_auth.py::test_missing_token_returns_401 -x` | ❌ Wave 0 |
| AUTH-05 | Unverified tenant gets 403 on protected endpoint | unit | `pytest tests/api/test_auth.py::test_unverified_tenant_blocked -x` | ❌ Wave 0 |
| AUTH-06 | PATCH /tenants/me updates profile fields | unit | `pytest tests/api/test_auth.py::test_update_profile -x` | ❌ Wave 0 |
| AUTH-07 | API key shown once in register response, not retrievable again | unit | `pytest tests/api/test_auth.py::test_api_key_shown_once -x` | ❌ Wave 0 |
| ISOL-01 | TenantAwareCollection.find_one always includes tenant_id in filter | unit | `pytest tests/db/test_tenant_collection.py::test_find_one_scoped -x` | ❌ Wave 0 |
| ISOL-02 | Two tenants cannot see each other's estimates | integration | `pytest tests/services/test_tenant_isolation.py::test_no_cross_tenant_leakage -m integration -x` | ✅ Partial (exists for RCF) |
| ISOL-03 | Compound indexes created with tenant_id first | integration | `pytest tests/db/test_indexes.py::test_compound_indexes_created -m integration -x` | ❌ Wave 0 |
| ISOL-04 | Platform data (tenant_id=None) visible to all tenants | integration | `pytest tests/services/test_tenant_isolation.py::test_platform_data_visible_to_all -m integration -x` | ✅ Exists |
| BYOK-01 | encrypt_openai_key produces different ciphertext per tenant | unit | `pytest tests/utils/test_crypto.py::test_per_tenant_encryption -x` | ❌ Wave 0 |
| BYOK-02 | decrypt_openai_key round-trips correctly; plaintext not persisted | unit | `pytest tests/utils/test_crypto.py::test_decrypt_roundtrip -x` | ❌ Wave 0 |
| BYOK-03 | PUT /tenants/me/openai-key overwrites old key | unit | `pytest tests/api/test_byok.py::test_key_rotation -x` | ❌ Wave 0 |
| BYOK-04 | LLM endpoint returns 402 when no OpenAI key stored | unit | `pytest tests/api/test_byok.py::test_no_key_returns_402 -x` | ❌ Wave 0 |
| RATE-01 | Trial tenant rate-limited after threshold | unit (fakeredis) | `pytest tests/api/test_rate_limit.py::test_trial_rate_limit -x` | ❌ Wave 0 |
| RATE-02 | Rate limit headers present in all API responses | unit | `pytest tests/api/test_rate_limit.py::test_rate_limit_headers -x` | ❌ Wave 0 |
| RATE-03 | Login rate limited to 5 attempts per 15min per IP | unit (fakeredis) | `pytest tests/api/test_rate_limit.py::test_login_rate_limit_ip -x` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cd apps/efofx-estimate && pytest tests/ -x -v --tb=short -m "not integration"`
- **Per wave merge:** `cd apps/efofx-estimate && pytest tests/ -v --cov=app --cov-report=term-missing`
- **Phase gate:** Full suite green (including integration with live MongoDB) before `/gsd:verify-work`

### Wave 0 Gaps (test infrastructure needed before implementation)

- [ ] `tests/api/__init__.py` — package file
- [ ] `tests/api/test_auth.py` — covers AUTH-01 through AUTH-07 (registration, login, verification, JWT validation)
- [ ] `tests/api/test_byok.py` — covers BYOK-01 through BYOK-04 (key storage, encryption, rotation, 402 gate)
- [ ] `tests/api/test_rate_limit.py` — covers RATE-01 through RATE-03 (use `fakeredis` for Valkey mock)
- [ ] `tests/db/__init__.py` — package file
- [ ] `tests/db/test_tenant_collection.py` — covers ISOL-01, ISOL-03 (TenantAwareCollection unit tests; no DB required)
- [ ] `tests/db/test_indexes.py` — covers ISOL-03 (integration, requires live MongoDB)
- [ ] `tests/utils/__init__.py` — package file
- [ ] `tests/utils/test_crypto.py` — covers BYOK-01, BYOK-02 (pure unit tests, no DB or network)
- [ ] `tests/services/test_auth_service.py` — covers AUTH-04 (JWT claim validation)
- [ ] `apps/efofx-estimate/pyproject.toml` update: add `asyncio_mode = "auto"` to `[tool.pytest.ini_options]`
- [ ] Add `fakeredis` to dev dependencies: `fakeredis>=2.0.0` — valkey/redis-compatible in-memory mock for rate limit tests

---

## Sources

### Primary (HIGH confidence)
- `apps/efofx-estimate/app/core/security.py` — existing auth skeleton, JWT patterns, RateLimiter class to replace
- `apps/efofx-estimate/app/db/mongodb.py` — existing Motor connection, index creation pattern
- `apps/efofx-estimate/app/models/tenant.py` — existing Tenant model to extend
- `apps/efofx-estimate/tests/services/test_tenant_isolation.py` — per-test Motor client pattern (pytest-asyncio 1.3.0)
- `apps/efofx-estimate/pyproject.toml` — confirmed installed deps: PyJWT 2.11.0, pwdlib[bcrypt] 0.3.0, cryptography (transitive), motor 3.3.2
- `.planning/research/STACK.md` — verified library versions and alternatives as of 2026-02-26
- `.planning/codebase/CONCERNS.md` — known bad patterns to fix (in-memory rate limiter, openai_api_key plaintext storage)
- PyJWT 2.x docs: `jwt.encode()` returns `str` directly (no `.decode()`), `jwt.InvalidTokenError` is base exception

### Secondary (MEDIUM confidence)
- `.planning/STATE.md` — decisions: Fernet HKDF per-tenant keys, TenantAwareCollection auto-inject, Valkey SSL concern noted for Phase 4 (applies here too)
- `.planning/research/STACK.md` — slowapi 0.1.9 with Valkey backend; fastapi-mail 1.6.2 async SMTP; `cryptography>=46.0.0` Fernet verified
- `cryptography` HKDF docs: `from cryptography.hazmat.primitives.kdf.hkdf import HKDF` — standard key derivation function

### Tertiary (LOW confidence — verify before implementing)
- Valkey SSL URI scheme with slowapi/limits library: `valkeys://` vs `rediss://` — requires local test to confirm
- `fastapi-mail` 1.6.2 behavior with various SMTP providers (SendGrid, AWS SES) — verify SMTP config format

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — PyJWT, pwdlib, cryptography, motor already in pyproject.toml; verified working in Phase 1
- Architecture: HIGH — TenantAwareCollection pattern, BYOK encrypt/decrypt, JWT claims all verified against existing code and library docs
- Pitfalls: HIGH — most pitfalls derived from reading existing code (CONCERNS.md, security.py, tenant_isolation.py) not speculation
- Valkey SSL URI: LOW — requires empirical test before production deployment

**Research date:** 2026-02-26
**Valid until:** 2026-03-26 (stable libraries; 30-day window)
