"""
Chat models for efOfX Estimation Service.

This module defines the data models for chat sessions and messages
used in the conversational estimation flow.
"""

from pydantic import BaseModel, Field
from typing import Optional, List, Dict, Any
from datetime import datetime
from bson import ObjectId

from app.models._objectid import PyObjectId


class ChatMessage(BaseModel):
    """Model for chat message."""
    
    id: Optional[PyObjectId] = Field(default_factory=PyObjectId, alias="_id")
    session_id: str = Field(..., description="Associated session ID")
    message_id: str = Field(..., description="Unique message identifier")
    role: str = Field(..., description="Message role (user, assistant, system)")
    content: str = Field(..., description="Message content")
    timestamp: datetime = Field(default_factory=datetime.utcnow, description="Message timestamp")
    metadata: Dict[str, Any] = Field(default_factory=dict, description="Additional metadata")
    
    class Config:
        allow_population_by_field_name = True
        arbitrary_types_allowed = True
        json_encoders = {ObjectId: str}
        schema_extra = {
            "example": {
                "session_id": "sess_123456789",
                "message_id": "msg_001",
                "role": "user",
                "content": "I want to install a 15x30 foot pool with spa in my backyard.",
                "timestamp": "2024-01-01T00:00:00Z",
                "metadata": {
                    "intent": "project_description",
                    "confidence": 0.9
                }
            }
        }


class ChatSession(BaseModel):
    """Model for chat session."""
    
    id: Optional[PyObjectId] = Field(default_factory=PyObjectId, alias="_id")
    session_id: str = Field(..., description="Unique session identifier")
    tenant_id: PyObjectId = Field(..., description="Associated tenant ID")
    estimation_session_id: str = Field(..., description="Associated estimation session ID")
    status: str = Field(default="active", description="Session status")
    messages: List[str] = Field(default_factory=list, description="Message IDs in order")
    context: Dict[str, Any] = Field(default_factory=dict, description="Session context")
    created_at: datetime = Field(default_factory=datetime.utcnow)
    updated_at: datetime = Field(default_factory=datetime.utcnow)
    expires_at: Optional[datetime] = Field(None, description="Session expiration")
    
    class Config:
        allow_population_by_field_name = True
        arbitrary_types_allowed = True
        json_encoders = {ObjectId: str}
        schema_extra = {
            "example": {
                "session_id": "chat_sess_123456789",
                "tenant_id": "507f1f77bcf86cd799439011",
                "estimation_session_id": "sess_123456789",
                "status": "active",
                "messages": ["msg_001", "msg_002", "msg_003"],
                "context": {
                    "project_type": "residential_pool",
                    "region": "SoCal - Coastal",
                    "current_step": "gathering_details"
                },
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:15:00Z"
            }
        }


class ChatRequest(BaseModel):
    """Model for chat request."""
    
    message: str = Field(..., description="User message", min_length=1, max_length=2000)
    session_id: Optional[str] = Field(None, description="Existing session ID")
    context: Optional[Dict[str, Any]] = Field(None, description="Additional context")
    
    class Config:
        schema_extra = {
            "example": {
                "message": "I want to install a 15x30 foot pool with spa in my backyard.",
                "session_id": "chat_sess_123456789",
                "context": {
                    "region": "SoCal - Coastal",
                    "budget_range": "50000-80000"
                }
            }
        }


class ChatResponse(BaseModel):
    """Model for chat response."""
    
    session_id: str = Field(..., description="Session identifier")
    message_id: str = Field(..., description="Response message ID")
    content: str = Field(..., description="Assistant response")
    timestamp: datetime = Field(..., description="Response timestamp")
    next_action: Optional[str] = Field(None, description="Suggested next action")
    context_updates: Optional[Dict[str, Any]] = Field(None, description="Context updates")
    is_complete: bool = Field(default=False, description="Whether conversation is complete")
    
    class Config:
        schema_extra = {
            "example": {
                "session_id": "chat_sess_123456789",
                "message_id": "msg_002",
                "content": "I understand you want to install a pool with spa. Let me gather some more details to provide an accurate estimate. What type of pool material are you considering (gunite, fiberglass, or vinyl)?",
                "timestamp": "2024-01-01T00:01:00Z",
                "next_action": "wait_for_user_input",
                "context_updates": {
                    "project_type": "residential_pool",
                    "pool_size": "15x30",
                    "spa_included": True
                },
                "is_complete": False
            }
        }


class ChatMessageCreate(BaseModel):
    """Model for creating a new chat message."""
    
    session_id: str = Field(..., description="Associated session ID")
    role: str = Field(..., description="Message role")
    content: str = Field(..., description="Message content")
    metadata: Optional[Dict[str, Any]] = Field(None, description="Additional metadata")
    
    class Config:
        schema_extra = {
            "example": {
                "session_id": "chat_sess_123456789",
                "role": "user",
                "content": "I want to install a 15x30 foot pool with spa in my backyard.",
                "metadata": {
                    "intent": "project_description",
                    "confidence": 0.9
                }
            }
        }


class ChatSessionCreate(BaseModel):
    """Model for creating a new chat session."""
    
    session_id: str = Field(..., description="Unique session identifier")
    tenant_id: str = Field(..., description="Associated tenant ID")
    estimation_session_id: str = Field(..., description="Associated estimation session ID")
    context: Optional[Dict[str, Any]] = Field(None, description="Initial session context")
    
    class Config:
        schema_extra = {
            "example": {
                "session_id": "chat_sess_123456789",
                "tenant_id": "507f1f77bcf86cd799439011",
                "estimation_session_id": "sess_123456789",
                "context": {
                    "project_type": "residential_pool",
                    "region": "SoCal - Coastal"
                }
            }
        } 