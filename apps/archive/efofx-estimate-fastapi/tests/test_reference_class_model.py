"""
Tests for Reference Class Pydantic models.

Tests validation rules, especially cost_breakdown_template percentage validation.
"""

import pytest
from pydantic import ValidationError
from app.models.reference_class import (
    ReferenceClass,
    CostDistribution,
    TimelineDistribution,
    ReferenceClassCreate
)


def test_valid_reference_class():
    """Test creating a valid reference class."""
    data = {
        "tenant_id": None,
        "category": "construction",
        "subcategory": "pool",
        "name": "Test Pool Class",
        "description": "A test reference class",
        "keywords": ["pool", "test"],
        "regions": ["us-ca-south"],
        "attributes": {
            "size_range": "300-500 sq ft",
            "depth": "4-8 feet"
        },
        "cost_distribution": {
            "p50": 50000,
            "p80": 65000,
            "p95": 85000,
            "currency": "USD"
        },
        "timeline_distribution": {
            "p50_days": 45,
            "p80_days": 60,
            "p95_days": 90
        },
        "cost_breakdown_template": {
            "excavation": 0.15,
            "materials": 0.40,
            "labor": 0.30,
            "permits": 0.05,
            "overhead": 0.10
        },
        "is_synthetic": True,
        "validation_source": "test"
    }

    ref_class = ReferenceClass(**data)

    assert ref_class.name == "Test Pool Class"
    assert ref_class.category == "construction"
    assert ref_class.tenant_id is None
    assert len(ref_class.cost_breakdown_template) == 5
    assert sum(ref_class.cost_breakdown_template.values()) == 1.0


def test_cost_breakdown_sum_validation_exact():
    """Test that cost breakdown percentages must sum to 1.0 exactly (within tolerance)."""
    data = {
        "tenant_id": None,
        "category": "construction",
        "subcategory": "pool",
        "name": "Test Pool Class",
        "description": "A test reference class",
        "cost_distribution": {
            "p50": 50000,
            "p80": 65000,
            "p95": 85000
        },
        "timeline_distribution": {
            "p50_days": 45,
            "p80_days": 60,
            "p95_days": 90
        },
        "cost_breakdown_template": {
            "materials": 0.50,
            "labor": 0.30,
            "overhead": 0.20
        },
        "validation_source": "test"
    }

    # Should pass - sums to exactly 1.0
    ref_class = ReferenceClass(**data)
    assert ref_class is not None


def test_cost_breakdown_sum_validation_within_tolerance():
    """Test that small floating point errors are tolerated."""
    data = {
        "tenant_id": None,
        "category": "construction",
        "subcategory": "pool",
        "name": "Test Pool Class",
        "description": "A test reference class",
        "cost_distribution": {
            "p50": 50000,
            "p80": 65000,
            "p95": 85000
        },
        "timeline_distribution": {
            "p50_days": 45,
            "p80_days": 60,
            "p95_days": 90
        },
        "cost_breakdown_template": {
            "materials": 0.333333,
            "labor": 0.333333,
            "overhead": 0.333334  # Sums to 1.000000 within tolerance
        },
        "validation_source": "test"
    }

    # Should pass - within 1% tolerance
    ref_class = ReferenceClass(**data)
    assert ref_class is not None


def test_cost_breakdown_sum_validation_fails_over():
    """Test that percentages summing to > 1.0 fail validation."""
    data = {
        "tenant_id": None,
        "category": "construction",
        "subcategory": "pool",
        "name": "Test Pool Class",
        "description": "A test reference class",
        "cost_distribution": {
            "p50": 50000,
            "p80": 65000,
            "p95": 85000
        },
        "timeline_distribution": {
            "p50_days": 45,
            "p80_days": 60,
            "p95_days": 90
        },
        "cost_breakdown_template": {
            "materials": 0.60,
            "labor": 0.40,
            "overhead": 0.20  # Sums to 1.20
        },
        "validation_source": "test"
    }

    with pytest.raises(ValidationError) as exc_info:
        ReferenceClass(**data)

    assert "must sum to 1.0" in str(exc_info.value)


