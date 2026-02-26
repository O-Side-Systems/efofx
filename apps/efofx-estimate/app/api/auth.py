"""
Auth API router for efOfX Estimation Service.

Provides registration, email verification, profile management, login, and
token refresh endpoints. The get_current_tenant dependency is imported from
app.core.security and supports both JWT bearer tokens and API key auth.
"""

import logging
from fastapi import APIRouter, BackgroundTasks, Depends, Query

from app.core.security import get_current_tenant
from app.models.auth import (
    LoginRequest,
    LoginResponse,
    ProfileUpdateRequest,
    ProfileResponse,
    RefreshRequest,
    RegisterRequest,
    RegisterResponse,
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

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/auth", tags=["auth"])


# ---------------------------------------------------------------------------
# Registration and email verification
# ---------------------------------------------------------------------------


@router.post("/register", response_model=RegisterResponse, status_code=201)
async def register(
    body: RegisterRequest,
    background_tasks: BackgroundTasks,
) -> RegisterResponse:
    """Register a new contractor account.

    Returns a one-time API key. Store it securely — it cannot be retrieved again.
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
async def login(body: LoginRequest) -> LoginResponse:
    """Log in with email and password.

    Returns JWT access token (20-minute expiry) and refresh token (14-day expiry).

    Error responses:
    - 401: Invalid credentials (wrong password, nonexistent email, inactive account)
    - 403: Email not verified
    """
    return await login_tenant(body)


@router.post("/refresh", response_model=TokenResponse)
async def refresh(body: RefreshRequest) -> TokenResponse:
    """Exchange a refresh token for a new access token.

    Token rotation: the submitted refresh token is invalidated and a new one
    is issued alongside the new access token.

    Error responses:
    - 401: Invalid or expired refresh token
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
