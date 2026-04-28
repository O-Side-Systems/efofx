"""
Reference Class data models for efOfX Estimation Service.

Domain-agnostic schema that supports construction, IT/dev, and future domains.
"""

from pydantic import BaseModel, Field, field_validator
from typing import Dict, List, Any, Optional
from datetime import datetime


class CostDistribution(BaseModel):
    """Cost distribution statistics."""

    p50: float = Field(..., description="50th percentile cost")
    p80: float = Field(..., description="80th percentile cost")
    p95: float = Field(..., description="95th percentile cost")
    currency: str = Field(default="USD", description="Currency code")


class TimelineDistribution(BaseModel):
    """Timeline distribution statistics in days."""

    p50_days: int = Field(..., description="50th percentile timeline in days")
    p80_days: int = Field(..., description="80th percentile timeline in days")
    p95_days: int = Field(..., description="95th percentile timeline in days")


class ReferenceClass(BaseModel):
    """
    Domain-agnostic reference class for estimation.

    Supports multiple domains (construction, IT/dev, etc.) through flexible attributes.
    """

    # Identity
    id: Optional[str] = Field(default=None, alias="_id", description="MongoDB ObjectId")
    tenant_id: Optional[str] = Field(
        default=None, description="Tenant ID (null = platform-provided)"
    )

    # Classification
    category: str = Field(
        ..., description="Domain category (e.g., 'construction', 'it_dev')"
    )
    subcategory: str = Field(
        ..., description="Subcategory (e.g., 'pool', 'api_development')"
    )
    name: str = Field(..., description="Reference class name")
    description: str = Field(..., description="Detailed description")
    keywords: List[str] = Field(default_factory=list, description="Search keywords")
    regions: List[str] = Field(default_factory=list, description="Applicable regions")

    # Domain-specific attributes (flexible)
    attributes: Dict[str, Any] = Field(
        default_factory=dict,
        description="Domain-specific attributes (e.g., size_range, tech_stack)",
    )

    # Cost and timeline distributions
    cost_distribution: CostDistribution = Field(..., description="Cost statistics")
    timeline_distribution: TimelineDistribution = Field(
        ..., description="Timeline statistics"
    )

    # Cost breakdown (percentages must sum to 1.0)
    cost_breakdown_template: Dict[str, float] = Field(
        ..., description="Cost breakdown by category (percentages summing to 1.0)"
    )

    # Metadata
    is_synthetic: bool = Field(
        default=False, description="Whether data is synthetically generated"
    )
    validation_source: str = Field(..., description="Source of validation data")
    created_at: datetime = Field(
        default_factory=datetime.utcnow, description="Creation timestamp"
    )

    class Config:
        populate_by_name = True
        json_schema_extra = {
            "example": {
                "_id": "507f1f77bcf86cd799439011",
                "tenant_id": None,
                "category": "construction",
                "subcategory": "pool",
                "name": "Residential Pool - Midrange (SoCal)",
                "description": "Standard residential swimming pool construction in Southern California",  # noqa: E501
                "keywords": ["pool", "swimming", "residential", "concrete"],
                "regions": ["us-ca-south", "us-west"],
                "attributes": {
                    "size_range": "300-500 sq ft",
                    "depth": "4-8 feet",
                    "includes_spa": False,
                    "finish_type": "plaster",
                },
                "cost_distribution": {
                    "p50": 55000,
                    "p80": 72000,
                    "p95": 95000,
                    "currency": "USD",
                },
                "timeline_distribution": {
                    "p50_days": 45,
                    "p80_days": 60,
                    "p95_days": 90,
                },
                "cost_breakdown_template": {
                    "excavation": 0.15,
                    "materials": 0.40,
                    "labor": 0.30,
                    "permits": 0.05,
                    "overhead": 0.10,
                },
                "is_synthetic": True,
                "validation_source": "synthetic_generator_v1",
                "created_at": "2025-11-16T00:00:00Z",
            }
        }

    @field_validator("cost_breakdown_template")
    @classmethod
    def validate_cost_breakdown_sum(cls, v: Dict[str, float]) -> Dict[str, float]:
        """Validate that cost breakdown percentages sum to 1.0 (within tolerance)."""
        total = sum(v.values())
        tolerance = 0.01  # Allow 1% tolerance for floating point errors

        if abs(total - 1.0) > tolerance:
            raise ValueError(
                f"Cost breakdown percentages must sum to 1.0, got {total:.4f}. "
                f"Breakdown: {v}"
            )

        return v


class ReferenceClassCreate(BaseModel):
    """Schema for creating a new reference class."""

    tenant_id: Optional[str] = None
    category: str
    subcategory: str
    name: str
    description: str
    keywords: List[str] = Field(default_factory=list)
    regions: List[str] = Field(default_factory=list)
    attributes: Dict[str, Any] = Field(default_factory=dict)
    cost_distribution: CostDistribution
    timeline_distribution: TimelineDistribution
    cost_breakdown_template: Dict[str, float]
    is_synthetic: bool = False
    validation_source: str


class ReferenceClassUpdate(BaseModel):
    """Schema for updating an existing reference class."""

    name: Optional[str] = None
    description: Optional[str] = None
    keywords: Optional[List[str]] = None
    regions: Optional[List[str]] = None
    attributes: Optional[Dict[str, Any]] = None
    cost_distribution: Optional[CostDistribution] = None
    timeline_distribution: Optional[TimelineDistribution] = None
    cost_breakdown_template: Optional[Dict[str, float]] = None
    validation_source: Optional[str] = None