def test_cost_breakdown_sum_validation_fails_under():
    """Test that percentages summing to < 1.0 fail validation."""
    data = {
        "tenant_id": None,
        "category": "construction",
        "subcategory": "pool",
        "name": "Test Pool Class",
        "description": "A test reference class",
        "cost_distribution": {
            "p50": 50000,
            "p80": 65000,
            "p95": 85000
        },
        "timeline_distribution": {
            "p50_days": 45,
            "p80_days": 60,
            "p95_days": 90
        },
        "cost_breakdown_template": {
            "materials": 0.40,
            "labor": 0.30,
            "overhead": 0.10  # Sums to 0.80
        },
        "validation_source": "test"
    }

    with pytest.raises(ValidationError) as exc_info:
        ReferenceClass(**data)

    assert "must sum to 1.0" in str(exc_info.value)


def test_tenant_specific_reference_class():
    """Test creating a tenant-specific reference class."""
    data = {
        "tenant_id": "507f1f77bcf86cd799439011",
        "category": "construction",
        "subcategory": "custom_pool",
        "name": "Tenant Custom Pool",
        "description": "Tenant-specific reference class",
        "cost_distribution": {
            "p50": 70000,
            "p80": 90000,
            "p95": 120000
        },
        "timeline_distribution": {
            "p50_days": 60,
            "p80_days": 80,
            "p95_days": 120
        },
        "cost_breakdown_template": {
            "materials": 0.50,
            "labor": 0.50
        },
        "validation_source": "tenant_data"
    }

    ref_class = ReferenceClass(**data)

    assert ref_class.tenant_id == "507f1f77bcf86cd799439011"
    assert ref_class.is_synthetic is False  # Default


def test_domain_specific_attributes():
    """Test that attributes dict can hold domain-specific data."""
    # Construction domain
    construction_data = {
        "tenant_id": None,
        "category": "construction",
        "subcategory": "pool",
        "name": "Pool with Attributes",
        "description": "Test",
        "attributes": {
            "size_range": "300-500 sq ft",
            "depth": "4-8 feet",
            "includes_spa": True,
            "finish_type": "tile"
        },
        "cost_distribution": {"p50": 50000, "p80": 65000, "p95": 85000},
        "timeline_distribution": {"p50_days": 45, "p80_days": 60, "p95_days": 90},
        "cost_breakdown_template": {"materials": 0.60, "labor": 0.40},
        "validation_source": "test"
    }

    construction_ref = ReferenceClass(**construction_data)
    assert construction_ref.attributes["includes_spa"] is True
    assert construction_ref.attributes["finish_type"] == "tile"

    # IT domain
    it_data = {
        "tenant_id": None,
        "category": "it_dev",
        "subcategory": "api_development",
        "name": "REST API Development",
        "description": "Test",
        "attributes": {
            "tech_stack": ["python", "fastapi", "postgresql"],
            "team_size": "3-5",
            "complexity": "medium",
            "endpoints_count": "10-20"
        },
        "cost_distribution": {"p50": 25000, "p80": 35000, "p95": 50000},
        "timeline_distribution": {"p50_days": 30, "p80_days": 45, "p95_days": 60},
        "cost_breakdown_template": {"development": 0.70, "testing": 0.20, "deployment": 0.10},
        "validation_source": "test"
    }

    it_ref = ReferenceClass(**it_data)
    assert it_ref.attributes["tech_stack"] == ["python", "fastapi", "postgresql"]
    assert it_ref.attributes["complexity"] == "medium"


def test_reference_class_create_schema():
    """Test the ReferenceClassCreate schema."""
    data = {
        "category": "construction",
        "subcategory": "pool",
        "name": "New Pool Class",
        "description": "Test",
        "cost_distribution": {
            "p50": 50000,
            "p80": 65000,
            "p95": 85000
        },
        "timeline_distribution": {
            "p50_days": 45,
            "p80_days": 60,
            "p95_days": 90
        },
        "cost_breakdown_template": {
            "materials": 0.60,
            "labor": 0.40
        },
        "validation_source": "test"
    }

    ref_class_create = ReferenceClassCreate(**data)

    assert ref_class_create.name == "New Pool Class"
    assert ref_class_create.tenant_id is None  # Default
    assert ref_class_create.is_synthetic is False  # Default
