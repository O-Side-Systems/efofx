"""
ADU (Accessory Dwelling Unit) reference class generator.

Generates synthetic reference classes for ADU construction projects
based on 2024 HomeAdvisor data.
"""

from typing import List, Dict
from .common import (
    set_seed,
    create_reference_class_dict,
    REGIONS
)


# HomeAdvisor 2024 average costs for ADUs
ADU_BASE_COSTS = {
    'studio': 120000,    # 400-600 sq ft
    'one_bedroom': 180000,  # 600-800 sq ft
    'two_bedroom': 250000,  # 800-1200 sq ft
}

ADU_COST_BREAKDOWN = {
    'materials': 0.35,
    'labor': 0.40,
    'permits': 0.08,
    'design': 0.07,
    'overhead': 0.10
}


def generate_adu_reference_classes() -> List[Dict]:
    """Generate ADU reference classes for all regions and sizes."""
    set_seed(42)
    reference_classes = []

    for region in REGIONS:
        rc = create_reference_class_dict(
            category="construction",
            subcategory="adu",
            name=f"ADU - Studio ({region})",
            description="Studio ADU, 400-600 sq ft, basic finishes",
            keywords=["adu", "accessory", "dwelling", "studio", "granny flat"],
            region=region,
            attributes={"size_range": "400-600 sq ft", "bedrooms": 0, "bathrooms": 1},
            base_mean_cost=ADU_BASE_COSTS['studio'],
            mean_timeline_days=120,
            cost_breakdown=ADU_COST_BREAKDOWN,
            validation_source="HomeAdvisor_2024_ADU_Guide"
        )
        reference_classes.append(rc)

        rc = create_reference_class_dict(
            category="construction",
            subcategory="adu",
            name=f"ADU - 1 Bedroom ({region})",
            description="One bedroom ADU, 600-800 sq ft, standard finishes",
            keywords=["adu", "accessory", "dwelling", "one bedroom"],
            region=region,
            attributes={"size_range": "600-800 sq ft", "bedrooms": 1, "bathrooms": 1},
            base_mean_cost=ADU_BASE_COSTS['one_bedroom'],
            mean_timeline_days=150,
            cost_breakdown=ADU_COST_BREAKDOWN,
            validation_source="HomeAdvisor_2024_ADU_Guide"
        )
        reference_classes.append(rc)

        rc = create_reference_class_dict(
            category="construction",
            subcategory="adu",
            name=f"ADU - 2 Bedroom ({region})",
            description="Two bedroom ADU, 800-1200 sq ft, upgraded finishes",
            keywords=["adu", "accessory", "dwelling", "two bedroom"],
            region=region,
            attributes={"size_range": "800-1200 sq ft", "bedrooms": 2, "bathrooms": 2},
            base_mean_cost=ADU_BASE_COSTS['two_bedroom'],
            mean_timeline_days=180,
            cost_breakdown=ADU_COST_BREAKDOWN,
            validation_source="HomeAdvisor_2024_ADU_Guide"
        )
        reference_classes.append(rc)

    return reference_classes
