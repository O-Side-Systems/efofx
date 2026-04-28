"""
Widget service for efOfX white-label widget.

Provides business logic for:
- Fetching branding config by API key prefix (public, no auth)
- Saving lead captures (authenticated)
- Recording analytics events (authenticated, fire-and-forget)
- Populating the per-tenant CORS origins cache
"""

import logging
from datetime import datetime, timezone
from typing import Optional

from app.core.config import settings
from app.core.constants import DB_COLLECTIONS
from app.db.mongodb import get_database, get_tenant_collection
from app.models.widget import (
    BrandingConfig,
    BrandingConfigResponse,
    ConsultationRequest,
    LeadCapture,
    LeadCaptureRequest,
)

logger = logging.getLogger(__name__)


def _prefix_to_tenant_id(api_key_prefix: str) -> str:
    """Reconstruct UUID from 32-char hex prefix (tenant_id without dashes).

    Format: {0:8}-{8:12}-{12:16}-{16:20}-{20:32}
    Same pattern as _validate_api_key in security.py.
    """
    p = api_key_prefix
    return f"{p[0:8]}-{p[8:12]}-{p[12:16]}-{p[16:20]}-{p[20:32]}"


async def get_branding_by_prefix(
    api_key_prefix: str,
) -> Optional[BrandingConfigResponse]:
    """Fetch branding config for a tenant identified by API key prefix.

    The prefix is the first 32 hex chars of the API key after 'sk_live_',
    which equals the tenant_id without dashes. Reconstructs the UUID and
    queries the tenants collection.

    Returns BrandingConfigResponse with safe fields only — never includes
    hashed keys, encrypted BYOK keys, passwords, or email.

    Returns None if tenant is not found.
    """
    try:
        tid = _prefix_to_tenant_id(api_key_prefix)
    except (IndexError, ValueError):
        return None

    db = get_database()
    tenant_doc = await db[DB_COLLECTIONS["TENANTS"]].find_one({"tenant_id": tid})

    if not tenant_doc:
        return None

    # Merge stored branding settings with defaults
    branding_settings = tenant_doc.get("settings", {}).get("branding", {})
    branding = BrandingConfig(**branding_settings)

    # company_name: prefer branding override, fall back to tenant's company_name
    company_name = branding_settings.get(
        "company_name", tenant_doc.get("company_name", "")
    )

    # Populate CORS cache lazily — no async DB call in middleware
    from app.middleware.cors import _tenant_origins_cache

    allowed_origins = tenant_doc.get("settings", {}).get("allowed_origins", [])
    if allowed_origins:
        _tenant_origins_cache[tid] = allowed_origins

    return BrandingConfigResponse(
        primary_color=branding.primary_color,
        secondary_color=branding.secondary_color,
        accent_color=branding.accent_color,
        logo_url=branding.logo_url,
        welcome_message=branding.welcome_message,
        button_text=branding.button_text,
        company_name=company_name,
        locale=branding.locale,
        consultation_form_labels=branding.consultation_form_labels,
    )


async def save_lead(tenant_id: str, lead: LeadCaptureRequest) -> str:
    """Save a lead capture document to the widget_leads collection.

    Uses TenantAwareCollection for automatic tenant_id injection.
    Returns the inserted document ID as a string.
    """
    lead_doc = LeadCapture(
        session_id=lead.session_id,
        tenant_id=tenant_id,
        name=lead.name,
        email=lead.email,
        phone=lead.phone,
        captured_at=datetime.now(timezone.utc),
    )

    col = get_tenant_collection(DB_COLLECTIONS["WIDGET_LEADS"], tenant_id)
    result = await col.insert_one(lead_doc.model_dump())
    return str(result.inserted_id)


async def save_consultation(
    tenant_id: str, consultation: ConsultationRequest, contractor_email: str
) -> str:
    """Save consultation request to widget_leads and email the contractor (DEBT-04).

    Saves the lead document first, then attempts email notification.
    If email settings are not configured, logs a warning and returns successfully.
    Email failures do NOT cause the endpoint to fail — the lead is already saved.
    """
    lead_doc = {
        "tenant_id": tenant_id,
        "session_id": consultation.session_id,
        "name": consultation.name,
        "email": consultation.email,
        "phone": consultation.phone,
        "message": consultation.message,
        "lead_type": "consultation",
        "captured_at": datetime.now(timezone.utc),
    }
    col = get_tenant_collection(DB_COLLECTIONS["WIDGET_LEADS"], tenant_id)
    result = await col.insert_one(lead_doc)
    lead_id = str(result.inserted_id)

    # Attempt email notification — failure is non-critical; lead is already saved
    await _send_consultation_email(contractor_email, consultation)

    return lead_id


async def _send_consultation_email(
    to_email: str, consultation: ConsultationRequest
) -> None:
    """Send consultation notification email to contractor. Silently skips if not configured."""  # noqa: E501
    try:
        if not settings.MAIL_SERVER or not settings.MAIL_USERNAME:
            logger.warning(
                "DEBT-04: Email not configured (MAIL_SERVER/MAIL_USERNAME not set) "
                "— skipping consultation notification email"
            )
            return

        from fastapi_mail import FastMail, MessageSchema, ConnectionConfig, MessageType

        conf = ConnectionConfig(
            MAIL_USERNAME=settings.MAIL_USERNAME,
            MAIL_PASSWORD=settings.MAIL_PASSWORD or "",
            MAIL_FROM=settings.MAIL_FROM or settings.MAIL_USERNAME,
            MAIL_PORT=settings.MAIL_PORT,
            MAIL_SERVER=settings.MAIL_SERVER,
            MAIL_STARTTLS=True,
            MAIL_SSL_TLS=False,
            USE_CREDENTIALS=True,
        )

        body = (
            f"New consultation request from your estimation widget:\n\n"
            f"Name: {consultation.name}\n"
            f"Email: {consultation.email}\n"
            f"Phone: {consultation.phone}\n\n"
            f"Message:\n{consultation.message}\n"
        )

        message = MessageSchema(
            subject="New consultation request from your widget",
            recipients=[to_email],
            body=body,
            subtype=MessageType.plain,
        )

        fm = FastMail(conf)
        await fm.send_message(message)
        logger.info("Consultation email sent to %s", to_email)
    except Exception as e:
        logger.error("Failed to send consultation email: %s", str(e))
        # Do NOT re-raise — lead is already saved, email failure is non-critical


async def record_analytics_event(tenant_id: str, event_type: str) -> None:
    """Record an analytics event using daily bucketing and $inc upsert.

    Fire-and-forget: callers do not need to await errors from this function.
    Uses TenantAwareCollection for automatic tenant_id injection.
    """
    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")

    try:
        col = get_tenant_collection(DB_COLLECTIONS["WIDGET_ANALYTICS"], tenant_id)
        await col.update_one(
            {"date": today},
            {"$inc": {event_type: 1}},
            upsert=True,
        )
    except Exception as exc:
        # Analytics failures must not propagate — fire-and-forget
        logger.warning("Analytics event recording failed: %s", exc)
