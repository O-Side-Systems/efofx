"""
Validation utility functions for efOfX Estimation Service.

This module provides validation utilities for data validation
and business rule enforcement.
"""

from typing import Dict, Any
from app.core.constants import Region, ReferenceClassCategory


def validate_region(region: str) -> bool:
    """Validate if region is supported."""
    try:
        Region(region)
        return True
    except ValueError:
        return False


def validate_reference_class(reference_class: str) -> bool:
    """Validate if reference class is supported."""
    try:
        ReferenceClassCategory(reference_class)
        return True
    except ValueError:
        return False


def validate_project_description(description: str) -> Dict[str, Any]:
    """Validate project description and extract information."""
    if not description or len(description.strip()) < 10:
        return {
            "valid": False,
            "error": "Description must be at least 10 characters long",
        }

    if len(description) > 2000:
        return {
            "valid": False,
            "error": "Description must be less than 2000 characters",
        }

    # Extract basic information
    info = {
        "valid": True,
        "word_count": len(description.split()),
        "has_size_info": any(word.isdigit() for word in description.split()),
        "has_material_info": any(
            word in description.lower()
            for word in ["pool", "deck", "patio", "renovation"]
        ),
        "has_location_info": any(
            word in description.lower()
            for word in ["backyard", "front", "side", "indoor", "outdoor"]
        ),
    }

    return info


def validate_confidence_threshold(threshold: float) -> bool:
    """Validate confidence threshold value."""
    return 0.0 <= threshold <= 1.0


def validate_cost_breakdown(breakdown: Dict[str, float]) -> Dict[str, Any]:
    """Validate cost breakdown structure and values."""
    required_categories = [
        "materials",
        "labor",
        "equipment",
        "permits",
        "design",
        "contingency",
        "profit_margin",
    ]

    # Check if all required categories are present
    missing_categories = [cat for cat in required_categories if cat not in breakdown]
    if missing_categories:
        return {
            "valid": False,
            "error": f"Missing required categories: {missing_categories}",
        }

    # Check if all values are non-negative
    negative_values = [cat for cat, value in breakdown.items() if value < 0]
    if negative_values:
        return {"valid": False, "error": f"Negative values found: {negative_values}"}

    # Calculate total and check if it's reasonable
    total = sum(breakdown.values())
    if total <= 0:
        return {"valid": False, "error": "Total cost must be greater than zero"}

    return {
        "valid": True,
        "total": total,
        "breakdown_percentages": {
            cat: (value / total) * 100 for cat, value in breakdown.items()
        },
    }


def validate_timeline_weeks(weeks: int) -> bool:
    """Validate timeline in weeks."""
    return 1 <= weeks <= 104  # 1 week to 2 years


def validate_team_size(size: int) -> bool:
    """Validate team size."""
    return 1 <= size <= 50  # Reasonable range for construction projects


def validate_rating(rating: int) -> bool:
    """Validate feedback rating."""
    return 1 <= rating <= 5


def validate_accuracy_score(score: float) -> bool:
    """Validate accuracy score."""
    return 0.0 <= score <= 1.0
