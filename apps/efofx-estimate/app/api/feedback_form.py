"""
Customer feedback form endpoints — PUBLIC (no auth).

GET  /feedback/form/{token} — Renders form (valid), expired page, or thank-you page
POST /feedback/form/{token} — Accepts form submission, stores feedback with estimate snapshot  # noqa: E501

These are served as HTML pages via Jinja2 templates, NOT JSON API responses.
The magic link token in the URL is the sole authentication mechanism.
"""

import logging
import os
from typing import Optional

from fastapi import APIRouter, Form
from fastapi.responses import HTMLResponse
from jinja2 import Environment, FileSystemLoader

from app.core.constants import DB_COLLECTIONS
from app.db.mongodb import get_database
from app.models.feedback import (
    DiscrepancyReason,
    EstimateSnapshot,
    FeedbackSubmission,
)
from app.services.feedback_service import FeedbackService
from app.services.magic_link_service import MagicLinkService

logger = logging.getLogger(__name__)

feedback_form_router = APIRouter(tags=["feedback-form"])

_templates_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "templates")
_jinja_env = Environment(loader=FileSystemLoader(_templates_dir), autoescape=True)

# Discrepancy reason labels for the form dropdown
DISCREPANCY_LABELS = [
    (DiscrepancyReason.SCOPE_CHANGED.value, "Scope changed"),
    (DiscrepancyReason.UNFORESEEN_ISSUES.value, "Unforeseen issues"),
    (DiscrepancyReason.TIMELINE_PRESSURE.value, "Timeline pressure"),
    (DiscrepancyReason.VENDOR_MATERIAL_COSTS.value, "Vendor/material costs"),
    (DiscrepancyReason.CLIENT_CHANGES.value, "Client changes"),
    (DiscrepancyReason.ESTIMATE_WAS_ACCURATE.value, "Estimate was accurate"),
]


async def _load_estimate_context(token_doc: dict) -> dict:
    """Load estimate data and tenant branding for form display."""
    db = get_database()

    # Load estimation session
    session_doc = await db[DB_COLLECTIONS["ESTIMATES"]].find_one(
        {
            "session_id": token_doc["estimation_session_id"],
            "tenant_id": token_doc["tenant_id"],
        }
    )
    estimate_data = {}
    reference_class_id = None
    if session_doc:
        estimate_data = (
            session_doc.get("estimation_output") or session_doc.get("result") or {}
        )
        reference_class_id = session_doc.get("reference_class")

    # Load tenant branding
    tenant_doc = await db[DB_COLLECTIONS["TENANTS"]].find_one(
        {"tenant_id": token_doc["tenant_id"]}
    )
    branding = {}
    if tenant_doc and tenant_doc.get("settings"):
        branding = tenant_doc["settings"].get("branding", {})

    company_name = branding.get("company_name", "")
    if not company_name and tenant_doc:
        company_name = tenant_doc.get("company_name", "Your Contractor")

    return {
        "company_name": company_name,
        "logo_url": branding.get("logo_url"),
        "primary_color": branding.get("primary_color", "#2563eb"),
        "accent_color": branding.get("accent_color", "#1d4ed8"),
        "secondary_color": branding.get("secondary_color", "#f3f4f6"),
        "project_name": token_doc.get("project_name", "Your Project"),
        "customer_email": token_doc.get("customer_email", ""),
        "total_cost_p50": estimate_data.get("total_cost_p50", 0),
        "total_cost_p80": estimate_data.get("total_cost_p80", 0),
        "timeline_weeks_p50": estimate_data.get("timeline_weeks_p50", 0),
        "timeline_weeks_p80": estimate_data.get("timeline_weeks_p80", 0),
        "cost_breakdown": estimate_data.get("cost_breakdown", []),
        "assumptions": estimate_data.get("assumptions", []),
        "reference_class_id": reference_class_id,
        "estimate_data": estimate_data,  # full dict for snapshot
    }


