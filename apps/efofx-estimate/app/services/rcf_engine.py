"""
RCF (Reference Class Forecasting) Matching Engine for efOfX Estimation Service.

This module implements the reference class matching algorithm that finds
the best matching reference class based on project description, category, and region.
"""

import logging
import time
from typing import Optional, List, Dict, Any, Tuple
from datetime import datetime, timedelta
from app.db.mongodb import get_tenant_collection
from app.core.constants import DB_COLLECTIONS
from app.models.reference_class import ReferenceClass

logger = logging.getLogger(__name__)

# In-memory cache with TTL (5 minutes as per story requirements)
_match_cache: Dict[str, Tuple[Any, datetime]] = {}
_CACHE_TTL_SECONDS = 300  # 5 minutes


def _get_cache_key(
    description: str, category: str, region: str, tenant_id: Optional[str]
) -> str:
    """Generate cache key for match query."""
    return f"{description}:{category}:{region}:{tenant_id or 'platform'}"


def _get_from_cache(cache_key: str) -> Optional[Any]:
    """Get result from cache if not expired."""
    if cache_key in _match_cache:
        result, timestamp = _match_cache[cache_key]
        if datetime.utcnow() - timestamp < timedelta(seconds=_CACHE_TTL_SECONDS):
            logger.debug(f"Cache hit for key: {cache_key}")
            return result
        else:
            # Remove expired entry
            del _match_cache[cache_key]
    return None


def _set_in_cache(cache_key: str, result: Any) -> None:
    """Store result in cache with current timestamp."""
    _match_cache[cache_key] = (result, datetime.utcnow())


def extract_keywords(description: str) -> List[str]:
    """
    Extract keywords from project description.

    Converts to lowercase and tokenizes by splitting on whitespace and common separators.  # noqa: E501
    Filters out very short words (< 2 chars) and common stop words.

    Args:
        description: Project description text

    Returns:
        List of normalized keywords
    """
    # Common stop words to filter out
    stop_words = {
        "a",
        "an",
        "and",
        "are",
        "as",
        "at",
        "be",
        "by",
        "for",
        "from",
        "has",
        "he",
        "in",
        "is",
        "it",
        "its",
        "of",
        "on",
        "that",
        "the",
        "to",
        "was",
        "will",
        "with",
        "we",
        "i",
        "you",
        "my",
        "our",
        "want",
    }

    # Lowercase and split on whitespace and common punctuation
    words = (
        description.lower()
        .replace(",", " ")
        .replace(".", " ")
        .replace(";", " ")
        .split()
    )

    # Filter: remove stop words and very short words
    keywords = [w.strip("()[]{}!?:;\"'") for w in words]
    keywords = [k for k in keywords if len(k) >= 2 and k not in stop_words]

    return keywords


def calculate_keyword_overlap(
    desc_keywords: List[str], rc_keywords: List[str]
) -> float:
    """
    Calculate keyword overlap score between description and reference class.

    Args:
        desc_keywords: Keywords from project description
        rc_keywords: Keywords from reference class

    Returns:
        Overlap ratio (0.0 to 1.0)
    """
    if not desc_keywords:
        return 0.0

    # Convert to sets for efficient intersection
    desc_set = set(desc_keywords)
    rc_set = set(rc_keywords)

    matches = desc_set & rc_set
    overlap_ratio = len(matches) / len(desc_set) if desc_set else 0.0

    return overlap_ratio


def calculate_confidence_score(
    keyword_overlap: float, category_match: bool, region_match: bool
) -> float:
    """
    Calculate overall confidence score using weighted formula.

    Formula per story requirements:
    (keyword_overlap) * 0.6 + (category_match) * 0.3 + (region_match) * 0.1

    Args:
        keyword_overlap: Keyword overlap ratio (0.0 to 1.0)
        category_match: Whether categories match exactly
        region_match: Whether region is in reference class regions

    Returns:
        Confidence score (0.0 to 1.0)
    """
    score = (
        keyword_overlap * 0.6
        + (1.0 if category_match else 0.0) * 0.3
        + (1.0 if region_match else 0.0) * 0.1
    )

    return min(1.0, max(0.0, score))  # Clamp to [0.0, 1.0]


def check_region_match(region: str, rc_regions: List[str]) -> bool:
    """
    Check if region matches any of the reference class regions.

    Args:
        region: User's region
        rc_regions: List of regions supported by reference class

    Returns:
        True if region is in rc_regions
    """
    return region.lower() in [r.lower() for r in rc_regions]


