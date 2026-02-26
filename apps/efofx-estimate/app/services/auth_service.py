"""
Authentication service for efOfX Estimation Service.

Handles contractor registration, email verification, and profile management.
API key shown exactly once at registration — bcrypt-hashed for storage.
"""

import logging
import secrets
import uuid
from datetime import datetime, timedelta, timezone
from typing import Optional, Tuple

from fastapi import HTTPException, BackgroundTasks
from pwdlib import PasswordHash
from pwdlib.hashers.bcrypt import BcryptHasher

from app.core.config import settings
from app.core.constants import DB_COLLECTIONS
from app.db.mongodb import get_database
from app.models.auth import (
    RegisterRequest,
    RegisterResponse,
    ProfileUpdateRequest,
    ProfileResponse,
    VerifyEmailResponse,
)

logger = logging.getLogger(__name__)

# Use bcrypt explicitly (argon2 not installed; bcrypt is the Phase 1 dependency)
_password_hash = PasswordHash((BcryptHasher(),))


# ---------------------------------------------------------------------------
# Helper functions
# ---------------------------------------------------------------------------


def generate_verification_token() -> Tuple[str, datetime]:
    """Generate a one-use verification token with 24h expiry."""
    token = secrets.token_urlsafe(32)
    expires_at = datetime.now(timezone.utc) + timedelta(hours=24)
    return token, expires_at


async def send_verification_email(email: str, token: str) -> None:
    """Send email verification link.

    If SMTP is not configured (SMTP_USERNAME is None), log a warning and skip.
    This allows local development without SMTP setup.
    """
    if not settings.SMTP_USERNAME:
        logger.warning(
            "SMTP not configured — skipping verification email for %s. "
            "Set SMTP_USERNAME to enable emails. "
            "Token for dev testing: %s",
            email,
            token,
        )
        return

    try:
        from fastapi_mail import FastMail, MessageSchema, ConnectionConfig, MessageType

        conf = ConnectionConfig(
            MAIL_USERNAME=settings.SMTP_USERNAME,
            MAIL_PASSWORD=settings.SMTP_PASSWORD,
            MAIL_FROM=settings.SMTP_FROM,
            MAIL_PORT=settings.SMTP_PORT,
            MAIL_SERVER=settings.SMTP_SERVER,
            MAIL_STARTTLS=True,
            MAIL_SSL_TLS=False,
            USE_CREDENTIALS=True,
        )

        verification_url = (
            f"{settings.APP_BASE_URL}/api/v1/auth/verify?token={token}"
        )

        body = (
            f"Hello,\n\n"
            f"Please verify your email address to activate your efOfX account:\n\n"
            f"{verification_url}\n\n"
            f"This link expires in 24 hours.\n\n"
            f"If you did not register for efOfX, you can ignore this email."
        )

        message = MessageSchema(
            subject="Verify your efOfX account",
            recipients=[email],
            body=body,
            subtype=MessageType.plain,
        )

        fm = FastMail(conf)
        await fm.send_message(message)
        logger.info("Verification email sent to %s", email)

    except Exception as exc:  # noqa: BLE001
        logger.error("Failed to send verification email to %s: %s", email, exc)
        # Don't raise — registration should succeed even if email fails


# ---------------------------------------------------------------------------
# Registration
# ---------------------------------------------------------------------------


async def register_tenant(
    body: RegisterRequest,
    background_tasks: BackgroundTasks,
) -> RegisterResponse:
    """Register a new contractor tenant.

    Security: Returns an identical response for duplicate emails to prevent
    email enumeration (locked decision from CONTEXT.md).
    """
    db = get_database()
    tenants = db[DB_COLLECTIONS["TENANTS"]]

    generic_message = (
        "If this email is new, check your inbox for a verification link."
    )

    # Check for existing email — return generic response to prevent enumeration
    existing = await tenants.find_one({"email": body.email})
    if existing:
        # Return 201 with generic message — identical to success response
        # Use a placeholder api_key that looks realistic but is useless
        return RegisterResponse(
            tenant_id=existing["tenant_id"],
            api_key="sk_live_already_registered",
            message=generic_message,
        )

    # Generate identifiers
    tenant_id = str(uuid.uuid4())

    # Hash password
    hashed_password = _password_hash.hash(body.password)

    # Generate API key — shown once, then bcrypt-hashed
    raw_api_key = f"sk_live_{tenant_id.replace('-', '')}_{secrets.token_urlsafe(16)}"
    hashed_api_key = _password_hash.hash(raw_api_key)
    api_key_last6 = raw_api_key[-6:]

    # Build tenant document
    now = datetime.now(timezone.utc)
    tenant_doc = {
        "tenant_id": tenant_id,
        "company_name": body.company_name,
        "email": body.email,
        "hashed_password": hashed_password,
        "hashed_api_key": hashed_api_key,
        "api_key_last6": api_key_last6,
        "tier": "trial",
        "email_verified": False,
        "encrypted_openai_key": None,
        "is_active": True,
        "created_at": now,
        "updated_at": now,
        "settings": {},
    }

    await tenants.insert_one(tenant_doc)
    logger.info("New tenant registered: tenant_id=%s email=%s", tenant_id, body.email)

    # Generate and store email verification token
    token, expires_at = generate_verification_token()
    verification_tokens = db[DB_COLLECTIONS["VERIFICATION_TOKENS"]]
    await verification_tokens.insert_one(
        {
            "token": token,
            "tenant_id": tenant_id,
            "email": body.email,
            "expires_at": expires_at,
        }
    )

    # Send verification email in background
    background_tasks.add_task(send_verification_email, body.email, token)

    return RegisterResponse(
        tenant_id=tenant_id,
        api_key=raw_api_key,
        message=generic_message,
    )


