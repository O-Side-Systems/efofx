"""
Tests for RCF (Reference Class Forecasting) Matching Engine.

Tests cover:
- Keyword extraction
- Scoring logic
- Confidence calculation
- Cache functionality
- Performance requirements (<50ms p95)
- Error handling (confidence < 0.7)
- Tenant-specific preference
"""

import pytest
import pytest_asyncio
import asyncio
import time
from typing import List, Dict, Any
from datetime import datetime

from motor.motor_asyncio import AsyncIOMotorClient
import app.db.mongodb as _mdb

from app.services.rcf_engine import (
    extract_keywords,
    calculate_keyword_overlap,
    calculate_confidence_score,
    check_region_match,
    find_matching_reference_class,
    clear_match_cache,
    _get_cache_key,
    _get_from_cache,
    _set_in_cache,
    calculate_baseline_estimate,
    apply_adjustments,
)
from app.db.mongodb import get_reference_classes_collection
from app.models.reference_class import ReferenceClass, CostDistribution, TimelineDistribution


class TestKeywordExtraction:
    """Test keyword extraction functionality."""

    def test_extract_keywords_basic(self):
        """Test basic keyword extraction."""
        description = "I want to build a swimming pool in my backyard"
        keywords = extract_keywords(description)

        assert "build" in keywords
        assert "swimming" in keywords
        assert "pool" in keywords
        assert "backyard" in keywords

        # Stop words should be filtered
        assert "i" not in keywords
        assert "want" not in keywords
        assert "to" not in keywords
        assert "a" not in keywords
        assert "in" not in keywords
        assert "my" not in keywords

    def test_extract_keywords_with_punctuation(self):
        """Test keyword extraction with punctuation."""
        description = "Need a pool, spa, and decking. Budget: $50,000!"
        keywords = extract_keywords(description)

        assert "need" in keywords
        assert "pool" in keywords
        assert "spa" in keywords
        assert "decking" in keywords
        assert "budget" in keywords
        assert "$50,000" not in keywords or "50,000" in keywords

    def test_extract_keywords_case_insensitive(self):
        """Test that keywords are lowercase."""
        description = "POOL Swimming BACKYARD"
        keywords = extract_keywords(description)

        assert "pool" in keywords
        assert "swimming" in keywords
        assert "backyard" in keywords
        assert "POOL" not in keywords

    def test_extract_keywords_empty_description(self):
        """Test with empty description."""
        keywords = extract_keywords("")
        assert keywords == []

    def test_extract_keywords_filters_short_words(self):
        """Test that very short words are filtered."""
        description = "I a pool in LA"
        keywords = extract_keywords(description)

        # "i" and "a" should be filtered (too short or stop words)
        assert "i" not in keywords
        assert "a" not in keywords
        # "la" should be kept (2 chars)
        assert "la" in keywords


class TestKeywordOverlap:
    """Test keyword overlap calculation."""

    def test_keyword_overlap_perfect_match(self):
        """Test perfect keyword overlap."""
        desc_keywords = ["pool", "swimming", "backyard"]
        rc_keywords = ["pool", "swimming", "backyard"]

        overlap = calculate_keyword_overlap(desc_keywords, rc_keywords)
        assert overlap == 1.0

    def test_keyword_overlap_partial_match(self):
        """Test partial keyword overlap."""
        desc_keywords = ["pool", "swimming", "backyard", "concrete"]
        rc_keywords = ["pool", "swimming"]

        overlap = calculate_keyword_overlap(desc_keywords, rc_keywords)
        assert overlap == 0.5  # 2 out of 4 keywords match

    def test_keyword_overlap_no_match(self):
        """Test no keyword overlap."""
        desc_keywords = ["kitchen", "renovation"]
        rc_keywords = ["pool", "swimming"]

        overlap = calculate_keyword_overlap(desc_keywords, rc_keywords)
        assert overlap == 0.0

    def test_keyword_overlap_empty_description(self):
        """Test with empty description keywords."""
        desc_keywords = []
        rc_keywords = ["pool", "swimming"]

        overlap = calculate_keyword_overlap(desc_keywords, rc_keywords)
        assert overlap == 0.0


