"""
Reference models for efOfX Estimation Service.

This module defines the data models for reference classes and reference projects
used in the Reference Class Forecasting (RCF) estimation process.
"""

from pydantic import BaseModel, Field
from typing import Optional, List, Dict, Any
from datetime import datetime
from bson import ObjectId

from app.core.constants import ReferenceClassCategory, Region
from app.models._objectid import PyObjectId


class ReferenceClass(BaseModel):
    """Model for reference class classification."""

    id: Optional[PyObjectId] = Field(default_factory=PyObjectId, alias="_id")
    name: str = Field(..., description="Reference class name")
    category: ReferenceClassCategory = Field(..., description="Class category")
    description: str = Field(..., description="Class description")
    keywords: List[str] = Field(
        default_factory=list, description="Identifying keywords"
    )
    regions: List[Region] = Field(
        default_factory=list, description="Applicable regions"
    )
    base_cost_per_sqft: Dict[str, float] = Field(
        default_factory=dict, description="Base cost per square foot by region"
    )
    cost_breakdown_template: Dict[str, float] = Field(
        default_factory=dict, description="Cost breakdown percentages"
    )
    timeline_multiplier: float = Field(
        1.0, description="Timeline adjustment multiplier"
    )
    team_size_template: Dict[str, int] = Field(
        default_factory=dict, description="Team size by project size"
    )
    tuning_factors: Dict[str, float] = Field(
        default_factory=dict, description="Region-specific tuning factors"
    )
    is_active: bool = Field(default=True, description="Whether class is active")
    created_at: datetime = Field(default_factory=datetime.utcnow)
    updated_at: datetime = Field(default_factory=datetime.utcnow)

    class Config:
        allow_population_by_field_name = True
        arbitrary_types_allowed = True
        json_encoders = {ObjectId: str}
        schema_extra = {
            "example": {
                "name": "residential_pool",
                "category": "residential",
                "description": "Residential swimming pool installation",
                "keywords": ["pool", "swimming", "backyard", "residential"],
                "regions": ["SoCal - Coastal", "SoCal - Inland"],
                "base_cost_per_sqft": {
                    "SoCal - Coastal": 150.0,
                    "SoCal - Inland": 140.0,
                },
                "cost_breakdown_template": {
                    "materials": 0.40,
                    "labor": 0.25,
                    "equipment": 0.08,
                    "permits": 0.03,
                    "design": 0.05,
                    "contingency": 0.08,
                    "profit_margin": 0.11,
                },
                "timeline_multiplier": 1.0,
                "team_size_template": {"small": 3, "medium": 4, "large": 6},
                "tuning_factors": {"SoCal - Coastal": 1.1, "SoCal - Inland": 1.0},
                "is_active": True,
            }
        }


class ReferenceProject(BaseModel):
    """Model for reference project data."""

    id: Optional[PyObjectId] = Field(default_factory=PyObjectId, alias="_id")
    project_id: str = Field(..., description="Unique project identifier")
    reference_class: str = Field(..., description="Associated reference class")
    region: Region = Field(..., description="Project region")
    description: str = Field(..., description="Project description")
    size_sqft: Optional[float] = Field(None, description="Project size in square feet")
    total_cost: float = Field(..., description="Actual project cost")
    timeline_weeks: int = Field(..., description="Actual project timeline")
    team_size: int = Field(..., description="Actual team size")
    cost_breakdown: Dict[str, float] = Field(..., description="Actual cost breakdown")
    completion_date: datetime = Field(..., description="Project completion date")
    quality_score: float = Field(..., ge=0.0, le=1.0, description="Data quality score")
    source: str = Field(..., description="Data source")
    metadata: Dict[str, Any] = Field(
        default_factory=dict, description="Additional metadata"
    )
    is_active: bool = Field(default=True, description="Whether project data is active")
    created_at: datetime = Field(default_factory=datetime.utcnow)
    updated_at: datetime = Field(default_factory=datetime.utcnow)

    class Config:
        allow_population_by_field_name = True
        arbitrary_types_allowed = True
        json_encoders = {ObjectId: str}
        schema_extra = {
            "example": {
                "project_id": "pool_001",
                "reference_class": "residential_pool",
                "region": "SoCal - Coastal",
                "description": "15x30 foot pool with spa installation",
                "size_sqft": 450.0,
                "total_cost": 63000.0,
                "timeline_weeks": 8,
                "team_size": 4,
                "cost_breakdown": {
                    "materials": 25000.0,
                    "labor": 15000.0,
                    "equipment": 5000.0,
                    "permits": 2000.0,
                    "design": 3000.0,
                    "contingency": 5000.0,
                    "profit_margin": 8000.0,
                },
                "completion_date": "2023-06-15T00:00:00Z",
                "quality_score": 0.9,
                "source": "internal_database",
                "metadata": {
                    "pool_type": "gunite",
                    "spa_included": True,
                    "site_preparation": "minimal",
                },
                "is_active": True,
            }
        }


