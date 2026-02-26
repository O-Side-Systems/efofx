"""
Validate synthetic reference classes against HomeAdvisor 2024 benchmarks.

This script validates that generated synthetic costs fall within ±25% of
HomeAdvisor 2024 average costs for each construction type.
"""

import asyncio
import sys
from pathlib import Path
from typing import Dict, List

# Add parent directories to path
sys.path.insert(0, str(Path(__file__).parent.parent / 'efofx-estimate'))

from app.db.mongodb import connect_to_mongo, close_mongo_connection, get_reference_classes_collection


# HomeAdvisor 2024 benchmark costs (SoCal Coastal baseline)
BENCHMARK_COSTS = {
    'pool': {
        'small': 35000,
        'medium': 55000,
        'large': 85000,
        'luxury': 135000,
    },
    'adu': {
        'studio': 120000,
        'one_bedroom': 180000,
        'two_bedroom': 250000,
    },
    'kitchen': {
        'budget': 15000,
        'midrange': 30000,
        'upscale': 65000,
    },
    'bathroom': {
        'basic': 8000,
        'midrange': 18000,
        'upscale': 35000,
    },
    'landscaping': {
        'basic': 5000,
        'standard': 12000,
        'premium': 25000,
    },
    'roofing': {
        'asphalt_shingle': 8000,
        'architectural_shingle': 12000,
        'tile': 20000,
        'metal': 25000,
    },
    'flooring': {
        'laminate': 3000,
        'vinyl': 4000,
        'hardwood': 8000,
        'tile': 6000,
    },
}


def check_cost_variance(actual: float, expected: float, tolerance: float = 0.25) -> tuple[bool, float]:
    """
    Check if actual cost is within tolerance of expected cost.

    Args:
        actual: Actual generated cost
        expected: Expected HomeAdvisor cost
        tolerance: Acceptable variance (default 0.25 = ±25%)

    Returns:
        Tuple of (is_valid, variance_pct)
    """
    variance = abs(actual - expected) / expected
    is_valid = variance <= tolerance
    return is_valid, variance * 100


async def validate_reference_classes():
    """Validate all synthetic reference classes against benchmarks."""
    print("🔍 Validating synthetic reference classes...\n")

    await connect_to_mongo()
    collection = get_reference_classes_collection()

    # Fetch all synthetic reference classes
    cursor = collection.find({"is_synthetic": True})
    ref_classes = await cursor.to_list(length=None)

    if not ref_classes:
        print("❌ No synthetic reference classes found in database")
        print("   Run seed_database.py first")
        await close_mongo_connection()
        return

    print(f"Found {len(ref_classes)} synthetic reference classes\n")

    # Validation results
    total_checked = 0
    total_passed = 0
    total_failed = 0
    failures: List[Dict] = []

    # Validate each reference class
    for rc in ref_classes:
        subcategory = rc.get('subcategory')
        name = rc.get('name', '')
        actual_p50 = rc['cost_distribution']['p50']
        region = rc['regions'][0] if rc.get('regions') else ''

        # Only validate SoCal Coastal (baseline) to avoid regional adjustment confusion
        if 'coastal' not in region.lower():
            continue

        # Determine expected cost based on subcategory and name
        expected_cost = None

        if subcategory == 'pool':
            if 'Small' in name:
                expected_cost = BENCHMARK_COSTS['pool']['small']
            elif 'Medium' in name:
                expected_cost = BENCHMARK_COSTS['pool']['medium']
            elif 'Large' in name:
                expected_cost = BENCHMARK_COSTS['pool']['large']
            elif 'Luxury' in name:
                expected_cost = BENCHMARK_COSTS['pool']['luxury']

        elif subcategory == 'adu':
            if 'Studio' in name:
                expected_cost = BENCHMARK_COSTS['adu']['studio']
            elif '1 Bedroom' in name:
                expected_cost = BENCHMARK_COSTS['adu']['one_bedroom']
            elif '2 Bedroom' in name:
                expected_cost = BENCHMARK_COSTS['adu']['two_bedroom']

        elif subcategory == 'renovation' and 'Kitchen' in name:
            if 'Budget' in name:
                expected_cost = BENCHMARK_COSTS['kitchen']['budget']
            elif 'Midrange' in name:
                expected_cost = BENCHMARK_COSTS['kitchen']['midrange']
            elif 'Upscale' in name:
                expected_cost = BENCHMARK_COSTS['kitchen']['upscale']

        elif subcategory == 'renovation' and 'Bathroom' in name:
            if 'Basic' in name:
                expected_cost = BENCHMARK_COSTS['bathroom']['basic']
            elif 'Midrange' in name:
                expected_cost = BENCHMARK_COSTS['bathroom']['midrange']
            elif 'Upscale' in name:
                expected_cost = BENCHMARK_COSTS['bathroom']['upscale']

        elif subcategory == 'landscaping':
            if 'Basic' in name:
                expected_cost = BENCHMARK_COSTS['landscaping']['basic']
            elif 'Standard' in name:
                expected_cost = BENCHMARK_COSTS['landscaping']['standard']
            elif 'Premium' in name:
                expected_cost = BENCHMARK_COSTS['landscaping']['premium']

        elif subcategory == 'roofing':
            for material in ['asphalt_shingle', 'architectural_shingle', 'tile', 'metal']:
                if material.replace('_', ' ').title() in name:
                    expected_cost = BENCHMARK_COSTS['roofing'][material]
                    break

        elif subcategory == 'flooring':
            for material in ['laminate', 'vinyl', 'hardwood', 'tile']:
                if material.title() in name:
                    expected_cost = BENCHMARK_COSTS['flooring'][material]
                    break

        if expected_cost:
            total_checked += 1
            is_valid, variance_pct = check_cost_variance(actual_p50, expected_cost)

            if is_valid:
                total_passed += 1
            else:
                total_failed += 1
                failures.append({
                    'name': name,
                    'expected': expected_cost,
                    'actual': actual_p50,
                    'variance': variance_pct
                })

    # Report results
    print("=" * 80)
    print(f"Validation Results:")
    print(f"  Total checked:  {total_checked}")
    print(f"  ✓ Passed:       {total_passed} ({total_passed/total_checked*100:.1f}%)")
    print(f"  ✗ Failed:       {total_failed} ({total_failed/total_checked*100:.1f}%)")
    print("=" * 80)

    if failures:
        print("\n❌ Validation Failures (>25% variance from HomeAdvisor 2024):")
        for failure in failures:
            print(f"  • {failure['name']}")
            print(f"    Expected: ${failure['expected']:,.0f}")
            print(f"    Actual:   ${failure['actual']:,.0f}")
            print(f"    Variance: {failure['variance']:.1f}%\n")
    else:
        print("\n✅ All synthetic costs are within ±25% of HomeAdvisor 2024 averages!")

    await close_mongo_connection()


if __name__ == "__main__":
    asyncio.run(validate_reference_classes())
