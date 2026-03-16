"""
Calculation utility functions for efOfX Estimation Service.

This module provides calculation utilities for cost breakdown,
tuning factors, and estimation adjustments.
"""

from typing import Dict, List, Any
import math


def calculate_cost_breakdown(
    total_cost: float, breakdown_template: Dict[str, float]
) -> Dict[str, float]:
    """Calculate cost breakdown based on template percentages."""
    breakdown = {}

    for category, percentage in breakdown_template.items():
        breakdown[category] = total_cost * percentage

    return breakdown


def apply_tuning_factors(
    base_cost: float, tuning_factors: Dict[str, float], region: str
) -> float:
    """Apply region-specific tuning factors to base cost."""
    if region in tuning_factors:
        return base_cost * tuning_factors[region]
    return base_cost


def calculate_confidence_score(
    reference_projects: List[Dict[str, Any]], project_description: str, region: str
) -> float:
    """Calculate confidence score based on reference data quality and project similarity."""  # noqa: E501
    if not reference_projects:
        return 0.5  # Default low confidence

    # Calculate average quality score of reference projects
    quality_scores = [p.get("quality_score", 0.5) for p in reference_projects]
    avg_quality = sum(quality_scores) / len(quality_scores)

    # Calculate similarity score (simplified)
    similarity_score = 0.7  # In production, this would be more sophisticated

    # Combine scores
    confidence = (avg_quality * 0.6) + (similarity_score * 0.4)

    return min(confidence, 1.0)


def calculate_timeline_multiplier(
    reference_class: str, project_complexity: str, region: str
) -> float:
    """Calculate timeline multiplier based on project characteristics."""
    base_multiplier = 1.0

    # Adjust for project complexity
    complexity_multipliers = {
        "simple": 0.8,
        "standard": 1.0,
        "complex": 1.3,
        "very_complex": 1.6,
    }

    complexity_multiplier = complexity_multipliers.get(project_complexity, 1.0)

    # Adjust for region (some regions have longer permitting times, etc.)
    region_multipliers = {
        "SoCal - Coastal": 1.1,
        "SoCal - Inland": 1.0,
        "NorCal - Bay Area": 1.2,
        "NorCal - Central": 0.9,
        "Arizona - Phoenix": 0.9,
        "Arizona - Tucson": 0.9,
        "Nevada - Las Vegas": 1.0,
        "Nevada - Reno": 0.9,
    }

    region_multiplier = region_multipliers.get(region, 1.0)

    return base_multiplier * complexity_multiplier * region_multiplier


def calculate_team_size(
    project_size: float, project_complexity: str, timeline_weeks: int
) -> int:
    """Calculate recommended team size based on project characteristics."""
    # Base team size calculation
    base_size = max(2, math.ceil(project_size / 1000))  # 1 person per 1000 sqft minimum

    # Adjust for complexity
    complexity_adjustments = {
        "simple": 0.8,
        "standard": 1.0,
        "complex": 1.3,
        "very_complex": 1.6,
    }

    complexity_factor = complexity_adjustments.get(project_complexity, 1.0)

    # Adjust for timeline (shorter timeline = more people)
    timeline_factor = max(0.8, min(1.5, 12 / timeline_weeks))  # 12 weeks as baseline

    team_size = math.ceil(base_size * complexity_factor * timeline_factor)

    return min(team_size, 20)  # Cap at 20 people


def calculate_materials_cost(
    project_size: float, material_type: str, region: str
) -> float:
    """Calculate materials cost based on project size and material type."""
    # Base cost per square foot by material type
    base_costs = {
        "concrete": 15.0,
        "wood": 12.0,
        "steel": 25.0,
        "composite": 18.0,
        "natural_stone": 35.0,
        "polymer": 20.0,
    }

    base_cost_per_sqft = base_costs.get(material_type, 15.0)

    # Apply region-specific adjustments
    region_multipliers = {
        "SoCal - Coastal": 1.2,
        "SoCal - Inland": 1.1,
        "NorCal - Bay Area": 1.3,
        "NorCal - Central": 1.0,
        "Arizona - Phoenix": 0.9,
        "Arizona - Tucson": 0.9,
        "Nevada - Las Vegas": 1.0,
        "Nevada - Reno": 0.9,
    }

    region_multiplier = region_multipliers.get(region, 1.0)

    return project_size * base_cost_per_sqft * region_multiplier


def calculate_labor_cost(
    project_size: float, labor_intensity: str, region: str, timeline_weeks: int
) -> float:
    """Calculate labor cost based on project characteristics."""
    # Base labor rate per hour by region
    labor_rates = {
        "SoCal - Coastal": 45.0,
        "SoCal - Inland": 40.0,
        "NorCal - Bay Area": 55.0,
        "NorCal - Central": 35.0,
        "Arizona - Phoenix": 35.0,
        "Arizona - Tucson": 32.0,
        "Nevada - Las Vegas": 38.0,
        "Nevada - Reno": 35.0,
    }

    hourly_rate = labor_rates.get(region, 40.0)

    # Labor intensity multipliers
    intensity_multipliers = {"low": 0.8, "standard": 1.0, "high": 1.3, "very_high": 1.6}

    intensity_multiplier = intensity_multipliers.get(labor_intensity, 1.0)

    # Calculate hours based on project size and timeline
    hours_per_sqft = 0.5  # Base hours per square foot
    total_hours = project_size * hours_per_sqft * intensity_multiplier

    # Adjust for timeline (shorter timeline = more overtime)
    timeline_factor = max(1.0, min(1.5, 12 / timeline_weeks))

    return total_hours * hourly_rate * timeline_factor


def calculate_equipment_cost(
    project_size: float, equipment_intensity: str, timeline_weeks: int
) -> float:
    """Calculate equipment cost based on project characteristics."""
    # Base equipment cost per square foot
    base_equipment_cost = 5.0

    # Equipment intensity multipliers
    intensity_multipliers = {
        "minimal": 0.5,
        "standard": 1.0,
        "heavy": 1.5,
        "very_heavy": 2.0,
    }

    intensity_multiplier = intensity_multipliers.get(equipment_intensity, 1.0)

    # Calculate base equipment cost
    base_cost = project_size * base_equipment_cost * intensity_multiplier

    # Adjust for timeline (longer timeline = more equipment rental)
    timeline_factor = max(0.8, min(1.3, timeline_weeks / 8))

    return base_cost * timeline_factor
