"""
Feedback email trigger API — contractor requests feedback from customer.

POST /api/v1/feedback/request-email/{session_id}
- Auth required (JWT/API key — contractor only)
- Creates magic link token
- Composes branded HTML email with estimate context
- Dispatches email via BackgroundTasks (non-blocking)
"""

import logging
import os

from fastapi import APIRouter, BackgroundTasks, Depends, HTTPException, Request
from jinja2 import Environment, FileSystemLoader
from pydantic import BaseModel, Field

from app.core.config import settings
from app.core.constants import DB_COLLECTIONS
from app.core.rate_limit import get_tenant_id_for_limit, get_tier_limit, limiter
from app.core.security import get_current_tenant
from app.db.mongodb import get_database
from app.models.tenant import Tenant
from app.services.feedback_email_service import FeedbackEmailService
from app.services.magic_link_service import MagicLinkService

logger = logging.getLogger(__name__)

feedback_email_router = APIRouter(prefix="/feedback", tags=["feedback"])

# Jinja2 template env — loaded once at module level
_templates_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "templates")
_jinja_env = Environment(loader=FileSystemLoader(_templates_dir), autoescape=True)


class FeedbackEmailRequest(BaseModel):
    """Request body for feedback email trigger."""

    customer_email: str = Field(
        ..., description="Customer email to send feedback request to"
    )
    project_name: str = Field(
        default="Your Project", description="Project name for email subject"
    )


class FeedbackEmailResponse(BaseModel):
    """Response after feedback email is queued."""

    message: str = "Feedback email queued"
    token_hash: str = Field(
        ..., description="Token hash for tracking (not the raw token)"
    )


@feedback_email_router.post(
    "/request-email/{session_id}", response_model=FeedbackEmailResponse
)
@limiter.limit(get_tier_limit, key_func=get_tenant_id_for_limit)
async def request_feedback_email(
    request: Request,
    session_id: str,
    body: FeedbackEmailRequest,
    background_tasks: BackgroundTasks,
    tenant: Tenant = Depends(get_current_tenant),
) -> FeedbackEmailResponse:
    """Contractor triggers feedback email for a delivered estimate.

    1. Validates the estimation session exists and belongs to tenant
    2. Creates magic link token (hash stored in DB)
    3. Loads estimate data + tenant branding
    4. Composes branded HTML email with estimate context
    5. Queues email send via BackgroundTasks (non-blocking)
    """
    db = get_database()

    # 1. Validate session exists and belongs to tenant
    session_doc = await db[DB_COLLECTIONS["ESTIMATES"]].find_one(
        {"session_id": session_id, "tenant_id": tenant.tenant_id}
    )
    if not session_doc:
        raise HTTPException(status_code=404, detail="Estimation session not found")

    # 2. Create magic link
    magic_link_svc = MagicLinkService()
    raw_token, token_hash = await magic_link_svc.create_magic_link(
        tenant_id=tenant.tenant_id,
        estimation_session_id=session_id,
        customer_email=body.customer_email,
        project_name=body.project_name,
    )
    magic_link_url = f"{settings.APP_BASE_URL}/feedback/form/{raw_token}"

    # 3. Load branding from tenant settings
    branding = tenant.settings.get("branding", {}) if tenant.settings else {}
    company_name = branding.get(
        "company_name", tenant.company_name or "Your Contractor"
    )
    logo_url = branding.get("logo_url")
    primary_color = branding.get("primary_color", "#2563eb")
    accent_color = branding.get("accent_color", "#1d4ed8")

    # 4. Extract estimate data from session document.
    # The estimation output may be stored in the session's result field or
    # we reconstruct from stored fields. Check for a nested estimation_output
    # or result dict in the session document.
    estimate_data = (
        session_doc.get("estimation_output") or session_doc.get("result") or {}
    )

    total_cost_p50 = estimate_data.get("total_cost_p50", 0)
    total_cost_p80 = estimate_data.get("total_cost_p80", 0)
    timeline_weeks_p50 = estimate_data.get("timeline_weeks_p50", 0)
    timeline_weeks_p80 = estimate_data.get("timeline_weeks_p80", 0)
    cost_breakdown = estimate_data.get("cost_breakdown", [])
    assumptions = estimate_data.get("assumptions", [])

    # 5. Render email HTML
    template = _jinja_env.get_template("feedback_email.html")
    html_body = template.render(
        company_name=company_name,
        logo_url=logo_url,
        primary_color=primary_color,
        accent_color=accent_color,
        project_name=body.project_name,
        total_cost_p50=total_cost_p50,
        total_cost_p80=total_cost_p80,
        timeline_weeks_p50=timeline_weeks_p50,
        timeline_weeks_p80=timeline_weeks_p80,
        cost_breakdown=cost_breakdown,
        assumptions=assumptions,
        magic_link_url=magic_link_url,
    )

    # 6. Queue email dispatch (non-blocking)
    email_svc = FeedbackEmailService()
    subject = f"How did {body.project_name} go?"

    async def _send() -> None:
        await email_svc.send_email(
            to_email=body.customer_email,
            subject=subject,
            html_body=html_body,
            magic_link_url=magic_link_url,
        )

    background_tasks.add_task(_send)

    logger.info(
        "Feedback email queued for session %s (tenant %s) -> %s",
        session_id,
        tenant.tenant_id,
        body.customer_email,
    )

    return FeedbackEmailResponse(message="Feedback email queued", token_hash=token_hash)
