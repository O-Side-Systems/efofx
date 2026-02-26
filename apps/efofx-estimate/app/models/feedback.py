"""
Feedback models for efOfX Estimation Service.

This module defines the data models for user feedback and system tuning
used to improve estimation accuracy over time.
"""

from pydantic import BaseModel, Field
from typing import Optional, List, Dict, Any
from datetime import datetime
from bson import ObjectId

from app.models._objectid import PyObjectId


class Feedback(BaseModel):
    """Model for user feedback on estimates."""
    
    id: Optional[PyObjectId] = Field(default_factory=PyObjectId, alias="_id")
    tenant_id: PyObjectId = Field(..., description="Associated tenant ID")
    estimation_session_id: str = Field(..., description="Associated estimation session ID")
    feedback_type: str = Field(..., description="Type of feedback (accuracy, cost, timeline, etc.)")
    rating: int = Field(..., ge=1, le=5, description="Feedback rating (1-5)")
    comment: Optional[str] = Field(None, description="User comment")
    actual_cost: Optional[float] = Field(None, description="Actual project cost")
    actual_timeline: Optional[int] = Field(None, description="Actual timeline in weeks")
    actual_team_size: Optional[int] = Field(None, description="Actual team size")
    cost_accuracy: Optional[float] = Field(None, ge=0.0, le=1.0, description="Cost accuracy score")
    timeline_accuracy: Optional[float] = Field(None, ge=0.0, le=1.0, description="Timeline accuracy score")
    reference_class_accuracy: Optional[float] = Field(None, ge=0.0, le=1.0, description="Reference class accuracy")
    metadata: Dict[str, Any] = Field(default_factory=dict, description="Additional metadata")
    created_at: datetime = Field(default_factory=datetime.utcnow)
    updated_at: datetime = Field(default_factory=datetime.utcnow)
    
    class Config:
        allow_population_by_field_name = True
        arbitrary_types_allowed = True
        json_encoders = {ObjectId: str}
        schema_extra = {
            "example": {
                "tenant_id": "507f1f77bcf86cd799439011",
                "estimation_session_id": "sess_123456789",
                "feedback_type": "accuracy",
                "rating": 4,
                "comment": "Cost estimate was very close to actual. Timeline was slightly underestimated.",
                "actual_cost": 65000.0,
                "actual_timeline": 10,
                "actual_team_size": 5,
                "cost_accuracy": 0.95,
                "timeline_accuracy": 0.8,
                "reference_class_accuracy": 0.9,
                "metadata": {
                    "project_completed": True,
                    "weather_delays": True,
                    "scope_changes": False
                },
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            }
        }


class FeedbackCreate(BaseModel):
    """Model for creating new feedback."""
    
    estimation_session_id: str = Field(..., description="Associated estimation session ID")
    feedback_type: str = Field(..., description="Type of feedback")
    rating: int = Field(..., ge=1, le=5, description="Feedback rating (1-5)")
    comment: Optional[str] = Field(None, description="User comment")
    actual_cost: Optional[float] = Field(None, description="Actual project cost")
    actual_timeline: Optional[int] = Field(None, description="Actual timeline in weeks")
    actual_team_size: Optional[int] = Field(None, description="Actual team size")
    cost_accuracy: Optional[float] = Field(None, ge=0.0, le=1.0, description="Cost accuracy score")
    timeline_accuracy: Optional[float] = Field(None, ge=0.0, le=1.0, description="Timeline accuracy score")
    reference_class_accuracy: Optional[float] = Field(None, ge=0.0, le=1.0, description="Reference class accuracy")
    metadata: Optional[Dict[str, Any]] = Field(None, description="Additional metadata")
    
    class Config:
        schema_extra = {
            "example": {
                "estimation_session_id": "sess_123456789",
                "feedback_type": "accuracy",
                "rating": 4,
                "comment": "Cost estimate was very close to actual. Timeline was slightly underestimated.",
                "actual_cost": 65000.0,
                "actual_timeline": 10,
                "actual_team_size": 5,
                "cost_accuracy": 0.95,
                "timeline_accuracy": 0.8,
                "reference_class_accuracy": 0.9,
                "metadata": {
                    "project_completed": True,
                    "weather_delays": True,
                    "scope_changes": False
                }
            }
        }