class TestConfidenceScore:
    """Test confidence score calculation."""

    def test_confidence_score_perfect_match(self):
        """Test perfect match score."""
        score = calculate_confidence_score(
            keyword_overlap=1.0,
            category_match=True,
            region_match=True
        )
        # 1.0 * 0.6 + 1.0 * 0.3 + 1.0 * 0.1 = 1.0
        assert abs(score - 1.0) < 0.001

    def test_confidence_score_keyword_only(self):
        """Test score with only keyword match."""
        score = calculate_confidence_score(
            keyword_overlap=1.0,
            category_match=False,
            region_match=False
        )
        # 1.0 * 0.6 + 0 + 0 = 0.6
        assert score == 0.6

    def test_confidence_score_category_only(self):
        """Test score with only category match."""
        score = calculate_confidence_score(
            keyword_overlap=0.0,
            category_match=True,
            region_match=False
        )
        # 0 + 1.0 * 0.3 + 0 = 0.3
        assert score == 0.3

    def test_confidence_score_partial(self):
        """Test score with partial matches."""
        score = calculate_confidence_score(
            keyword_overlap=0.5,
            category_match=True,
            region_match=False
        )
        # 0.5 * 0.6 + 1.0 * 0.3 + 0 = 0.3 + 0.3 = 0.6
        assert score == 0.6

    def test_confidence_score_weights(self):
        """Test that weights are applied correctly per story formula."""
        score = calculate_confidence_score(
            keyword_overlap=0.8,
            category_match=True,
            region_match=True
        )
        # 0.8 * 0.6 + 1.0 * 0.3 + 1.0 * 0.1 = 0.48 + 0.3 + 0.1 = 0.88
        assert abs(score - 0.88) < 0.001


class TestRegionMatch:
    """Test region matching functionality."""

    def test_region_match_exact(self):
        """Test exact region match."""
        assert check_region_match("us-ca-south", ["us-ca-south", "us-ca-north"])

    def test_region_match_case_insensitive(self):
        """Test case-insensitive region match."""
        assert check_region_match("US-CA-SOUTH", ["us-ca-south"])
        assert check_region_match("us-ca-south", ["US-CA-SOUTH"])

    def test_region_no_match(self):
        """Test region with no match."""
        assert not check_region_match("us-tx-houston", ["us-ca-south", "us-ca-north"])

    def test_region_empty_list(self):
        """Test with empty region list."""
        assert not check_region_match("us-ca-south", [])


class TestCacheOperations:
    """Test cache functionality."""

    def test_cache_key_generation(self):
        """Test cache key generation."""
        key1 = _get_cache_key("pool project", "construction", "us-ca", "tenant1")
        key2 = _get_cache_key("pool project", "construction", "us-ca", "tenant1")
        key3 = _get_cache_key("different project", "construction", "us-ca", "tenant1")

        assert key1 == key2
        assert key1 != key3

    def test_cache_set_and_get(self):
        """Test setting and getting from cache."""
        clear_match_cache()

        cache_key = "test_key"
        test_data = {"result": "test_value"}

        _set_in_cache(cache_key, test_data)
        result = _get_from_cache(cache_key)

        assert result == test_data

    def test_cache_miss(self):
        """Test cache miss returns None."""
        clear_match_cache()

        result = _get_from_cache("nonexistent_key")
        assert result is None

    def test_cache_clear(self):
        """Test cache clearing."""
        cache_key = "test_key"
        test_data = {"result": "test_value"}

        _set_in_cache(cache_key, test_data)
        clear_match_cache()

        result = _get_from_cache(cache_key)
        assert result is None