async def find_matching_reference_class(
    description: str, category: str, region: str, tenant_id: Optional[str] = None
) -> Optional[Dict[str, Any]]:
    """
    Find best matching reference class for a project description.

    Matches based on:
    - Keyword overlap from description
    - Category exact match
    - Region match

    Returns reference class with confidence >= 0.7, preferring tenant-specific
    over platform-provided when scores are tied.

    Args:
        description: Project description from user
        category: Project category (e.g., 'construction', 'it_dev')
        region: Project region (e.g., 'us-ca-south')
        tenant_id: Optional tenant ID for tenant-specific matches

    Returns:
        Dict with 'reference_class' and 'confidence' keys, or None if no good match

    Raises:
        ValueError: If confidence < 0.7 (suggesting more details needed)
    """
    start_time = time.perf_counter()

    # Check cache first
    cache_key = _get_cache_key(description, category, region, tenant_id)
    cached_result = _get_from_cache(cache_key)
    if cached_result is not None:
        elapsed_ms = (time.perf_counter() - start_time) * 1000
        logger.info(f"RCF match (cached) completed in {elapsed_ms:.2f}ms")
        return cached_result

    try:
        # Extract keywords from description
        desc_keywords = extract_keywords(description)

        if not desc_keywords:
            logger.warning(f"No keywords extracted from description: {description}")
            raise ValueError("Please provide more details in your project description")

        # Query reference classes using TenantAwareCollection for hard isolation.
        # When tenant_id is provided, allow_platform_data=True includes platform
        # data (tenant_id=None) alongside the tenant's own classes via $or filter.
        # When no tenant_id, query platform-only data directly via raw collection.
        if tenant_id:
            collection = get_tenant_collection(
                DB_COLLECTIONS["REFERENCE_CLASSES"],
                tenant_id,
                allow_platform_data=True,
            )
            query = {"category": category}
        else:
            # No tenant context — query platform data (tenant_id=None) only
            from app.db.mongodb import get_collection

            collection = get_collection(DB_COLLECTIONS["REFERENCE_CLASSES"])
            query = {"category": category, "tenant_id": None}

        cursor = collection.find(query)
        reference_classes = await cursor.to_list(length=None)

        if not reference_classes:
            logger.warning(f"No reference classes found for category: {category}")
            raise ValueError(
                f"No reference classes available for category '{category}'"
            )

        # Score each reference class
        scored_matches: List[Tuple[Dict[str, Any], float, bool]] = []

        for rc_doc in reference_classes:
            # Check category match (should always be true given query, but being explicit)  # noqa: E501
            category_match = rc_doc.get("category", "").lower() == category.lower()

            # Check region match
            rc_regions = rc_doc.get("regions", [])
            region_match = check_region_match(region, rc_regions)

            # Calculate keyword overlap
            rc_keywords = rc_doc.get("keywords", [])
            keyword_overlap = calculate_keyword_overlap(desc_keywords, rc_keywords)

            # Calculate confidence score
            confidence = calculate_confidence_score(
                keyword_overlap, category_match, region_match
            )

            # Track if this is tenant-specific
            is_tenant_specific = (
                rc_doc.get("tenant_id") == tenant_id if tenant_id else False
            )

            scored_matches.append((rc_doc, confidence, is_tenant_specific))

        # Sort by confidence (desc), then by tenant-specific preference
        scored_matches.sort(key=lambda x: (x[1], x[2]), reverse=True)

        # Get top match
        if not scored_matches:
            raise ValueError("No matching reference classes found")

        top_match, top_confidence, is_tenant = scored_matches[0]

        # Log match attempt
        elapsed_ms = (time.perf_counter() - start_time) * 1000
        logger.info(
            f"RCF match attempt: category={category}, region={region}, "
            f"confidence={top_confidence:.3f}, tenant_specific={is_tenant}, "
            f"elapsed={elapsed_ms:.2f}ms"
        )

        # Check confidence threshold
        if top_confidence < 0.7:
            logger.warning(
                f"Low confidence match: {top_confidence:.3f} < 0.7 for description: {description[:50]}..."  # noqa: E501
            )
            raise ValueError(
                f"Could not find a confident match (confidence: {top_confidence:.2f}). "
                "Please provide more specific details about your project."
            )

        # Convert to ReferenceClass model for response
        # Convert MongoDB ObjectId to string for Pydantic validation
        top_match_dict = dict(top_match)
        if "_id" in top_match_dict:
            top_match_dict["_id"] = str(top_match_dict["_id"])
        rc_model = ReferenceClass(**top_match_dict)

        result = {
            "reference_class": rc_model.model_dump(by_alias=True),
            "confidence": round(top_confidence, 3),
            "match_metadata": {
                "description_keywords": desc_keywords,
                "matched_keywords": list(
                    set(desc_keywords) & set(top_match.get("keywords", []))
                ),
                "is_tenant_specific": is_tenant,
                "processing_time_ms": round(elapsed_ms, 2),
            },
        }

        # Cache the result
        _set_in_cache(cache_key, result)

        return result

    except ValueError:
        # Re-raise ValueError for confidence threshold failures
        raise
    except Exception as e:
        elapsed_ms = (time.perf_counter() - start_time) * 1000
        logger.error(f"Error in RCF matching after {elapsed_ms:.2f}ms: {e}")
        raise


