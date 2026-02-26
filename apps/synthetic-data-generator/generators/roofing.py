"""
Roofing project reference class generator.
"""

from typing import List, Dict
from .common import set_seed, create_reference_class_dict, REGIONS

ROOFING_BASE_COSTS = {
    'asphalt_shingle': 8000,    # Standard 2000 sq ft roof
    'architectural_shingle': 12000,
    'tile': 20000,
    'metal': 25000,
}

ROOFING_COST_BREAKDOWN = {
    'materials': 0.40,
    'labor': 0.45,
    'removal': 0.08,
    'permits': 0.02,
    'overhead': 0.05
}


def generate_roofing_reference_classes() -> List[Dict]:
    """Generate roofing reference classes."""
    set_seed(42)
    reference_classes = []

    for region in REGIONS:
        for material, cost in ROOFING_BASE_COSTS.items():
            timeline = {'asphalt_shingle': 3, 'architectural_shingle': 4, 'tile': 7, 'metal': 5}[material]
            display_name = material.replace('_', ' ').title()
            rc = create_reference_class_dict(
                category="construction",
                subcategory="roofing",
                name=f"Roof Replacement - {display_name} ({region})",
                description=f"Roof replacement with {display_name.lower()}",
                keywords=["roof", "roofing", "replacement", material],
                region=region,
                attributes={"material": material, "area_sqft": 2000},
                base_mean_cost=cost,
                mean_timeline_days=timeline,
                cost_breakdown=ROOFING_COST_BREAKDOWN,
                validation_source="HomeAdvisor_2024_Roofing_Guide"
            )
            reference_classes.append(rc)

    return reference_classes