class TestRCFMatching:
    """Test the main RCF matching algorithm."""

    @pytest_asyncio.fixture(autouse=True)
    async def setup_test_data(self):
        """Set up test reference classes in database using per-test Motor client."""
        from app.core.config import settings as _settings

        clear_match_cache()

        # Per-test client to avoid event-loop conflicts with session-scoped fixtures
        client = AsyncIOMotorClient(_settings.MONGO_URI)
        db = client[_settings.MONGO_DB_NAME]
        _mdb._client = client
        _mdb._database = db

        collection = get_reference_classes_collection()
        await collection.delete_many({})  # Clean slate

        # Create test reference classes
        test_classes = [
            {
                "_id": "rc_pool_1",
                "tenant_id": None,  # Platform-provided
                "category": "construction",
                "subcategory": "pool",
                "name": "Residential Pool - Midrange",
                "description": "Standard residential swimming pool construction",
                "keywords": ["pool", "swimming", "residential", "concrete"],
                "regions": ["us-ca-south", "us-west"],
                "attributes": {"size_range": "300-500 sq ft"},
                "cost_distribution": {
                    "p50": 55000,
                    "p80": 72000,
                    "p95": 95000,
                    "currency": "USD"
                },
                "timeline_distribution": {
                    "p50_days": 45,
                    "p80_days": 60,
                    "p95_days": 90
                },
                "cost_breakdown_template": {
                    "materials": 0.40,
                    "labor": 0.30,
                    "permits": 0.05,
                    "overhead": 0.25
                },
                "is_synthetic": True,
                "validation_source": "test",
                "created_at": datetime.utcnow()
            },
            {
                "_id": "rc_pool_2",
                "tenant_id": "tenant_123",  # Tenant-specific
                "category": "construction",
                "subcategory": "pool",
                "name": "Tenant Pool Spec",
                "description": "Tenant-specific pool reference",
                "keywords": ["pool", "swimming", "residential", "custom"],
                "regions": ["us-ca-south"],
                "attributes": {"size_range": "300-500 sq ft"},
                "cost_distribution": {
                    "p50": 60000,
                    "p80": 75000,
                    "p95": 100000,
                    "currency": "USD"
                },
                "timeline_distribution": {
                    "p50_days": 50,
                    "p80_days": 65,
                    "p95_days": 95
                },
                "cost_breakdown_template": {
                    "materials": 0.40,
                    "labor": 0.30,
                    "permits": 0.05,
                    "overhead": 0.25
                },
                "is_synthetic": True,
                "validation_source": "test",
                "created_at": datetime.utcnow()
            },
            {
                "_id": "rc_kitchen_1",
                "tenant_id": None,
                "category": "construction",
                "subcategory": "renovation",
                "name": "Kitchen Renovation",
                "description": "Kitchen remodeling project",
                "keywords": ["kitchen", "renovation", "remodel", "cabinets"],
                "regions": ["us-ca-south", "us-ca-north"],
                "attributes": {},
                "cost_distribution": {
                    "p50": 30000,
                    "p80": 45000,
                    "p95": 65000,
                    "currency": "USD"
                },
                "timeline_distribution": {
                    "p50_days": 30,
                    "p80_days": 45,
                    "p95_days": 60
                },
                "cost_breakdown_template": {
                    "materials": 0.50,
                    "labor": 0.25,
                    "permits": 0.05,
                    "overhead": 0.20
                },
                "is_synthetic": True,
                "validation_source": "test",
                "created_at": datetime.utcnow()
            }
        ]

        await collection.insert_many(test_classes)

        yield

        # Cleanup
        await collection.delete_many({})
        clear_match_cache()

        client.close()
        _mdb._client = None
        _mdb._database = None

    @pytest.mark.asyncio
    async def test_successful_match(self):
        """Test successful reference class matching."""
        result = await find_matching_reference_class(
            description="I want to build a residential swimming pool in my backyard",
            category="construction",
            region="us-ca-south",
            tenant_id=None
        )

        assert result is not None
        assert result["confidence"] >= 0.7
        assert result["reference_class"]["category"] == "construction"
        assert "pool" in result["reference_class"]["keywords"]

    @pytest.mark.asyncio
    async def test_tenant_specific_preference(self):
        """Test that tenant-specific reference classes are preferred when scores are tied."""
        # This should match the tenant-specific pool reference
        result = await find_matching_reference_class(
            description="I want a custom residential swimming pool",
            category="construction",
            region="us-ca-south",
            tenant_id="tenant_123"
        )

        assert result is not None
        assert result["reference_class"]["tenant_id"] == "tenant_123"
        assert result["match_metadata"]["is_tenant_specific"] is True

    @pytest.mark.asyncio
    async def test_low_confidence_raises_error(self):
        """Test that low confidence (<0.7) raises ValueError."""
        with pytest.raises(ValueError, match="Could not find a confident match"):
            await find_matching_reference_class(
                description="something vague",  # Very generic, low keyword overlap
                category="construction",
                region="us-tx-houston",  # Different region
                tenant_id=None
            )

    @pytest.mark.asyncio
    async def test_no_category_match_raises_error(self):
        """Test that non-existent category raises ValueError."""
        with pytest.raises(ValueError, match="No reference classes available"):
            await find_matching_reference_class(
                description="build an IT system",
                category="it_dev",  # No reference classes for this category
                region="us-ca-south",
                tenant_id=None
            )

    @pytest.mark.asyncio
    async def test_empty_description_raises_error(self):
        """Test that empty description raises ValueError."""
        with pytest.raises(ValueError, match="provide more details"):
            await find_matching_reference_class(
                description="",
                category="construction",
                region="us-ca-south",
                tenant_id=None
            )

    @pytest.mark.asyncio
    async def test_caching_works(self):
        """Test that caching reduces processing time."""
        description = "residential swimming pool project"
        category = "construction"
        region = "us-ca-south"
        tenant_id = None

        # First call - not cached
        start = time.perf_counter()
        result1 = await find_matching_reference_class(description, category, region, tenant_id)
        time1 = (time.perf_counter() - start) * 1000

        # Second call - should be cached
        start = time.perf_counter()
        result2 = await find_matching_reference_class(description, category, region, tenant_id)
        time2 = (time.perf_counter() - start) * 1000

        assert result1 == result2
        # Cached call should be significantly faster (at least 2x)
        assert time2 < time1 / 2

    @pytest.mark.asyncio
    async def test_match_metadata_includes_keywords(self):
        """Test that match metadata includes keyword information."""
        result = await find_matching_reference_class(
            description="I want to build a swimming pool",
            category="construction",
            region="us-ca-south",
            tenant_id=None
        )

        assert "match_metadata" in result
        assert "description_keywords" in result["match_metadata"]
        assert "matched_keywords" in result["match_metadata"]
        assert "processing_time_ms" in result["match_metadata"]

        # Check that some keywords matched
        assert len(result["match_metadata"]["matched_keywords"]) > 0

    @pytest.mark.asyncio
    async def test_performance_requirement(self):
        """Test that matching completes in < 50ms (p95 requirement)."""
        description = "residential swimming pool with concrete finish"
        category = "construction"
        region = "us-ca-south"

        # Run multiple times to get p95
        times = []
        for _ in range(20):
            clear_match_cache()  # Clear cache for each run
            start = time.perf_counter()
            await find_matching_reference_class(description, category, region, None)
            elapsed = (time.perf_counter() - start) * 1000
            times.append(elapsed)

        # Calculate p95
        times.sort()
        p95_index = int(len(times) * 0.95)
        p95_time = times[p95_index]

        print(f"\nPerformance metrics:")
        print(f"  Min: {min(times):.2f}ms")
        print(f"  Median: {times[len(times)//2]:.2f}ms")
        print(f"  P95: {p95_time:.2f}ms")
        print(f"  Max: {max(times):.2f}ms")

        # Assert p95 < 50ms
        assert p95_time < 50.0, f"P95 performance ({p95_time:.2f}ms) exceeds 50ms requirement"

    @pytest.mark.asyncio
    async def test_different_regions_return_different_results(self):
        """Test that region affects matching."""
        description = "swimming pool project"
        category = "construction"

        result_ca = await find_matching_reference_class(
            description, category, "us-ca-south", None
        )

        # With a region that doesn't match any reference class perfectly,
        # the confidence should be different or error raised
        # (In this test setup, only us-ca-south/us-ca-north regions exist)
        try:
            result_tx = await find_matching_reference_class(
                description, category, "us-tx-houston", None
            )
            # If it succeeds, confidence should be lower due to region mismatch
            assert result_tx["confidence"] < result_ca["confidence"]
        except ValueError:
            # Or it might fail due to low confidence from region mismatch
            pass

    @pytest.mark.asyncio
    async def test_multiple_keyword_matches(self):
        """Test matching with various keyword overlaps."""
        # Good overlap - should succeed
        result1 = await find_matching_reference_class(
            description="residential swimming pool with concrete finish",
            category="construction",
            region="us-ca-south",
            tenant_id=None
        )
        assert result1["confidence"] >= 0.7

        # Moderate overlap - might succeed or fail depending on other factors
        try:
            result2 = await find_matching_reference_class(
                description="backyard water feature",
                category="construction",
                region="us-ca-south",
                tenant_id=None
            )
            # If it succeeds, confidence should be lower
            assert result2["confidence"] < result1["confidence"]
        except ValueError:
            # Low confidence is acceptable
            pass


