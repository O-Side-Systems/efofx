"""
Widget Pydantic models for efOfX white-label widget.

Provides data models for:
- Branding configuration (public endpoint, no PII)
- Lead capture (widget prospects)
- Consultation requests (DEBT-04)
- Analytics events (no PII)
"""

from pydantic import BaseModel, Field
from typing import Dict, Optional
from datetime import datetime, timezone


class BrandingConfig(BaseModel):
    """Stored in Tenant.settings['branding']. Defaults used when no branding configured."""  # noqa: E501

    primary_color: str = Field(default="#2563eb")
    secondary_color: str = Field(default="#f3f4f6")
    accent_color: str = Field(default="#1d4ed8")
    logo_url: Optional[str] = Field(default=None)
    welcome_message: str = Field(
        default="Hi! Tell me about your project and I'll help estimate the cost."
    )
    button_text: str = Field(default="Get an Estimate")
    company_name: str = Field(default="")
    locale: str = Field(default="en")
    consultation_form_labels: Optional[Dict[str, str]] = Field(default=None)


class BrandingConfigResponse(BaseModel):
    """Public response — safe fields only, never includes keys or PII."""

    primary_color: str
    secondary_color: str
    accent_color: str
    logo_url: Optional[str]
    welcome_message: str
    button_text: str
    company_name: str
    locale: str = "en"
    consultation_form_labels: Optional[Dict[str, str]] = None


class LeadCapture(BaseModel):
    """Lead captured from widget before estimate generation."""

    session_id: str
    tenant_id: str
    name: str = Field(..., min_length=1, max_length=200)
    email: str = Field(..., description="Prospect email")
    phone: str = Field(..., min_length=5, max_length=30)
    captured_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    estimate_session_id: Optional[str] = None


class LeadCaptureRequest(BaseModel):
    """Request body for lead submission from widget."""

    session_id: str
    name: str = Field(..., min_length=1, max_length=200)
    email: str  # EmailStr would be ideal but widget may send simple strings
    phone: str = Field(..., min_length=5, max_length=30)


class LeadCaptureResponse(BaseModel):
    """Response after lead is saved."""

    message: str = "Lead captured successfully"
    session_id: str


class WidgetAnalyticsEvent(BaseModel):
    """Analytics event from widget — no PII."""

    tenant_id: str
    event_type: str = Field(
        ..., description="widget_view, chat_start, estimate_complete"
    )
    date: str = Field(..., description="ISO date YYYY-MM-DD for daily bucketing")


class ConsultationRequest(BaseModel):
    """Contact form submission from widget ConsultationCTA (DEBT-04)."""

    session_id: str = Field(..., description="Chat session ID")
    name: str = Field(..., min_length=1, max_length=100, description="Customer name")
    email: str = Field(..., description="Customer email")
    phone: str = Field(
        ..., min_length=1, max_length=30, description="Customer phone number"
    )
    message: str = Field(
        ...,
        min_length=1,
        max_length=2000,
        description="Free-text message about the project",
    )


class ConsultationResponse(BaseModel):
    """Response after consultation request is saved."""

    lead_id: str
    message: str = "Consultation request received"
