"""
Widget API endpoints for efOfX white-label widget.

Provides:
- GET /widget/branding/{api_key_prefix} — PUBLIC, no auth, rate-limited 30/min
- POST /widget/lead              — API key auth required
- POST /widget/analytics         — API key auth required (WSEC-02, WFTR-04)
- GET  /widget/analytics         — API key or JWT auth required
"""

import logging
from datetime import datetime, timedelta, timezone

from fastapi import APIRouter, Depends, HTTPException, Request
from fastapi.responses import Response
from pydantic import BaseModel

from app.core.constants import DB_COLLECTIONS
from app.core.rate_limit import limiter
from app.core.security import get_current_tenant
from app.db.mongodb import get_database
from app.models.tenant import Tenant
from app.models.widget import (
    BrandingConfigResponse,
    ConsultationRequest,
    ConsultationResponse,
    LeadCaptureRequest,
    LeadCaptureResponse,
)
from app.services.widget_service import (
    get_branding_by_prefix,
    record_analytics_event,
    save_consultation,
    save_lead,
)
from slowapi.util import get_remote_address

logger = logging.getLogger(__name__)

widget_router = APIRouter(prefix="/widget", tags=["widget"])

# Allowed analytics event types (WFTR-04)
VALID_EVENT_TYPES = {"widget_view", "chat_start", "estimate_complete"}


# ---------------------------------------------------------------------------
# Public branding endpoint — NO auth required
# ---------------------------------------------------------------------------


@widget_router.get(
    "/branding/{api_key_prefix}",
    response_model=BrandingConfigResponse,
    summary="Get contractor branding config",
    description=(
        "Returns the branding configuration for a contractor identified by their "
        "API key prefix. Public endpoint — no authentication required. "
        "Rate limited to 30 requests per minute per IP."
    ),
)
@limiter.limit("30/minute", key_func=get_remote_address)
async def get_branding(request: Request, api_key_prefix: str) -> BrandingConfigResponse:
    """Fetch branding config by API key prefix.

    The prefix is the 32-char hex portion of the API key (tenant_id without dashes).
    Returns 404 if no tenant matches.

    CRITICAL: This endpoint intentionally has NO auth dependency.
    The widget must be able to fetch branding before any user interaction.
    """
    branding = await get_branding_by_prefix(api_key_prefix)
    if branding is None:
        raise HTTPException(status_code=404, detail="Contractor not found")
    return branding


# ---------------------------------------------------------------------------
# Lead capture endpoint — API key auth required
# ---------------------------------------------------------------------------


@widget_router.post(
    "/lead",
    response_model=LeadCaptureResponse,
    summary="Capture a prospect lead",
    description="Save a prospect lead from the widget. Requires API key authentication.",
    status_code=201,
)
async def capture_lead(
    lead: LeadCaptureRequest,
    tenant: Tenant = Depends(get_current_tenant),
) -> LeadCaptureResponse:
    """Save a lead capture document for the authenticated tenant."""
    await save_lead(tenant.tenant_id, lead)
    logger.info("Lead captured for tenant %s, session %s", tenant.tenant_id, lead.session_id)
    return LeadCaptureResponse(
        message="Lead captured successfully",
        session_id=lead.session_id,
    )


# ---------------------------------------------------------------------------
# Consultation endpoint — API key auth required (DEBT-04)
# ---------------------------------------------------------------------------


@widget_router.post(
    "/consultation",
    response_model=ConsultationResponse,
    summary="Submit consultation request from widget",
    description=(
        "Save a consultation request from the widget contact form. "
        "Requires API key authentication. Returns 201 with the lead ID. "
        "Sends email notification to contractor if mail is configured; "
        "otherwise logs a warning and returns successfully."
    ),
    status_code=201,
)
async def submit_consultation(
    consultation: ConsultationRequest,
    tenant: Tenant = Depends(get_current_tenant),
) -> ConsultationResponse:
    """Save a consultation request and email the contractor.

    The lead is saved to widget_leads regardless of email configuration.
    Email failures are non-critical and do not affect the response.
    """
    lead_id = await save_consultation(
        tenant_id=tenant.tenant_id,
        consultation=consultation,
        contractor_email=tenant.email,
    )
    logger.info(
        "Consultation request saved for tenant %s, session %s, lead %s",
        tenant.tenant_id,
        consultation.session_id,
        lead_id,
    )
    return ConsultationResponse(lead_id=lead_id)


# ---------------------------------------------------------------------------
# Analytics endpoint — API key auth required
# ---------------------------------------------------------------------------


class AnalyticsRequest(BaseModel):
    """Request body for analytics event recording."""

    event_type: str


@widget_router.post(
    "/analytics",
    summary="Record a widget analytics event",
    description=(
        "Record a widget analytics event (widget_view, chat_start, estimate_complete). "
        "Requires API key authentication. Returns 204 No Content. "
        "Rejects unknown event_type values with 400."
    ),
    status_code=204,
)
async def record_event(
    body: AnalyticsRequest,
    tenant: Tenant = Depends(get_current_tenant),
) -> Response:
    """Record an analytics event for the authenticated tenant (fire-and-forget).

    Validates event_type against the allowed set (WSEC-02, WFTR-04).
    Returns 400 for invalid event types. Analytics failures are swallowed
    at the service layer — never propagated to widget users.
    """
    if body.event_type not in VALID_EVENT_TYPES:
        raise HTTPException(
            status_code=400,
            detail=f"Invalid event_type. Must be one of: {sorted(VALID_EVENT_TYPES)}",
        )
    await record_analytics_event(tenant.tenant_id, body.event_type)
    return Response(status_code=204)


# ---------------------------------------------------------------------------
# Analytics retrieval endpoint — API key or JWT auth required
# ---------------------------------------------------------------------------


@widget_router.get(
    "/analytics",
    summary="Get widget analytics",
    description=(
        "Get widget analytics for the authenticated tenant. "
        "Returns daily event counts for widget_view, chat_start, and estimate_complete. "
        "Analytics documents contain no PII — only tenant_id, date, and event counts."
    ),
)
@limiter.limit("10/minute")
async def get_analytics(
    request: Request,
    tenant: Tenant = Depends(get_current_tenant),
    days: int = 30,
) -> dict:
    """Return daily analytics counters for the last N days (default 30).

    Analytics documents contain only: tenant_id, date, widget_view count,
    chat_start count, estimate_complete count. No PII stored.
    """
    if days < 1 or days > 365:
        days = 30

    start_date = (datetime.now(timezone.utc) - timedelta(days=days)).strftime("%Y-%m-%d")
    db = get_database()
    cursor = (
        db[DB_COLLECTIONS["WIDGET_ANALYTICS"]]
        .find(
            {"tenant_id": tenant.tenant_id, "date": {"$gte": start_date}},
            {"_id": 0},  # Exclude MongoDB _id field
        )
        .sort("date", -1)
    )
    results = await cursor.to_list(length=days)
    return {"analytics": results, "days": days}