class TestBaselineEstimateCalculation:
    """Test baseline estimate calculation from reference classes."""

    def test_calculate_baseline_estimate_basic(self):
        """Test basic baseline estimate calculation."""
        # Create a sample reference class
        ref_class = {
            "name": "Test Pool - Medium",
            "cost_distribution": {
                "p50": 55000.0,
                "p80": 72000.0,
                "p95": 95000.0,
                "currency": "USD"
            },
            "timeline_distribution": {
                "p50_days": 50,
                "p80_days": 65,
                "p95_days": 90
            },
            "cost_breakdown_template": {
                "materials": 0.40,
                "labor": 0.30,
                "equipment": 0.10,
                "permits": 0.05,
                "finishing": 0.15
            }
        }

        result = calculate_baseline_estimate(ref_class)

        # Verify P50/P80 costs
        assert result["p50_cost"] == 55000.0
        assert result["p80_cost"] == 72000.0
        assert result["variance"] == 17000.0  # 72000 - 55000

        # Verify timelines
        assert result["p50_timeline_days"] == 50
        assert result["p80_timeline_days"] == 65

        # Verify cost breakdown
        breakdown = result["cost_breakdown"]
        assert "materials" in breakdown
        assert "labor" in breakdown
        assert "equipment" in breakdown
        assert "permits" in breakdown
        assert "finishing" in breakdown

        # Verify breakdown sums to P50 (within 1 cent tolerance)
        total = sum(breakdown.values())
        assert abs(total - 55000.0) <= 0.01

        # Verify reference class name
        assert result["reference_class_name"] == "Test Pool - Medium"

    def test_cost_breakdown_percentages(self):
        """Test that cost breakdown calculates correct percentages."""
        ref_class = {
            "name": "Test",
            "cost_distribution": {"p50": 100000.0, "p80": 120000.0, "p95": 140000.0, "currency": "USD"},
            "timeline_distribution": {"p50_days": 30, "p80_days": 40, "p95_days": 50},
            "cost_breakdown_template": {
                "materials": 0.50,  # Should be 50000
                "labor": 0.30,      # Should be 30000
                "overhead": 0.20    # Should be 20000
            }
        }

        result = calculate_baseline_estimate(ref_class)
        breakdown = result["cost_breakdown"]

        # Check individual categories (allowing for rounding adjustments)
        assert abs(breakdown["materials"] - 50000.0) <= 1.0
        assert abs(breakdown["labor"] - 30000.0) <= 1.0
        assert abs(breakdown["overhead"] - 20000.0) <= 1.0

        # Verify total sums exactly to P50
        total = sum(breakdown.values())
        assert abs(total - 100000.0) <= 0.01

    def test_rounding_adjustment(self):
        """Test that rounding adjustment ensures breakdown sums to P50 exactly."""
        # Use a cost that will create rounding issues
        ref_class = {
            "name": "Test",
            "cost_distribution": {"p50": 33333.33, "p80": 40000.0, "p95": 50000.0, "currency": "USD"},
            "timeline_distribution": {"p50_days": 30, "p80_days": 40, "p95_days": 50},
            "cost_breakdown_template": {
                "materials": 0.40,
                "labor": 0.30,
                "equipment": 0.10,
                "permits": 0.05,
                "overhead": 0.15
            }
        }

        result = calculate_baseline_estimate(ref_class)
        breakdown = result["cost_breakdown"]

        # Verify breakdown sums exactly to P50
        total = sum(breakdown.values())
        assert abs(total - 33333.33) <= 0.01

    def test_performance_requirement(self):
        """Test that baseline estimate calculation completes in < 10ms."""
        ref_class = {
            "name": "Performance Test",
            "cost_distribution": {"p50": 50000.0, "p80": 65000.0, "p95": 80000.0, "currency": "USD"},
            "timeline_distribution": {"p50_days": 45, "p80_days": 60, "p95_days": 75},
            "cost_breakdown_template": {
                "materials": 0.40,
                "labor": 0.30,
                "equipment": 0.10,
                "permits": 0.05,
                "finishing": 0.15
            }
        }

        # Run multiple times to get average
        times = []
        for _ in range(100):
            start = time.perf_counter()
            calculate_baseline_estimate(ref_class)
            elapsed = (time.perf_counter() - start) * 1000
            times.append(elapsed)

        avg_time = sum(times) / len(times)
        max_time = max(times)

        print(f"\nBaseline estimate performance:")
        print(f"  Average: {avg_time:.2f}ms")
        print(f"  Max: {max_time:.2f}ms")

        # Assert average time is well below 10ms requirement
        assert avg_time < 10.0, f"Average time ({avg_time:.2f}ms) exceeds 10ms requirement"
        assert max_time < 10.0, f"Max time ({max_time:.2f}ms) exceeds 10ms requirement"

    def test_variance_calculation(self):
        """Test that variance is correctly calculated as p80 - p50."""
        ref_class = {
            "name": "Test",
            "cost_distribution": {"p50": 50000.0, "p80": 70000.0, "p95": 90000.0, "currency": "USD"},
            "timeline_distribution": {"p50_days": 30, "p80_days": 40, "p95_days": 50},
            "cost_breakdown_template": {"materials": 0.50, "labor": 0.50}
        }

        result = calculate_baseline_estimate(ref_class)

        assert result["variance"] == 20000.0  # 70000 - 50000

    def test_timeline_extraction(self):
        """Test that timelines are correctly extracted."""
        ref_class = {
            "name": "Test",
            "cost_distribution": {"p50": 50000.0, "p80": 60000.0, "p95": 70000.0, "currency": "USD"},
            "timeline_distribution": {
                "p50_days": 45,
                "p80_days": 60,
                "p95_days": 90
            },
            "cost_breakdown_template": {"materials": 1.0}
        }

        result = calculate_baseline_estimate(ref_class)

        assert result["p50_timeline_days"] == 45
        assert result["p80_timeline_days"] == 60
        # P95 should not be included in result (only p50 and p80)


