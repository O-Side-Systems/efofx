"""
Auth API router for efOfX Estimation Service.

Provides registration, email verification, profile management, login, and
token refresh endpoints. The get_current_tenant dependency is imported from
app.core.security and supports both JWT bearer tokens and API key auth.
"""

import logging
from fastapi import APIRouter, BackgroundTasks, Depends, Query, Request
from slowapi.util import get_remote_address

from app.core.rate_limit import limiter
from app.core.security import get_current_tenant
from app.models.auth import (
    LoginRequest,
    LoginResponse,
    OpenAIKeyStatusResponse,
    ProfileUpdateRequest,
    ProfileResponse,
    RefreshRequest,
    RegisterRequest,
    RegisterResponse,
    StoreOpenAIKeyRequest,
    StoreOpenAIKeyResponse,
    TokenResponse,
    VerifyEmailResponse,
)
from app.models.tenant import Tenant
from app.services.auth_service import (
    get_profile,
    login_tenant,
    refresh_access_token,
    register_tenant,
    update_profile,
    verify_email,
)
from app.services.byok_service import (
    get_openai_key_status,
    validate_and_store_openai_key,
)

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/auth", tags=["auth"])


# ---------------------------------------------------------------------------
# Registration and email verification
# ---------------------------------------------------------------------------


@router.post("/register", response_model=RegisterResponse, status_code=201)
@limiter.limit("10/hour", key_func=get_remote_address)
async def register(
    request: Request,
    body: RegisterRequest,
    background_tasks: BackgroundTasks,
) -> RegisterResponse:
    """Register a new contractor account.

    Returns a one-time API key. Store it securely — it cannot be retrieved again.

    Rate limit: 10 registrations per hour per IP (anti-abuse).
    """
    return await register_tenant(body, background_tasks)


@router.get("/verify", response_model=VerifyEmailResponse)
async def verify(
    token: str = Query(..., description="Email verification token from the link"),
) -> VerifyEmailResponse:
    """Verify contractor email address using the link from the verification email."""
    return await verify_email(token)


# ---------------------------------------------------------------------------
# Login and token refresh (plan 02-02)
# ---------------------------------------------------------------------------


@router.post("/login", response_model=LoginResponse)
@limiter.limit("5/15minutes", key_func=get_remote_address)
async def login(request: Request, body: LoginRequest) -> LoginResponse:
    """Log in with email and password.

    Returns JWT access token (20-minute expiry) and refresh token (14-day expiry).

    Rate limit: 5 attempts per 15 minutes per IP (brute-force protection).

    Error responses:
    - 401: Invalid credentials (wrong password, nonexistent email, inactive account)
    - 403: Email not verified
    - 429: Rate limit exceeded (too many login attempts)
    """
    return await login_tenant(body)


@router.post("/refresh", response_model=TokenResponse)
@limiter.limit("30/minute", key_func=get_remote_address)
async def refresh(request: Request, body: RefreshRequest) -> TokenResponse:
    """Exchange a refresh token for a new access token.

    Token rotation: the submitted refresh token is invalidated and a new one
    is issued alongside the new access token.

    Rate limit: 30 per minute per IP (moderate protection).

    Error responses:
    - 401: Invalid or expired refresh token
    - 429: Rate limit exceeded
    """
    return await refresh_access_token(body)


# ---------------------------------------------------------------------------
# Profile management (protected — requires JWT or API key auth)
# ---------------------------------------------------------------------------


@router.get("/profile", response_model=ProfileResponse)
async def get_my_profile(
    tenant: Tenant = Depends(get_current_tenant),
) -> ProfileResponse:
    """Get the authenticated contractor's profile."""
    return await get_profile(tenant.tenant_id)


@router.patch("/profile", response_model=ProfileResponse)
async def update_my_profile(
    body: ProfileUpdateRequest,
    tenant: Tenant = Depends(get_current_tenant),
) -> ProfileResponse:
    """Update the authenticated contractor's profile (company name and/or settings)."""
    return await update_profile(tenant.tenant_id, body)


# ---------------------------------------------------------------------------
# BYOK key management (plan 02-04)
# ---------------------------------------------------------------------------


@router.put("/openai-key", response_model=StoreOpenAIKeyResponse)
async def store_openai_key(
    body: StoreOpenAIKeyRequest,
    tenant: Tenant = Depends(get_current_tenant),
) -> StoreOpenAIKeyResponse:
    """Store or rotate the authenticated contractor's BYOK OpenAI API key.

    The key is validated against OpenAI's models.list() endpoint before storage.
    It is then encrypted with a per-tenant HKDF-derived Fernet key and stored as
    ciphertext in the tenant document. The plaintext key is never persisted.

    This endpoint handles both initial storage and rotation (same behavior per
    locked decision — new key immediately overwrites old, no version history).

    Error responses:
    - 400: Invalid OpenAI API key (rejected by OpenAI)
    - 401: Not authenticated
    - 403: Email not verified
    - 422: Request body validation failure (key too short, etc.)
    - 503: OpenAI service unavailable during key validation
    """
    masked_key = await validate_and_store_openai_key(tenant.tenant_id, body.openai_key)
    return StoreOpenAIKeyResponse(
        masked_key=masked_key,
        message="OpenAI API key stored successfully",
    )


@router.get("/openai-key/status", response_model=OpenAIKeyStatusResponse)
async def openai_key_status(
    tenant: Tenant = Depends(get_current_tenant),
) -> OpenAIKeyStatusResponse:
    """Return BYOK key presence and masked display for the authenticated contractor.

    Does NOT decrypt the key — reads only the openai_key_last6 field for display.

    Returns:
        has_key: Whether an OpenAI API key is currently stored.
        masked_key: "sk-...{last6}" if stored, else None.
    """
    from app.db.mongodb import get_database
    from app.core.constants import DB_COLLECTIONS

    db = get_database()
    tenant_doc = await db[DB_COLLECTIONS["TENANTS"]].find_one(
        {"tenant_id": tenant.tenant_id}
    )
    status = get_openai_key_status(tenant_doc or {})
    return OpenAIKeyStatusResponse(**status)