# ---------------------------------------------------------------------------
# Email verification
# ---------------------------------------------------------------------------


async def verify_email(token: str) -> VerifyEmailResponse:
    """Verify a contractor's email address using a one-time token."""
    db = get_database()
    verification_tokens = db[DB_COLLECTIONS["VERIFICATION_TOKENS"]]

    token_doc = await verification_tokens.find_one({"token": token})

    if not token_doc:
        raise HTTPException(status_code=400, detail="Invalid or expired verification token")

    # Check expiry
    expires_at = token_doc["expires_at"]
    if expires_at.tzinfo is None:
        expires_at = expires_at.replace(tzinfo=timezone.utc)
    if datetime.now(timezone.utc) > expires_at:
        # Clean up expired token
        await verification_tokens.delete_one({"token": token})
        raise HTTPException(status_code=400, detail="Invalid or expired verification token")

    tenant_id = token_doc["tenant_id"]

    # Mark tenant as verified
    tenants = db[DB_COLLECTIONS["TENANTS"]]
    result = await tenants.update_one(
        {"tenant_id": tenant_id},
        {"$set": {"email_verified": True, "updated_at": datetime.now(timezone.utc)}},
    )

    if result.matched_count == 0:
        raise HTTPException(status_code=400, detail="Invalid or expired verification token")

    # Delete the used token (single-use)
    await verification_tokens.delete_one({"token": token})

    logger.info("Email verified for tenant_id=%s", tenant_id)
    return VerifyEmailResponse(message="Email verified successfully.", email_verified=True)


# ---------------------------------------------------------------------------
# Profile management
# ---------------------------------------------------------------------------


def _build_profile_response(tenant_doc: dict) -> ProfileResponse:
    """Build ProfileResponse from a tenant document."""
    last6 = tenant_doc.get("api_key_last6", "")
    masked_api_key = f"sk-...{last6}" if last6 else "sk-...******"
    return ProfileResponse(
        tenant_id=tenant_doc["tenant_id"],
        company_name=tenant_doc["company_name"],
        email=tenant_doc["email"],
        tier=tenant_doc["tier"],
        email_verified=tenant_doc["email_verified"],
        created_at=tenant_doc["created_at"],
        masked_api_key=masked_api_key,
        has_openai_key=tenant_doc.get("encrypted_openai_key") is not None,
    )


async def get_profile(tenant_id: str) -> ProfileResponse:
    """Get tenant profile by tenant_id."""
    db = get_database()
    tenants = db[DB_COLLECTIONS["TENANTS"]]
    tenant_doc = await tenants.find_one({"tenant_id": tenant_id})

    if not tenant_doc:
        raise HTTPException(status_code=404, detail="Tenant not found")

    return _build_profile_response(tenant_doc)


async def update_profile(tenant_id: str, body: ProfileUpdateRequest) -> ProfileResponse:
    """Update allowed tenant profile fields (company_name, settings)."""
    db = get_database()
    tenants = db[DB_COLLECTIONS["TENANTS"]]

    update_fields: dict = {"updated_at": datetime.now(timezone.utc)}
    if body.company_name is not None:
        update_fields["company_name"] = body.company_name
    if body.settings is not None:
        update_fields["settings"] = body.settings

    result = await tenants.update_one(
        {"tenant_id": tenant_id},
        {"$set": update_fields},
    )

    if result.matched_count == 0:
        raise HTTPException(status_code=404, detail="Tenant not found")

    return await get_profile(tenant_id)