def clear_match_cache() -> None:
    """Clear the entire match cache. Useful for testing or after data updates."""
    global _match_cache
    _match_cache.clear()
    logger.info("RCF match cache cleared")


def calculate_baseline_estimate(reference_class: Dict[str, Any]) -> Dict[str, Any]:
    """
    Calculate baseline P50/P80 cost and timeline estimates from reference class.

    This function extracts probabilistic cost and timeline forecasts directly from
    the matched reference class without applying any regional or complexity adjustments
    (those are handled in Story 2.5).

    Args:
        reference_class: Reference class dictionary from find_matching_reference_class()

    Returns:
        Dict with baseline estimate containing:
        - p50_cost: Median cost estimate
        - p80_cost: 80th percentile cost estimate
        - variance: Difference between p80 and p50
        - p50_timeline_days: Median timeline in days
        - p80_timeline_days: 80th percentile timeline in days
        - cost_breakdown: Dict of costs by category at P50 level
        - reference_class_name: Name of reference class used
    """
    start_time = time.perf_counter()

    # Extract cost distribution
    cost_dist = reference_class["cost_distribution"]
    p50_cost = cost_dist["p50"]
    p80_cost = cost_dist["p80"]
    variance = p80_cost - p50_cost

    # Extract timeline distribution
    timeline_dist = reference_class["timeline_distribution"]
    p50_timeline = timeline_dist["p50_days"]
    p80_timeline = timeline_dist["p80_days"]

    # Calculate cost breakdown at P50 level
    breakdown_template = reference_class["cost_breakdown_template"]
    cost_breakdown = {}

    # Calculate each category's cost
    for category, percentage in breakdown_template.items():
        cost_breakdown[category] = round(p50_cost * percentage, 2)

    # Handle rounding to ensure breakdown sums exactly to P50
    breakdown_total = sum(cost_breakdown.values())
    if breakdown_total != p50_cost:
        # Adjust the largest category to make up the difference
        largest_category = max(
            breakdown_template.keys(), key=lambda k: breakdown_template[k]
        )
        adjustment = round(p50_cost - breakdown_total, 2)
        cost_breakdown[largest_category] += adjustment

    # Verify breakdown sums correctly
    final_total = sum(cost_breakdown.values())
    if abs(final_total - p50_cost) > 0.01:  # Allow 1 cent tolerance
        logger.warning(
            f"Cost breakdown total ({final_total}) doesn't match p50_cost ({p50_cost})"
        )

    elapsed_ms = (time.perf_counter() - start_time) * 1000

    logger.info(
        f"Baseline estimate calculated: p50=${p50_cost:,.0f}, p80=${p80_cost:,.0f}, "
        f"timeline={p50_timeline}d, reference_class={reference_class.get('name')}, "
        f"elapsed={elapsed_ms:.2f}ms"
    )

    return {
        "p50_cost": p50_cost,
        "p80_cost": p80_cost,
        "variance": variance,
        "p50_timeline_days": p50_timeline,
        "p80_timeline_days": p80_timeline,
        "cost_breakdown": cost_breakdown,
        "reference_class_name": reference_class.get("name", "Unknown"),
        "calculation_time_ms": round(elapsed_ms, 2),
    }


# Complexity multiplier mapping
COMPLEXITY_MULTIPLIERS = {
    "simple": 0.8,
    "standard": 1.0,
    "complex": 1.5,
}


