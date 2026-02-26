"""
Kitchen renovation reference class generator.

Generates synthetic reference classes for kitchen remodeling projects
based on 2024 HomeAdvisor data.
"""

from typing import List, Dict
from .common import (
    set_seed,
    create_reference_class_dict,
    REGIONS
)


# HomeAdvisor 2024 average costs for kitchen renovations
KITCHEN_BASE_COSTS = {
    'budget': 15000,      # Basic updates, keep layout
    'midrange': 30000,    # New cabinets, countertops, appliances
    'upscale': 65000,     # Premium materials, layout changes
    'luxury': 125000,     # Complete custom renovation
}

KITCHEN_COST_BREAKDOWN = {
    'materials': 0.50,   # Cabinets, countertops, flooring
    'labor': 0.25,
    'appliances': 0.15,
    'permits': 0.03,
    'overhead': 0.07
}


def generate_kitchen_reference_classes() -> List[Dict]:
    """Generate kitchen renovation reference classes."""
    set_seed(42)
    reference_classes = []

    for region in REGIONS:
        rc = create_reference_class_dict(
            category="construction",
            subcategory="renovation",
            name=f"Kitchen Remodel - Budget ({region})",
            description="Budget kitchen update, refinish cabinets, new countertops and backsplash",
            keywords=["kitchen", "renovation", "remodel", "budget", "update"],
            region=region,
            attributes={"scope": "cosmetic", "layout_change": False},
            base_mean_cost=KITCHEN_BASE_COSTS['budget'],
            mean_timeline_days=20,
            cost_breakdown=KITCHEN_COST_BREAKDOWN,
            validation_source="HomeAdvisor_2024_Kitchen_Guide"
        )
        reference_classes.append(rc)

        rc = create_reference_class_dict(
            category="construction",
            subcategory="renovation",
            name=f"Kitchen Remodel - Midrange ({region})",
            description="Midrange kitchen remodel, new cabinets, quartz countertops, mid-range appliances",
            keywords=["kitchen", "renovation", "remodel", "midrange"],
            region=region,
            attributes={"scope": "full", "layout_change": False},
            base_mean_cost=KITCHEN_BASE_COSTS['midrange'],
            mean_timeline_days=35,
            cost_breakdown=KITCHEN_COST_BREAKDOWN,
            validation_source="HomeAdvisor_2024_Kitchen_Guide"
        )
        reference_classes.append(rc)

        rc = create_reference_class_dict(
            category="construction",
            subcategory="renovation",
            name=f"Kitchen Remodel - Upscale ({region})",
            description="Upscale kitchen renovation, custom cabinets, granite/marble, premium appliances",
            keywords=["kitchen", "renovation", "remodel", "upscale", "premium"],
            region=region,
            attributes={"scope": "full", "layout_change": True},
            base_mean_cost=KITCHEN_BASE_COSTS['upscale'],
            mean_timeline_days=50,
            cost_breakdown=KITCHEN_COST_BREAKDOWN,
            validation_source="HomeAdvisor_2024_Kitchen_Guide"
        )
        reference_classes.append(rc)

    return reference_classes
