"""
Bathroom renovation reference class generator.
"""

from typing import List, Dict
from .common import set_seed, create_reference_class_dict, REGIONS

BATHROOM_BASE_COSTS = {
    'basic': 8000,       # Fixtures, tile, vanity
    'midrange': 18000,   # Full remodel, standard materials
    'upscale': 35000,    # Premium materials, spa features
}

BATHROOM_COST_BREAKDOWN = {
    'materials': 0.45,
    'labor': 0.35,
    'fixtures': 0.12,
    'permits': 0.03,
    'overhead': 0.05
}


def generate_bathroom_reference_classes() -> List[Dict]:
    """Generate bathroom renovation reference classes."""
    set_seed(42)
    reference_classes = []

    for region in REGIONS:
        for size, cost in BATHROOM_BASE_COSTS.items():
            timeline = {'basic': 15, 'midrange': 25, 'upscale': 35}[size]
            rc = create_reference_class_dict(
                category="construction",
                subcategory="renovation",
                name=f"Bathroom Remodel - {size.title()} ({region})",
                description=f"{size.title()} bathroom renovation",
                keywords=["bathroom", "renovation", "remodel", size],
                region=region,
                attributes={"scope": size, "size": "standard"},
                base_mean_cost=cost,
                mean_timeline_days=timeline,
                cost_breakdown=BATHROOM_COST_BREAKDOWN,
                validation_source="HomeAdvisor_2024_Bathroom_Guide"
            )
            reference_classes.append(rc)

    return reference_classes
