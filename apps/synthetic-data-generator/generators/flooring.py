"""
Flooring project reference class generator.
"""

from typing import List, Dict
from .common import set_seed, create_reference_class_dict, REGIONS

FLOORING_BASE_COSTS = {
    'laminate': 3000,     # 500 sq ft
    'vinyl': 4000,
    'hardwood': 8000,
    'tile': 6000,
}

FLOORING_COST_BREAKDOWN = {
    'materials': 0.50,
    'labor': 0.40,
    'removal': 0.05,
    'permits': 0.0,
    'overhead': 0.05
}


def generate_flooring_reference_classes() -> List[Dict]:
    """Generate flooring reference classes."""
    set_seed(42)
    reference_classes = []

    for region in REGIONS:
        for material, cost in FLOORING_BASE_COSTS.items():
            timeline = {'laminate': 3, 'vinyl': 3, 'hardwood': 5, 'tile': 7}[material]
            rc = create_reference_class_dict(
                category="construction",
                subcategory="flooring",
                name=f"Flooring - {material.title()} ({region})",
                description=f"Flooring installation with {material}",
                keywords=["flooring", "floor", material],
                region=region,
                attributes={"material": material, "area_sqft": 500},
                base_mean_cost=cost,
                mean_timeline_days=timeline,
                cost_breakdown=FLOORING_COST_BREAKDOWN,
                validation_source="HomeAdvisor_2024_Flooring_Guide"
            )
            reference_classes.append(rc)

    return reference_classes
