"""
Chat models for efOfX Estimation Service.

This module defines the data models for chat sessions and messages
used in the conversational estimation flow.
"""

from pydantic import BaseModel, Field
from typing import Optional, List
from datetime import datetime, timedelta
from bson import ObjectId

from app.models._objectid import PyObjectId


class ScopingContext(BaseModel):
    """Tracks which project details have been gathered during scoping."""
    project_type: Optional[str] = Field(None, description="Type of project (e.g., pool, deck, renovation)")
    project_size: Optional[str] = Field(None, description="Approximate size/scope (e.g., '15x30 feet', '500 sqft')")
    location: Optional[str] = Field(None, description="Project location/region")
    timeline: Optional[str] = Field(None, description="Desired timeline (e.g., 'spring 2026', '3 months')")
    special_conditions: Optional[str] = Field(None, description="Special conditions (access, soil, HOA, etc.)")

    def populated_fields(self) -> set:
        """Return set of field names that have non-None values."""
        return {k for k, v in self.model_dump().items() if v is not None}

    def is_ready(self) -> bool:
        """True when enough detail exists for a quality estimate.
        Required: project_type, location. Sufficient: project_type, project_size, location, timeline.
        """
        populated = self.populated_fields()
        return {"project_type", "project_size", "location", "timeline"}.issubset(populated)

    def missing_fields(self) -> list:
        """Return list of field names that are still None, in priority order."""
        priority = ["project_type", "project_size", "location", "timeline", "special_conditions"]
        return [f for f in priority if getattr(self, f) is None]


class ChatMessage(BaseModel):
    """A single message in the conversation."""
    role: str = Field(..., description="Message role: 'user', 'assistant', or 'system'")
    content: str = Field(..., description="Message content")
    timestamp: datetime = Field(default_factory=datetime.utcnow)


class ChatSession(BaseModel):
    """A chat session with full conversation history."""
    id: Optional[PyObjectId] = Field(default_factory=PyObjectId, alias="_id")
    session_id: str = Field(..., description="Unique session identifier")
    tenant_id: str = Field(..., description="Associated tenant ID")
    status: str = Field(default="active", description="Session status: active, ready, completed, expired")
    messages: List[ChatMessage] = Field(default_factory=list, description="Full conversation history (embedded)")
    scoping_context: ScopingContext = Field(default_factory=ScopingContext, description="Gathered project details")
    is_ready: bool = Field(default=False, description="Whether enough info exists for estimate generation")
    prompt_version: Optional[str] = Field(None, description="Scoping prompt version used")
    created_at: datetime = Field(default_factory=datetime.utcnow)
    updated_at: datetime = Field(default_factory=datetime.utcnow)
    expires_at: datetime = Field(default_factory=lambda: datetime.utcnow() + timedelta(hours=24))

    class Config:
        populate_by_name = True
        arbitrary_types_allowed = True
        json_encoders = {ObjectId: str}


class ChatRequest(BaseModel):
    """Model for chat request."""
    message: str = Field(..., description="User message", min_length=1, max_length=2000)
    session_id: Optional[str] = Field(None, description="Existing session ID (None = create new)")


class ChatResponse(BaseModel):
    """Model for chat response."""
    session_id: str = Field(..., description="Session identifier")
    content: str = Field(..., description="Assistant response message")
    timestamp: datetime = Field(default_factory=datetime.utcnow)
    is_ready: bool = Field(default=False, description="Whether system has enough info for estimate")
    scoping_context: Optional[ScopingContext] = Field(None, description="Current scoping state")
    status: str = Field(default="active", description="Session status")