def apply_adjustments(
    baseline_estimate: Dict[str, Any],
    complexity: Optional[str] = None,
    risk_level: Optional[str] = None,
) -> Dict[str, Any]:
    """
    Apply regional, complexity, and risk adjustments to baseline estimate.

    Regional adjustments are already baked into the reference class costs.
    This function applies additional complexity and risk multipliers.

    Args:
        baseline_estimate: Result from calculate_baseline_estimate()
        complexity: Complexity level ('simple', 'standard', 'complex'), default 'standard'  # noqa: E501
        risk_level: Risk level ('low', 'medium', 'high'), default 'low'

    Returns:
        Dict with adjusted estimate containing:
        - baseline_p50: Original P50 cost
        - baseline_p80: Original P80 cost
        - complexity_factor: Applied complexity multiplier
        - risk_factor: Applied risk multiplier
        - adjusted_p50: Final P50 cost after adjustments
        - adjusted_p80: Final P80 cost after adjustments
        - adjusted_variance: Variance after adjustments
        - baseline_p50_timeline: Original P50 timeline
        - adjusted_p50_timeline: Timeline after complexity adjustment
        - adjusted_p80_timeline: Timeline after complexity adjustment
        - cost_breakdown: Adjusted cost breakdown
        - adjustment_summary: Text summary of applied adjustments
    """
    start_time = time.perf_counter()

    # Get baseline values
    baseline_p50 = baseline_estimate["p50_cost"]
    baseline_p80 = baseline_estimate["p80_cost"]
    baseline_p50_timeline = baseline_estimate["p50_timeline_days"]
    baseline_p80_timeline = baseline_estimate["p80_timeline_days"]
    baseline_breakdown = baseline_estimate["cost_breakdown"]

    # Determine complexity factor
    complexity = (complexity or "standard").lower()
    complexity_factor = COMPLEXITY_MULTIPLIERS.get(complexity, 1.0)

    # Determine risk factor
    risk_level = (risk_level or "low").lower()
    risk_factors = {
        "low": 1.0,
        "medium": 1.15,
        "high": 1.3,
    }
    risk_factor = risk_factors.get(risk_level, 1.0)

    # Apply adjustments to costs
    # Formula: final_cost = baseline × complexity × risk
    adjusted_p50 = round(baseline_p50 * complexity_factor * risk_factor, 2)
    adjusted_p80 = round(baseline_p80 * complexity_factor * risk_factor, 2)
    adjusted_variance = adjusted_p80 - adjusted_p50

    # Apply complexity adjustment to timelines (risk doesn't affect timeline)
    # Formula: final_timeline = baseline_timeline × complexity
    adjusted_p50_timeline = int(baseline_p50_timeline * complexity_factor)
    adjusted_p80_timeline = int(baseline_p80_timeline * complexity_factor)

    # Adjust cost breakdown proportionally
    adjusted_breakdown = {}
    for category, baseline_cost in baseline_breakdown.items():
        adjusted_breakdown[category] = round(
            baseline_cost * complexity_factor * risk_factor, 2
        )

    # Verify adjusted breakdown sums to adjusted P50
    breakdown_total = sum(adjusted_breakdown.values())
    if abs(breakdown_total - adjusted_p50) > 0.01:
        # Adjust largest category to handle rounding
        largest_category = max(
            baseline_breakdown.keys(), key=lambda k: baseline_breakdown[k]
        )
        adjustment = round(adjusted_p50 - breakdown_total, 2)
        adjusted_breakdown[largest_category] += adjustment

    # Create adjustment summary
    adjustments = []
    if complexity_factor != 1.0:
        adjustments.append(f"complexity={complexity} ({complexity_factor}x)")
    if risk_factor != 1.0:
        adjustments.append(f"risk={risk_level} ({risk_factor}x)")

    if not adjustments:
        adjustment_summary = "No adjustments applied (standard complexity, low risk)"
    else:
        adjustment_summary = f"Applied adjustments: {', '.join(adjustments)}"

    elapsed_ms = (time.perf_counter() - start_time) * 1000

    logger.info(
        f"Adjustments applied: complexity={complexity}({complexity_factor}x), "
        f"risk={risk_level}({risk_factor}x), "
        f"baseline_p50=${baseline_p50:,.0f} → adjusted_p50=${adjusted_p50:,.0f}, "
        f"elapsed={elapsed_ms:.2f}ms"
    )

    return {
        # Baseline values
        "baseline_p50": baseline_p50,
        "baseline_p80": baseline_p80,
        "baseline_variance": baseline_estimate.get(
            "variance", baseline_p80 - baseline_p50
        ),
        "baseline_p50_timeline": baseline_p50_timeline,
        "baseline_p80_timeline": baseline_p80_timeline,
        # Adjustment factors
        "complexity": complexity,
        "complexity_factor": complexity_factor,
        "risk_level": risk_level,
        "risk_factor": risk_factor,
        # Adjusted values
        "adjusted_p50": adjusted_p50,
        "adjusted_p80": adjusted_p80,
        "adjusted_variance": adjusted_variance,
        "adjusted_p50_timeline": adjusted_p50_timeline,
        "adjusted_p80_timeline": adjusted_p80_timeline,
        # Breakdown and metadata
        "cost_breakdown": adjusted_breakdown,
        "adjustment_summary": adjustment_summary,
        "reference_class_name": baseline_estimate.get(
            "reference_class_name", "Unknown"
        ),
        "calculation_time_ms": round(elapsed_ms, 2),
    }
