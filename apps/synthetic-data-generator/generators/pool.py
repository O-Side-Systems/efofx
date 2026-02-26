"""
Pool reference class generator.

Generates synthetic reference classes for residential and commercial pool
construction projects based on 2024 HomeAdvisor data.
"""

from typing import List, Dict

# Handle both relative and absolute imports
try:
    from .common import set_seed, create_reference_class_dict, REGIONS
except ImportError:
    from common import set_seed, create_reference_class_dict, REGIONS


# HomeAdvisor 2024 average costs for pools (baseline: SoCal Coastal)
# Source: HomeAdvisor Pool Installation Cost Guide 2024
POOL_BASE_COSTS = {
    'small': 35000,      # 200-300 sq ft, basic concrete
    'medium': 55000,     # 300-500 sq ft, standard finish
    'large': 85000,      # 500-700 sq ft, premium finish
    'luxury': 135000,    # 700+ sq ft, high-end features
}

# Standard cost breakdown for pools
POOL_COST_BREAKDOWN = {
    'materials': 0.40,    # Concrete, plaster, tile
    'labor': 0.30,        # Excavation, construction, finishing
    'equipment': 0.10,    # Pumps, filters, heaters
    'permits': 0.05,      # Permits and inspections
    'finishing': 0.15     # Decking, landscaping, features
}


def generate_pool_reference_classes() -> List[Dict]:
    """
    Generate pool reference classes for all regions and size variations.

    Returns:
        List of reference class dictionaries (4 sizes × 4 regions = 16 classes)
    """
    set_seed(42)  # Reproducible results

    reference_classes = []

    # Small pools (200-300 sq ft)
    for region in REGIONS:
        rc = create_reference_class_dict(
            category="construction",
            subcategory="pool",
            name=f"Residential Pool - Small ({region})",
            description="Basic concrete pool, 200-300 sq ft, standard plaster finish, minimal features",
            keywords=["pool", "swimming", "residential", "small", "concrete", "basic"],
            region=region,
            attributes={
                "size_range": "200-300 sq ft",
                "depth": "4-6 feet",
                "includes_spa": False,
                "finish_type": "plaster",
                "decking_material": "concrete"
            },
            base_mean_cost=POOL_BASE_COSTS['small'],
            mean_timeline_days=35,
            cost_breakdown=POOL_COST_BREAKDOWN,
            validation_source="HomeAdvisor_2024_Pool_Installation_Guide"
        )
        reference_classes.append(rc)

    # Medium pools (300-500 sq ft) - Most common
    for region in REGIONS:
        rc = create_reference_class_dict(
            category="construction",
            subcategory="pool",
            name=f"Residential Pool - Medium ({region})",
            description="Standard concrete pool, 300-500 sq ft, enhanced plaster or pebble finish, basic features",
            keywords=["pool", "swimming", "residential", "medium", "concrete", "standard"],
            region=region,
            attributes={
                "size_range": "300-500 sq ft",
                "depth": "4-8 feet",
                "includes_spa": False,
                "finish_type": "pebble",
                "decking_material": "concrete_pavers"
            },
            base_mean_cost=POOL_BASE_COSTS['medium'],
            mean_timeline_days=50,
            cost_breakdown=POOL_COST_BREAKDOWN,
            validation_source="HomeAdvisor_2024_Pool_Installation_Guide"
        )
        reference_classes.append(rc)

    # Large pools (500-700 sq ft)
    for region in REGIONS:
        rc = create_reference_class_dict(
            category="construction",
            subcategory="pool",
            name=f"Residential Pool - Large ({region})",
            description="Premium concrete pool, 500-700 sq ft, tile or premium finish, advanced features",
            keywords=["pool", "swimming", "residential", "large", "concrete", "premium"],
            region=region,
            attributes={
                "size_range": "500-700 sq ft",
                "depth": "4-9 feet",
                "includes_spa": True,
                "finish_type": "tile",
                "decking_material": "travertine",
                "features": ["waterfall", "LED lighting"]
            },
            base_mean_cost=POOL_BASE_COSTS['large'],
            mean_timeline_days=70,
            cost_breakdown=POOL_COST_BREAKDOWN,
            validation_source="HomeAdvisor_2024_Pool_Installation_Guide"
        )
        reference_classes.append(rc)

    # Luxury pools (700+ sq ft)
    for region in REGIONS:
        rc = create_reference_class_dict(
            category="construction",
            subcategory="pool",
            name=f"Residential Pool - Luxury ({region})",
            description="High-end custom pool, 700+ sq ft, premium finishes, extensive features and automation",
            keywords=["pool", "swimming", "residential", "luxury", "custom", "high-end"],
            region=region,
            attributes={
                "size_range": "700+ sq ft",
                "depth": "4-12 feet",
                "includes_spa": True,
                "finish_type": "glass_tile",
                "decking_material": "natural_stone",
                "features": ["infinity_edge", "waterfall", "swim_up_bar", "LED_lighting", "automation"]
            },
            base_mean_cost=POOL_BASE_COSTS['luxury'],
            mean_timeline_days=90,
            cost_breakdown=POOL_COST_BREAKDOWN,
            validation_source="HomeAdvisor_2024_Pool_Installation_Guide"
        )
        reference_classes.append(rc)

    return reference_classes


if __name__ == "__main__":
    # Test generation
    classes = generate_pool_reference_classes()
    print(f"Generated {len(classes)} pool reference classes")
    print(f"\nExample (first class):")
    import json
    example = classes[0].copy()
    example['created_at'] = str(example['created_at'])
    print(json.dumps(example, indent=2))
