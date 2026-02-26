"""
Tenant model for efOfX Estimation Service.

This module defines the tenant data model used for multitenancy support.
The Tenant model uses tenant_id (UUID string) as the primary identifier,
with bcrypt-hashed passwords and API keys, and Fernet-encrypted BYOK fields.
"""

from pydantic import BaseModel, Field
from typing import Optional, Dict, Any
from datetime import datetime, timezone


class Tenant(BaseModel):
    """Tenant model for multitenancy support."""

    model_config = {"populate_by_name": True, "arbitrary_types_allowed": True}

    tenant_id: str = Field(..., description="UUID string, primary identifier")
    company_name: str = Field(..., description="Contractor company name")
    email: str = Field(..., description="Unique email address for login")
    hashed_password: str = Field(..., description="bcrypt hash of contractor password")
    hashed_api_key: str = Field(..., description="bcrypt hash of the one-time API key")
    api_key_last6: str = Field(default="", description="Last 6 chars of raw API key for display masking")
    tier: str = Field(default="trial", description="Subscription tier: 'trial' or 'paid'")
    email_verified: bool = Field(default=False, description="Whether email has been verified")
    encrypted_openai_key: Optional[str] = Field(
        default=None, description="Fernet ciphertext of tenant's BYOK OpenAI key"
    )
    is_active: bool = Field(default=True, description="Whether tenant account is active")
    created_at: datetime = Field(
        default_factory=lambda: datetime.now(timezone.utc),
        description="Account creation timestamp",
    )
    updated_at: datetime = Field(
        default_factory=lambda: datetime.now(timezone.utc),
        description="Last update timestamp",
    )
    settings: Dict[str, Any] = Field(
        default_factory=dict, description="Tenant-specific settings (branding, etc.)"
    )


class TenantCreate(BaseModel):
    """Internal model for creating a new tenant (post-registration)."""

    model_config = {"populate_by_name": True}

    tenant_id: str
    company_name: str
    email: str
    hashed_password: str
    hashed_api_key: str
    api_key_last6: str = ""
    tier: str = "trial"
    email_verified: bool = False
    encrypted_openai_key: Optional[str] = None
    is_active: bool = True
    settings: Dict[str, Any] = Field(default_factory=dict)


class TenantUpdate(BaseModel):
    """Model for updating an existing tenant."""

    model_config = {"populate_by_name": True}

    company_name: Optional[str] = None
    encrypted_openai_key: Optional[str] = None
    email_verified: Optional[bool] = None
    is_active: Optional[bool] = None
    tier: Optional[str] = None
    settings: Optional[Dict[str, Any]] = None
    updated_at: Optional[datetime] = None
