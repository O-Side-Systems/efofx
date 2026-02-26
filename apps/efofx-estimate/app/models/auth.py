"""
Authentication request/response models for efOfX Estimation Service.

This module defines Pydantic models for registration, email verification,
profile management, login, and token refresh.
"""

from pydantic import BaseModel, EmailStr, Field
from typing import Optional, Dict, Any
from datetime import datetime


class RegisterRequest(BaseModel):
    """Request body for contractor registration."""

    company_name: str = Field(..., min_length=2, max_length=100, description="Contractor company name")
    email: EmailStr = Field(..., description="Email address (used for login and verification)")
    password: str = Field(..., min_length=8, description="Password (minimum 8 characters)")


class RegisterResponse(BaseModel):
    """Response for successful registration."""

    tenant_id: str = Field(..., description="Unique tenant identifier")
    api_key: str = Field(
        ..., description="One-time API key — store this securely, it will never be shown again"
    )
    message: str = Field(..., description="Status message")


class ProfileUpdateRequest(BaseModel):
    """Request body for profile updates."""

    company_name: Optional[str] = Field(
        default=None, min_length=2, max_length=100, description="Updated company name"
    )
    settings: Optional[Dict[str, Any]] = Field(
        default=None, description="Updated branding/settings configuration"
    )


class ProfileResponse(BaseModel):
    """Public profile view of a tenant."""

    tenant_id: str
    company_name: str
    email: str
    tier: str
    email_verified: bool
    created_at: datetime
    masked_api_key: str = Field(..., description="Masked API key, e.g. sk-...abc123")
    has_openai_key: bool = Field(..., description="Whether a BYOK OpenAI key is stored")


class VerifyEmailResponse(BaseModel):
    """Response for email verification."""

    message: str
    email_verified: bool


# ---------------------------------------------------------------------------
# Login and token models (added in plan 02-02)
# ---------------------------------------------------------------------------


class LoginRequest(BaseModel):
    """Request body for contractor login."""

    email: EmailStr = Field(..., description="Email address")
    password: str = Field(..., description="Password")


class LoginResponse(BaseModel):
    """Response for successful login — contains both access and refresh tokens."""

    access_token: str = Field(..., description="JWT access token (20-minute expiry)")
    refresh_token: str = Field(..., description="Opaque refresh token (14-day expiry)")
    token_type: str = Field(default="bearer", description="Token type")
    expires_in: int = Field(..., description="Access token lifetime in seconds")


class RefreshRequest(BaseModel):
    """Request body for token refresh."""

    refresh_token: str = Field(..., description="Valid refresh token from a previous login or refresh")


class TokenResponse(BaseModel):
    """Response for a successful token refresh — contains a new access token."""

    access_token: str = Field(..., description="New JWT access token (20-minute expiry)")
    token_type: str = Field(default="bearer", description="Token type")
    expires_in: int = Field(..., description="Access token lifetime in seconds")
