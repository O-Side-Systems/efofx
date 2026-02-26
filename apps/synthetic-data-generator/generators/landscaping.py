"""
Landscaping project reference class generator.
"""

from typing import List, Dict
from .common import set_seed, create_reference_class_dict, REGIONS

LANDSCAPING_BASE_COSTS = {
    'basic': 5000,       # Basic plants, mulch, cleanup
    'standard': 12000,   # Sod, plants, irrigation, lighting
    'premium': 25000,    # Hardscaping, mature plants, water features
}

LANDSCAPING_COST_BREAKDOWN = {
    'materials': 0.40,
    'labor': 0.45,
    'equipment': 0.08,
    'permits': 0.02,
    'overhead': 0.05
}


def generate_landscaping_reference_classes() -> List[Dict]:
    """Generate landscaping reference classes."""
    set_seed(42)
    reference_classes = []

    for region in REGIONS:
        for scope, cost in LANDSCAPING_BASE_COSTS.items():
            timeline = {'basic': 7, 'standard': 14, 'premium': 25}[scope]
            rc = create_reference_class_dict(
                category="construction",
                subcategory="landscaping",
                name=f"Landscaping - {scope.title()} ({region})",
                description=f"{scope.title()} landscaping project",
                keywords=["landscaping", "landscape", "yard", scope],
                region=region,
                attributes={"scope": scope, "area": "standard"},
                base_mean_cost=cost,
                mean_timeline_days=timeline,
                cost_breakdown=LANDSCAPING_COST_BREAKDOWN,
                validation_source="HomeAdvisor_2024_Landscaping_Guide"
            )
            reference_classes.append(rc)

    return reference_classes