class ReferenceClassCreate(BaseModel):
    """Model for creating a new reference class."""

    name: str = Field(..., description="Reference class name")
    category: ReferenceClassCategory = Field(..., description="Class category")
    description: str = Field(..., description="Class description")
    keywords: List[str] = Field(
        default_factory=list, description="Identifying keywords"
    )
    regions: List[Region] = Field(
        default_factory=list, description="Applicable regions"
    )
    base_cost_per_sqft: Dict[str, float] = Field(
        default_factory=dict, description="Base cost per square foot by region"
    )
    cost_breakdown_template: Dict[str, float] = Field(
        default_factory=dict, description="Cost breakdown percentages"
    )
    timeline_multiplier: float = Field(
        1.0, description="Timeline adjustment multiplier"
    )
    team_size_template: Dict[str, int] = Field(
        default_factory=dict, description="Team size by project size"
    )
    tuning_factors: Dict[str, float] = Field(
        default_factory=dict, description="Region-specific tuning factors"
    )

    class Config:
        schema_extra = {
            "example": {
                "name": "residential_pool",
                "category": "residential",
                "description": "Residential swimming pool installation",
                "keywords": ["pool", "swimming", "backyard", "residential"],
                "regions": ["SoCal - Coastal", "SoCal - Inland"],
                "base_cost_per_sqft": {
                    "SoCal - Coastal": 150.0,
                    "SoCal - Inland": 140.0,
                },
                "cost_breakdown_template": {
                    "materials": 0.40,
                    "labor": 0.25,
                    "equipment": 0.08,
                    "permits": 0.03,
                    "design": 0.05,
                    "contingency": 0.08,
                    "profit_margin": 0.11,
                },
                "timeline_multiplier": 1.0,
                "team_size_template": {"small": 3, "medium": 4, "large": 6},
                "tuning_factors": {"SoCal - Coastal": 1.1, "SoCal - Inland": 1.0},
            }
        }


class ReferenceProjectCreate(BaseModel):
    """Model for creating a new reference project."""

    project_id: str = Field(..., description="Unique project identifier")
    reference_class: str = Field(..., description="Associated reference class")
    region: Region = Field(..., description="Project region")
    description: str = Field(..., description="Project description")
    size_sqft: Optional[float] = Field(None, description="Project size in square feet")
    total_cost: float = Field(..., description="Actual project cost")
    timeline_weeks: int = Field(..., description="Actual project timeline")
    team_size: int = Field(..., description="Actual team size")
    cost_breakdown: Dict[str, float] = Field(..., description="Actual cost breakdown")
    completion_date: datetime = Field(..., description="Project completion date")
    quality_score: float = Field(..., ge=0.0, le=1.0, description="Data quality score")
    source: str = Field(..., description="Data source")
    metadata: Dict[str, Any] = Field(
        default_factory=dict, description="Additional metadata"
    )

    class Config:
        schema_extra = {
            "example": {
                "project_id": "pool_001",
                "reference_class": "residential_pool",
                "region": "SoCal - Coastal",
                "description": "15x30 foot pool with spa installation",
                "size_sqft": 450.0,
                "total_cost": 63000.0,
                "timeline_weeks": 8,
                "team_size": 4,
                "cost_breakdown": {
                    "materials": 25000.0,
                    "labor": 15000.0,
                    "equipment": 5000.0,
                    "permits": 2000.0,
                    "design": 3000.0,
                    "contingency": 5000.0,
                    "profit_margin": 8000.0,
                },
                "completion_date": "2023-06-15T00:00:00Z",
                "quality_score": 0.9,
                "source": "internal_database",
                "metadata": {
                    "pool_type": "gunite",
                    "spa_included": True,
                    "site_preparation": "minimal",
                },
            }
        }