class FeedbackUpdate(BaseModel):
    """Model for updating existing feedback."""
    
    feedback_type: Optional[str] = Field(None, description="Type of feedback")
    rating: Optional[int] = Field(None, ge=1, le=5, description="Feedback rating (1-5)")
    comment: Optional[str] = Field(None, description="User comment")
    actual_cost: Optional[float] = Field(None, description="Actual project cost")
    actual_timeline: Optional[int] = Field(None, description="Actual timeline in weeks")
    actual_team_size: Optional[int] = Field(None, description="Actual team size")
    cost_accuracy: Optional[float] = Field(None, ge=0.0, le=1.0, description="Cost accuracy score")
    timeline_accuracy: Optional[float] = Field(None, ge=0.0, le=1.0, description="Timeline accuracy score")
    reference_class_accuracy: Optional[float] = Field(None, ge=0.0, le=1.0, description="Reference class accuracy")
    metadata: Optional[Dict[str, Any]] = Field(None, description="Additional metadata")
    
    class Config:
        schema_extra = {
            "example": {
                "rating": 5,
                "comment": "Updated feedback - project completed successfully within budget",
                "actual_cost": 62000.0,
                "actual_timeline": 8,
                "cost_accuracy": 0.98,
                "timeline_accuracy": 1.0
            }
        }


class FeedbackSummary(BaseModel):
    """Model for feedback summary statistics."""
    
    total_feedback: int = Field(..., description="Total number of feedback entries")
    average_rating: float = Field(..., description="Average feedback rating")
    cost_accuracy_avg: Optional[float] = Field(None, description="Average cost accuracy")
    timeline_accuracy_avg: Optional[float] = Field(None, description="Average timeline accuracy")
    reference_class_accuracy_avg: Optional[float] = Field(None, description="Average reference class accuracy")
    feedback_by_type: Dict[str, int] = Field(default_factory=dict, description="Feedback count by type")
    recent_feedback: List[Feedback] = Field(default_factory=list, description="Recent feedback entries")
    
    class Config:
        schema_extra = {
            "example": {
                "total_feedback": 150,
                "average_rating": 4.2,
                "cost_accuracy_avg": 0.92,
                "timeline_accuracy_avg": 0.87,
                "reference_class_accuracy_avg": 0.89,
                "feedback_by_type": {
                    "accuracy": 120,
                    "cost": 25,
                    "timeline": 5
                },
                "recent_feedback": []
            }
        }


class TuningData(BaseModel):
    """Model for system tuning data derived from feedback."""
    
    id: Optional[PyObjectId] = Field(default_factory=PyObjectId, alias="_id")
    reference_class: str = Field(..., description="Reference class being tuned")
    region: str = Field(..., description="Region being tuned")
    tuning_factor: float = Field(..., description="Calculated tuning factor")
    confidence: float = Field(..., ge=0.0, le=1.0, description="Confidence in tuning factor")
    sample_size: int = Field(..., description="Number of samples used")
    feedback_ids: List[str] = Field(default_factory=list, description="Associated feedback IDs")
    created_at: datetime = Field(default_factory=datetime.utcnow)
    applied_at: Optional[datetime] = Field(None, description="When tuning was applied")
    
    class Config:
        allow_population_by_field_name = True
        arbitrary_types_allowed = True
        json_encoders = {ObjectId: str}
        schema_extra = {
            "example": {
                "reference_class": "residential_pool",
                "region": "SoCal - Coastal",
                "tuning_factor": 1.05,
                "confidence": 0.85,
                "sample_size": 25,
                "feedback_ids": ["feedback_001", "feedback_002"],
                "created_at": "2024-01-01T00:00:00Z",
                "applied_at": "2024-01-01T01:00:00Z"
            }
        } 