class TestAdjustments:
    """Test complexity and risk adjustments."""

    def setup_method(self):
        """Create sample baseline estimate for testing."""
        self.baseline = {
            "p50_cost": 100000.0,
            "p80_cost": 120000.0,
            "variance": 20000.0,
            "p50_timeline_days": 60,
            "p80_timeline_days": 75,
            "cost_breakdown": {
                "materials": 50000.0,
                "labor": 30000.0,
                "overhead": 20000.0
            },
            "reference_class_name": "Test Project"
        }

    def test_no_adjustments(self):
        """Test with standard complexity and low risk (no adjustments)."""
        result = apply_adjustments(self.baseline)

        # Should equal baseline with standard/low defaults
        assert result["complexity"] == "standard"
        assert result["complexity_factor"] == 1.0
        assert result["risk_level"] == "low"
        assert result["risk_factor"] == 1.0

        assert result["adjusted_p50"] == 100000.0
        assert result["adjusted_p80"] == 120000.0
        assert result["adjusted_p50_timeline"] == 60
        assert result["adjusted_p80_timeline"] == 75

    def test_complexity_simple(self):
        """Test simple complexity (0.8x multiplier)."""
        result = apply_adjustments(self.baseline, complexity="simple")

        assert result["complexity_factor"] == 0.8
        assert result["adjusted_p50"] == 80000.0  # 100000 * 0.8
        assert result["adjusted_p80"] == 96000.0  # 120000 * 0.8
        assert result["adjusted_p50_timeline"] == 48  # 60 * 0.8
        assert result["adjusted_p80_timeline"] == 60  # 75 * 0.8

    def test_complexity_complex(self):
        """Test complex complexity (1.5x multiplier)."""
        result = apply_adjustments(self.baseline, complexity="complex")

        assert result["complexity_factor"] == 1.5
        assert result["adjusted_p50"] == 150000.0  # 100000 * 1.5
        assert result["adjusted_p80"] == 180000.0  # 120000 * 1.5
        assert result["adjusted_p50_timeline"] == 90  # 60 * 1.5
        assert result["adjusted_p80_timeline"] == 112  # 75 * 1.5 = 112.5 rounded

    def test_risk_medium(self):
        """Test medium risk (1.15x multiplier)."""
        result = apply_adjustments(self.baseline, risk_level="medium")

        assert result["risk_factor"] == 1.15
        assert result["adjusted_p50"] == 115000.0  # 100000 * 1.15
        assert result["adjusted_p80"] == 138000.0  # 120000 * 1.15
        # Timeline should NOT be affected by risk
        assert result["adjusted_p50_timeline"] == 60
        assert result["adjusted_p80_timeline"] == 75

    def test_risk_high(self):
        """Test high risk (1.3x multiplier)."""
        result = apply_adjustments(self.baseline, risk_level="high")

        assert result["risk_factor"] == 1.3
        assert result["adjusted_p50"] == 130000.0  # 100000 * 1.3
        assert result["adjusted_p80"] == 156000.0  # 120000 * 1.3

    def test_combined_adjustments(self):
        """Test combined complexity and risk adjustments."""
        result = apply_adjustments(self.baseline, complexity="complex", risk_level="high")

        # Should multiply both factors: 100000 * 1.5 * 1.3
        assert result["complexity_factor"] == 1.5
        assert result["risk_factor"] == 1.3
        assert result["adjusted_p50"] == 195000.0  # 100000 * 1.5 * 1.3
        assert result["adjusted_p80"] == 234000.0  # 120000 * 1.5 * 1.3

        # Timeline only affected by complexity
        assert result["adjusted_p50_timeline"] == 90  # 60 * 1.5

    def test_cost_breakdown_adjustment(self):
        """Test that cost breakdown is adjusted proportionally."""
        result = apply_adjustments(self.baseline, complexity="complex", risk_level="high")

        breakdown = result["cost_breakdown"]

        # Each category should be multiplied by complexity × risk (1.5 × 1.3 = 1.95)
        assert abs(breakdown["materials"] - 50000.0 * 1.95) <= 1.0
        assert abs(breakdown["labor"] - 30000.0 * 1.95) <= 1.0
        assert abs(breakdown["overhead"] - 20000.0 * 1.95) <= 1.0

        # Total should sum to adjusted P50
        total = sum(breakdown.values())
        assert abs(total - 195000.0) <= 0.01

    def test_variance_recalculation(self):
        """Test that variance is recalculated after adjustments."""
        result = apply_adjustments(self.baseline, complexity="complex")

        # Original variance: 20000 (120000 - 100000)
        # Adjusted variance: 30000 (180000 - 150000)
        expected_variance = (120000.0 * 1.5) - (100000.0 * 1.5)
        assert result["adjusted_variance"] == expected_variance

    def test_case_insensitive_inputs(self):
        """Test that complexity and risk inputs are case-insensitive."""
        result1 = apply_adjustments(self.baseline, complexity="COMPLEX", risk_level="HIGH")
        result2 = apply_adjustments(self.baseline, complexity="Complex", risk_level="High")
        result3 = apply_adjustments(self.baseline, complexity="complex", risk_level="high")

        assert result1["adjusted_p50"] == result2["adjusted_p50"] == result3["adjusted_p50"]

    def test_invalid_complexity_defaults_to_standard(self):
        """Test that invalid complexity defaults to standard (1.0)."""
        result = apply_adjustments(self.baseline, complexity="invalid")

        assert result["complexity_factor"] == 1.0

    def test_invalid_risk_defaults_to_low(self):
        """Test that invalid risk defaults to low (1.0)."""
        result = apply_adjustments(self.baseline, risk_level="invalid")

        assert result["risk_factor"] == 1.0

    def test_baseline_values_preserved(self):
        """Test that baseline values are preserved in response."""
        result = apply_adjustments(self.baseline, complexity="complex", risk_level="high")

        assert result["baseline_p50"] == 100000.0
        assert result["baseline_p80"] == 120000.0
        assert result["baseline_p50_timeline"] == 60
        assert result["baseline_p80_timeline"] == 75

    def test_adjustment_summary(self):
        """Test that adjustment summary is generated correctly."""
        result1 = apply_adjustments(self.baseline)
        assert "No adjustments" in result1["adjustment_summary"]

        result2 = apply_adjustments(self.baseline, complexity="complex")
        assert "complexity=complex" in result2["adjustment_summary"]

        result3 = apply_adjustments(self.baseline, risk_level="high")
        assert "risk=high" in result3["adjustment_summary"]

        result4 = apply_adjustments(self.baseline, complexity="complex", risk_level="high")
        assert "complexity=complex" in result4["adjustment_summary"]
        assert "risk=high" in result4["adjustment_summary"]
