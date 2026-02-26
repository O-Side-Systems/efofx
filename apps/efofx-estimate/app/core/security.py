"""
Security utilities for efOfX Estimation Service.

Provides the get_current_tenant FastAPI dependency used by every protected
endpoint. Supports both JWT bearer tokens and API key authentication.

RateLimiter removed — will be replaced by slowapi in plan 02-05.
AuthService class removed — replaced by standalone functions in auth_service.py.
"""

import logging

import jwt  # PyJWT 2.x
from fastapi import Depends, HTTPException
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer
from pwdlib import PasswordHash
from pwdlib.hashers.bcrypt import BcryptHasher

from app.core.config import settings
from app.core.constants import DB_COLLECTIONS
from app.db.mongodb import get_database
from app.models.tenant import Tenant

logger = logging.getLogger(__name__)

# HTTP Bearer scheme — auto_error=False lets us return custom 401 messages
security = HTTPBearer(auto_error=False)

# Bcrypt hasher for API key verification
_password_hash = PasswordHash((BcryptHasher(),))


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------


async def _validate_api_key(raw_key: str) -> Tenant:
    """Validate a raw API key (sk_live_... format) and return the Tenant.

    API key format: sk_live_{tenant_id_no_dashes}_{random}
    The first 32 chars after "sk_live_" are the tenant_id without dashes,
    enabling O(1) tenant lookup without a full-collection bcrypt scan.

    Raises:
        HTTPException 401: If the key is invalid or the tenant is not found.
        HTTPException 403: If the tenant's email is not verified.
    """
    # Format: sk_live_{32-char-tenant-id-no-dashes}_{random}
    parts = raw_key.split("_")
    # parts[0]="sk", parts[1]="live", parts[2]={tenant_id_no_dashes+random...}
    if len(parts) >= 3:
        candidate_raw = parts[2][:32]  # first 32 hex chars = tenant_id no dashes
        try:
            # Reconstruct UUID format from 32 hex chars
            tid = (
                f"{candidate_raw[0:8]}-"
                f"{candidate_raw[8:12]}-"
                f"{candidate_raw[12:16]}-"
                f"{candidate_raw[16:20]}-"
                f"{candidate_raw[20:32]}"
            )
            db = get_database()
            tenant_doc = await db[DB_COLLECTIONS["TENANTS"]].find_one({"tenant_id": tid})

            if tenant_doc and _password_hash.verify(raw_key, tenant_doc["hashed_api_key"]):
                if not tenant_doc.get("email_verified", False):
                    raise HTTPException(
                        status_code=403,
                        detail="Email not verified. Please verify your email before using the API.",
                    )
                return Tenant(**tenant_doc)
        except HTTPException:
            raise
        except Exception:
            pass  # Fall through to 401

    raise HTTPException(status_code=401, detail="Invalid token")


# ---------------------------------------------------------------------------
# Public dependency
# ---------------------------------------------------------------------------


async def get_current_tenant(
    credentials: HTTPAuthorizationCredentials = Depends(security),
) -> Tenant:
    """FastAPI dependency: authenticate the request and return the Tenant.

    Supports two authentication schemes:
    1. API key: Bearer sk_live_... (parsed and bcrypt-verified)
    2. JWT: Bearer <signed-jwt> (decoded, claims validated, email_verified checked)

    HTTP status codes:
    - 401 "Authentication required": No Authorization header present.
    - 401 "Token expired": JWT has passed its expiry time.
    - 401 "Invalid token": JWT signature invalid, malformed, or missing required claims;
                           also returned for invalid API keys (no enumeration).
    - 403 "Email not verified": Valid credentials but email_verified=False.
    """
    if not credentials:
        raise HTTPException(status_code=401, detail="Authentication required")

    token = credentials.credentials

    # -------------------------------------------------------------------
    # Route 1: API key authentication (sk_live_ prefix)
    # -------------------------------------------------------------------
    if token.startswith("sk_live_"):
        return await _validate_api_key(token)

    # -------------------------------------------------------------------
    # Route 2: JWT authentication
    # -------------------------------------------------------------------
    try:
        payload = jwt.decode(
            token,
            settings.JWT_SECRET_KEY,
            algorithms=["HS256"],
            options={"require": ["exp", "iat", "sub", "tenant_id", "role"]},
        )
    except jwt.ExpiredSignatureError:
        raise HTTPException(status_code=401, detail="Token expired")
    except jwt.InvalidTokenError:
        raise HTTPException(status_code=401, detail="Invalid token")

    db = get_database()
    tenant_doc = await db[DB_COLLECTIONS["TENANTS"]].find_one(
        {"tenant_id": payload["tenant_id"]}
    )

    if not tenant_doc:
        raise HTTPException(status_code=401, detail="Invalid token")

    if not tenant_doc.get("email_verified", False):
        raise HTTPException(status_code=403, detail="Email not verified")

    return Tenant(**tenant_doc)


async def get_current_tenant_optional(
    credentials: HTTPAuthorizationCredentials = Depends(security),
) -> Tenant | None:
    """Optional variant of get_current_tenant — returns None if no credentials provided.

    Use on endpoints that have different behavior for authenticated vs anonymous callers.
    """
    if not credentials:
        return None
    return await get_current_tenant(credentials)
