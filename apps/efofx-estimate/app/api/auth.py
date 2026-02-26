"""
Auth API router for efOfX Estimation Service.

Provides registration, email verification, and profile endpoints.
Login and token refresh are implemented in plan 02-02.
"""

import logging
from fastapi import APIRouter, BackgroundTasks, Depends, HTTPException, Query
from fastapi.security import HTTPBearer, HTTPAuthorizationCredentials
from pwdlib import PasswordHash
from pwdlib.hashers.bcrypt import BcryptHasher

from app.core.constants import HTTP_STATUS, DB_COLLECTIONS
from app.db.mongodb import get_database
from app.models.auth import (
    RegisterRequest,
    RegisterResponse,
    ProfileUpdateRequest,
    ProfileResponse,
    VerifyEmailResponse,
)
from app.services.auth_service import (
    register_tenant,
    verify_email,
    get_profile,
    update_profile,
)

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/auth", tags=["auth"])

_security = HTTPBearer(auto_error=False)
_password_hash = PasswordHash((BcryptHasher(),))


# ---------------------------------------------------------------------------
# Auth dependency — API key validation
# ---------------------------------------------------------------------------


async def get_current_tenant(
    credentials: HTTPAuthorizationCredentials = Depends(_security),
) -> dict:
    """Validate Bearer API key and return tenant document.

    Accepts sk_live_... API keys via Authorization: Bearer header.
    JWT-based auth will be added in plan 02-02.

    Returns the raw tenant dict from MongoDB.
    """
    if not credentials or not credentials.credentials:
        raise HTTPException(
            status_code=HTTP_STATUS["UNAUTHORIZED"],
            detail="Authentication credentials required",
        )

    token = credentials.credentials

    # Only API keys are supported at this stage (sk_live_ prefix)
    if not token.startswith("sk_live_"):
        raise HTTPException(
            status_code=HTTP_STATUS["UNAUTHORIZED"],
            detail="Invalid credentials",
        )

    db = get_database()
    tenants = db[DB_COLLECTIONS["TENANTS"]]

    # We must iterate through tenants to find API key match (bcrypt)
    # This is O(n) but acceptable for now — plan 02-02 adds JWT-based auth
    # In practice, the tenant count is small during beta
    #
    # Security note: bcrypt verification is constant-time per check.
    # To avoid O(n) scan, the API key encodes the tenant_id:
    # sk_live_{tenant_id_no_dashes}_{random}
    # We can extract tenant_id from key prefix for fast lookup.

    raw_key = token
    parts = raw_key.split("_")
    # Format: sk_live_{32-char tenant_id no dashes}_{random}
    # parts[0]="sk", parts[1]="live", parts[2]={tenant_id_no_dashes+random...}
    # The tenant_id without dashes is 32 hex chars
    if len(parts) >= 3:
        candidate_tenant_id_raw = parts[2][:32]  # first 32 chars after "sk_live_"
        # Reconstruct UUID format
        try:
            tid = (
                f"{candidate_tenant_id_raw[0:8]}-"
                f"{candidate_tenant_id_raw[8:12]}-"
                f"{candidate_tenant_id_raw[12:16]}-"
                f"{candidate_tenant_id_raw[16:20]}-"
                f"{candidate_tenant_id_raw[20:32]}"
            )
            tenant_doc = await tenants.find_one({"tenant_id": tid})
            if tenant_doc and _password_hash.verify(raw_key, tenant_doc["hashed_api_key"]):
                # Enforce email verification for API access
                if not tenant_doc.get("email_verified", False):
                    raise HTTPException(
                        status_code=HTTP_STATUS["FORBIDDEN"],
                        detail="Email not verified. Please verify your email before using the API.",
                    )
                return tenant_doc
        except HTTPException:
            raise
        except Exception:
            pass  # Fall through to invalid credentials

    raise HTTPException(
        status_code=HTTP_STATUS["UNAUTHORIZED"],
        detail="Invalid credentials",
    )


# ---------------------------------------------------------------------------
# Endpoints
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


@router.get("/profile", response_model=ProfileResponse)
async def get_my_profile(
    tenant: dict = Depends(get_current_tenant),
) -> ProfileResponse:
    """Get the authenticated contractor's profile."""
    return await get_profile(tenant["tenant_id"])


@router.patch("/profile", response_model=ProfileResponse)
async def update_my_profile(
    body: ProfileUpdateRequest,
    tenant: dict = Depends(get_current_tenant),
) -> ProfileResponse:
    """Update the authenticated contractor's profile (company name and/or settings)."""
    return await update_profile(tenant["tenant_id"], body)