@feedback_form_router.get("/feedback/form/{token}", response_class=HTMLResponse)
async def get_feedback_form(token: str) -> HTMLResponse:
    """Render feedback form, expired page, or thank-you page.

    Idempotent: sets opened_at on first visit but NEVER sets used_at.
    Email security scanners can follow this URL without consuming the token.
    """
    magic_link_svc = MagicLinkService()
    state, token_doc = await magic_link_svc.resolve_token_state(token)

    if state in ("not_found", "expired"):
        template = _jinja_env.get_template("feedback_expired.html")
        return HTMLResponse(template.render())

    if state == "used":
        ctx = await _load_estimate_context(token_doc)
        template = _jinja_env.get_template("feedback_submitted.html")
        return HTMLResponse(template.render(company_name=ctx["company_name"]))

    # state == "valid" — mark as opened (idempotent) and render form
    await magic_link_svc.mark_opened(token)

    ctx = await _load_estimate_context(token_doc)
    template = _jinja_env.get_template("feedback_form.html")
    return HTMLResponse(
        template.render(
            token=token,
            discrepancy_reasons=DISCREPANCY_LABELS,
            **ctx,
        )
    )


@feedback_form_router.post("/feedback/form/{token}", response_class=HTMLResponse)
async def submit_feedback_form(
    token: str,
    actual_cost: float = Form(...),
    actual_timeline: int = Form(...),
    rating: int = Form(...),
    discrepancy_reason_primary: str = Form(...),
    discrepancy_reason_secondary: Optional[str] = Form(default=None),
    comment: Optional[str] = Form(default=None),
) -> HTMLResponse:
    """Process feedback form submission — consumes the token.

    1. Validate token state (must be 'valid')
    2. Consume token (sets used_at — prevents double-submit)
    3. Build EstimateSnapshot from session data
    4. Store FeedbackDocument with snapshot
    5. Render thank-you page
    """
    magic_link_svc = MagicLinkService()
    state, token_doc = await magic_link_svc.resolve_token_state(token)

    if state in ("not_found", "expired"):
        template = _jinja_env.get_template("feedback_expired.html")
        return HTMLResponse(template.render())

    if state == "used":
        ctx = await _load_estimate_context(token_doc)
        template = _jinja_env.get_template("feedback_submitted.html")
        return HTMLResponse(template.render(company_name=ctx["company_name"]))

    # Consume token (atomic — prevents double-submit)
    consumed = await magic_link_svc.consume(token)
    if not consumed:
        # Race condition: another request consumed it between resolve and consume
        ctx = await _load_estimate_context(token_doc)
        template = _jinja_env.get_template("feedback_submitted.html")
        return HTMLResponse(template.render(company_name=ctx["company_name"]))

    # Parse submission
    # Handle empty string from form as None for optional secondary reason
    secondary = discrepancy_reason_secondary if discrepancy_reason_secondary else None
    submission = FeedbackSubmission(
        actual_cost=actual_cost,
        actual_timeline=actual_timeline,
        rating=rating,
        discrepancy_reason_primary=DiscrepancyReason(discrepancy_reason_primary),
        discrepancy_reason_secondary=(
            DiscrepancyReason(secondary) if secondary else None
        ),
        comment=comment if comment else None,
    )

    # Build estimate snapshot
    ctx = await _load_estimate_context(token_doc)
    estimate_data = ctx["estimate_data"]

    snapshot = EstimateSnapshot(
        total_cost_p50=estimate_data.get("total_cost_p50", 0),
        total_cost_p80=estimate_data.get("total_cost_p80", 0),
        timeline_weeks_p50=estimate_data.get("timeline_weeks_p50", 0),
        timeline_weeks_p80=estimate_data.get("timeline_weeks_p80", 0),
        cost_breakdown=estimate_data.get("cost_breakdown", []),
        assumptions=estimate_data.get("assumptions", []),
        confidence_score=estimate_data.get("confidence_score", 0),
    )

    # Store feedback document
    feedback_svc = FeedbackService()
    await feedback_svc.store_feedback_with_snapshot(
        tenant_id=token_doc["tenant_id"],
        estimation_session_id=token_doc["estimation_session_id"],
        submission=submission,
        estimate_snapshot=snapshot,
        reference_class_id=ctx.get("reference_class_id"),
    )

    template = _jinja_env.get_template("feedback_submitted.html")
    return HTMLResponse(template.render(company_name=ctx["company_name"]))
