"""
Common utilities for synthetic data generation.

Provides shared functions for generating realistic cost and timeline distributions
using scipy statistical distributions.
"""

import numpy as np
from scipy import stats
from typing import Dict, List, Tuple
from datetime import datetime


# Regional adjustment factors (relative to SoCal Coastal baseline)
REGIONAL_ADJUSTMENTS = {
    'us-ca-south-coastal': 1.00,   # Baseline (highest)
    'us-ca-south-inland': 0.85,    # -15%
    'us-ca-north': 0.90,           # -10%
    'us-ca-central-coast': 0.95,   # -5%
}

REGIONS = list(REGIONAL_ADJUSTMENTS.keys())


def set_seed(seed: int = 42):
    """Set reproducible seed for all random generators."""
    np.random.seed(seed)


def generate_lognormal_cost_distribution(
    mean_cost: float,
    coefficient_of_variation: float = 0.25
) -> Tuple[float, float, float]:
    """
    Generate lognormal cost distribution percentiles.

    Lognormal distribution is used because:
    - Costs are always positive
    - Distribution is right-skewed (realistic for construction costs)
    - Matches empirical cost data patterns

    Args:
        mean_cost: Expected mean cost
        coefficient_of_variation: Std dev / mean (default 0.25 = 25% variation)

    Returns:
        Tuple of (p50, p80, p95) cost values
    """
    # Convert mean and CV to lognormal parameters
    variance = (mean_cost * coefficient_of_variation) ** 2
    mu = np.log(mean_cost ** 2 / np.sqrt(variance + mean_cost ** 2))
    sigma = np.sqrt(np.log(1 + variance / mean_cost ** 2))

    # Generate percentiles from lognormal distribution
    dist = stats.lognorm(s=sigma, scale=np.exp(mu))

    p50 = dist.ppf(0.50)
    p80 = dist.ppf(0.80)
    p95 = dist.ppf(0.95)

    return (round(p50, 2), round(p80, 2), round(p95, 2))


def generate_normal_timeline_distribution(
    mean_days: int,
    std_dev_days: int = None
) -> Tuple[int, int, int]:
    """
    Generate normal timeline distribution percentiles.

    Normal distribution is used for timelines because:
    - Timelines tend to be more symmetric than costs
    - Easier to reason about for planning

    Args:
        mean_days: Expected mean timeline in days
        std_dev_days: Standard deviation (default: 20% of mean)

    Returns:
        Tuple of (p50_days, p80_days, p95_days)
    """
    if std_dev_days is None:
        std_dev_days = max(1, int(mean_days * 0.20))

    dist = stats.norm(loc=mean_days, scale=std_dev_days)

    p50 = int(dist.ppf(0.50))
    p80 = int(dist.ppf(0.80))
    p95 = int(dist.ppf(0.95))

    return (max(1, p50), max(1, p80), max(1, p95))


def apply_regional_adjustment(base_cost: float, region: str) -> float:
    """Apply regional cost adjustment factor."""
    factor = REGIONAL_ADJUSTMENTS.get(region, 1.0)
    return base_cost * factor


def create_reference_class_dict(
    category: str,
    subcategory: str,
    name: str,
    description: str,
    keywords: List[str],
    region: str,
    attributes: Dict,
    base_mean_cost: float,
    mean_timeline_days: int,
    cost_breakdown: Dict[str, float],
    validation_source: str,
    cost_cv: float = 0.25
) -> Dict:
    """
    Create a complete reference class dictionary.

    Args:
        category: Domain category (e.g., 'construction')
        subcategory: Subcategory (e.g., 'pool', 'adu')
        name: Reference class name
        description: Detailed description
        keywords: Search keywords
        region: Region code
        attributes: Domain-specific attributes
        base_mean_cost: Mean cost before regional adjustment
        mean_timeline_days: Mean timeline in days
        cost_breakdown: Cost breakdown percentages (must sum to 1.0)
        validation_source: Source of validation data
        cost_cv: Coefficient of variation for cost (default 0.25)

    Returns:
        Reference class dictionary matching ReferenceClass model
    """
    # Apply regional adjustment
    adjusted_mean_cost = apply_regional_adjustment(base_mean_cost, region)

    # Generate distributions
    p50, p80, p95 = generate_lognormal_cost_distribution(adjusted_mean_cost, cost_cv)
    t50, t80, t95 = generate_normal_timeline_distribution(mean_timeline_days)

    # Validate cost breakdown
    total = sum(cost_breakdown.values())
    if abs(total - 1.0) > 0.01:
        raise ValueError(f"Cost breakdown must sum to 1.0, got {total}")

    return {
        "tenant_id": None,  # Platform-provided
        "category": category,
        "subcategory": subcategory,
        "name": name,
        "description": description,
        "keywords": keywords,
        "regions": [region],
        "attributes": attributes,
        "cost_distribution": {
            "p50": p50,
            "p80": p80,
            "p95": p95,
            "currency": "USD"
        },
        "timeline_distribution": {
            "p50_days": t50,
            "p80_days": t80,
            "p95_days": t95
        },
        "cost_breakdown_template": cost_breakdown,
        "is_synthetic": True,
        "validation_source": validation_source,
        "created_at": datetime.utcnow()
    }
