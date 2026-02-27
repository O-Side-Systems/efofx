"""
Widget API endpoints for efOfX white-label widget.

Provides:
- GET /widget/branding/{api_key_prefix} — PUBLIC, no auth, rate-limited 30/min
- POST /widget/lead              — API key auth required
- POST /widget/analytics         — API key auth required
"""

import logging
from fastapi import APIRouter, Depends, HTTPException, Request
from fastapi.responses import Response

from app.core.rate_limit import limiter
from app.core.security import get_current_tenant
from app.models.tenant import Tenant
from app.models.widget import (
    BrandingConfigResponse,
    LeadCaptureRequest,
    LeadCaptureResponse,
)
from app.services.widget_service import (
    get_branding_by_prefix,
    record_analytics_event,
    save_lead,
)
from slowapi.util import get_remote_address

logger = logging.getLogger(__name__)

widget_router = APIRouter(prefix="/widget", tags=["widget"])


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
    inserted_id = await save_lead(tenant.tenant_id, lead)
    logger.info("Lead captured for tenant %s, session %s", tenant.tenant_id, lead.session_id)
    return LeadCaptureResponse(
        message="Lead captured successfully",
        session_id=lead.session_id,
    )


# ---------------------------------------------------------------------------
# Analytics endpoint — API key auth required
# ---------------------------------------------------------------------------


class AnalyticsEventRequest:
    """Simple inline model for analytics event body."""
    pass


from pydantic import BaseModel


class AnalyticsRequest(BaseModel):
    """Request body for analytics event recording."""

    event_type: str


@widget_router.post(
    "/analytics",
    summary="Record a widget analytics event",
    description=(
        "Record a widget analytics event (widget_view, chat_start, estimate_complete). "
        "Requires API key authentication. Returns 204 No Content."
    ),
    status_code=204,
)
async def record_event(
    body: AnalyticsRequest,
    tenant: Tenant = Depends(get_current_tenant),
) -> Response:
    """Record an analytics event for the authenticated tenant (fire-and-forget)."""
    await record_analytics_event(tenant.tenant_id, body.event_type)
    return Response(status_code=204)
