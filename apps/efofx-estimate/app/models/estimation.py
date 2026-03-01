"""
Estimation models for efOfX Estimation Service.

This module defines the data models for estimation sessions, requests,
and responses used throughout the estimation service.
"""

from pydantic import BaseModel, Field
from typing import Optional, List, Any
from datetime import datetime
from bson import ObjectId

from app.core.constants import EstimationStatus, Region
from app.models._objectid import PyObjectId


# ---------------------------------------------------------------------------
# OpenAI Structured Output models
# Used as response_format for client.beta.chat.completions.parse()
# ---------------------------------------------------------------------------

class CostCategoryEstimate(BaseModel):
    """A single cost category with P50/P80 estimates."""

    category: str = Field(description="Cost category name (e.g., 'Materials', 'Labor')")
    p50_cost: float = Field(description="50th percentile cost estimate")
    p80_cost: float = Field(description="80th percentile cost estimate")
    percentage_of_total: float = Field(description="This category's share of total cost (0.0-1.0)")


class AdjustmentFactor(BaseModel):
    """A named adjustment multiplier applied to the estimate."""

    name: str = Field(description="Adjustment name (e.g., 'Urban premium', 'Complexity multiplier')")
    multiplier: float = Field(description="Multiplier value (e.g., 1.15 for 15% increase)")
    reason: str = Field(description="Brief explanation of why this adjustment applies")


class EstimationOutput(BaseModel):
    """Structured LLM output for project estimation.

    Used as response_format for OpenAI structured outputs via
    client.beta.chat.completions.parse(response_format=EstimationOutput).
    """

    total_cost_p50: float = Field(description="50th percentile total cost estimate")
    total_cost_p80: float = Field(description="80th percentile total cost estimate")
    timeline_weeks_p50: int = Field(description="50th percentile timeline in weeks")
    timeline_weeks_p80: int = Field(description="80th percentile timeline in weeks")
    cost_breakdown: List[CostCategoryEstimate] = Field(
        description="Cost breakdown by category with P50/P80 ranges"
    )
    adjustment_factors: List[AdjustmentFactor] = Field(
        description="Named adjustment multipliers applied to this estimate"
    )
    confidence_score: float = Field(
        ge=0, le=100, description="Confidence score 0-100 reflecting information completeness"
    )
    assumptions: List[str] = Field(description="Explicit assumptions the estimate is based on")
    summary: str = Field(description="One-paragraph plain-language summary of the estimate")


class EstimationSession(BaseModel):
    """Model for estimation session."""

    id: Optional[PyObjectId] = Field(default_factory=PyObjectId, alias="_id")
    tenant_id: str = Field(..., description="Associated tenant ID (UUID string)")
    session_id: str = Field(..., description="Unique session identifier")
    status: EstimationStatus = Field(default=EstimationStatus.INITIATED, description="Session status")
    description: str = Field(..., description="Project description")
    region: Region = Field(..., description="Geographic region")
    reference_class: Optional[str] = Field(None, description="Applied reference class")
    confidence_threshold: float = Field(0.7, description="Confidence threshold")
    prompt_version: Optional[str] = Field(None, description="Prompt version used for this estimate (e.g., '1.0.0')")
    result: Optional[Any] = Field(None, description="Estimation result (legacy field, always None in new flow)")
    chat_messages: List[str] = Field(default_factory=list, description="Chat message IDs")
    images: List[str] = Field(default_factory=list, description="Uploaded image URLs")
    created_at: datetime = Field(default_factory=datetime.utcnow)
    updated_at: datetime = Field(default_factory=datetime.utcnow)
    completed_at: Optional[datetime] = Field(None, description="Completion timestamp")
    expires_at: Optional[datetime] = Field(None, description="Session expiration")
    
    class Config:
        allow_population_by_field_name = True
        arbitrary_types_allowed = True
        json_encoders = {ObjectId: str}
        schema_extra = {
            "example": {
                "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
                "session_id": "sess_123456789",
                "status": "completed",
                "description": "I want to install a 15x30 foot pool with spa in my backyard.",
                "region": "SoCal - Coastal",
                "reference_class": "residential_pool",
                "confidence_threshold": 0.7,
                "prompt_version": "1.0.0",
                "result": None,
                "chat_messages": ["msg_001", "msg_002"],
                "images": ["https://example.com/image1.jpg"],
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:30:00Z",
                "completed_at": "2024-01-01T00:30:00Z"
            }
        }


class EstimationResponse(BaseModel):
    """Model for estimation API response."""
    
    session_id: str = Field(..., description="Session identifier")
    status: EstimationStatus = Field(..., description="Current status")
    message: str = Field(..., description="Response message")
    result: Optional[Any] = Field(None, description="Estimation result if completed (legacy field)")
    next_action: Optional[str] = Field(None, description="Next action required")
    estimated_completion: Optional[datetime] = Field(None, description="Estimated completion time")
    
    class Config:
        schema_extra = {
            "example": {
                "session_id": "sess_123456789",
                "status": "completed",
                "message": "Estimation completed successfully",
                "result": None,
                "next_action": None,
                "estimated_completion": "2024-01-01T00:30:00Z"
            }
        } 