"""
Unit tests for CalibrationService and migrate_synthetic_reference_classes.

Covers:
- migrate_synthetic_reference_classes: tagging synthetic documents, idempotency
- CalibrationService.get_metrics: below-threshold, above-threshold, date range filters
- CalibrationService.get_trend: monthly aggregation, below-threshold, tenant scoping
- CALB-04 security: $lookup inner pipeline explicitly scopes tenant_id
- Helper functions: _compute_accuracy_buckets, _compute_variance
"""

from datetime import datetime, timedelta, timezone
from unittest.mock import AsyncMock, MagicMock, call, patch

import pytest

from app.services.calibration_service import (
    CALIBRATION_THRESHOLD,
    CalibrationService,
    _compute_accuracy_buckets,
    _compute_variance,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_mock_feedback_collection(count: int = 15):
    """Return a mock TenantAwareCollection for feedback."""
    mock_col = MagicMock()
    mock_col.count_documents = AsyncMock(return_value=count)

    # aggregate returns a cursor — TenantAwareCollection.aggregate is async
    mock_cursor = MagicMock()
    mock_cursor.to_list = AsyncMock(return_value=[])
    mock_col.aggregate = AsyncMock(return_value=mock_cursor)
    return mock_col


# ---------------------------------------------------------------------------
# migrate_synthetic_reference_classes
# ---------------------------------------------------------------------------


class TestMigrateSyntheticReferenceClasses:
    """Tests for the CALB-01 migration."""

    @pytest.mark.asyncio
    async def test_migrate_synthetic_reference_classes_tags_documents(self):
        """update_many called with correct filter and update to tag synthetic docs."""
        from app.db.mongodb import migrate_synthetic_reference_classes

        mock_collection = MagicMock()
        mock_collection.update_many = AsyncMock(
            return_value=MagicMock(modified_count=3)
        )
        mock_db = MagicMock()
        mock_db.__getitem__ = MagicMock(return_value=mock_collection)

        with patch("app.db.mongodb.get_database", return_value=mock_db):
            await migrate_synthetic_reference_classes()

        mock_collection.update_many.assert_awaited_once()
        filter_arg, update_arg = mock_collection.update_many.call_args[0]
        assert filter_arg == {
            "is_synthetic": True,
            "data_source": {"$exists": False},
        }
        assert update_arg == {"$set": {"data_source": "synthetic"}}

    @pytest.mark.asyncio
    async def test_migrate_synthetic_reference_classes_idempotent(self):
        """Second run finds 0 documents due to $exists: False guard."""
        from app.db.mongodb import migrate_synthetic_reference_classes

        # First call modifies 3, second call modifies 0 (already tagged)
        mock_collection = MagicMock()
        mock_collection.update_many = AsyncMock(
            side_effect=[
                MagicMock(modified_count=3),
                MagicMock(modified_count=0),
            ]
        )
        mock_db = MagicMock()
        mock_db.__getitem__ = MagicMock(return_value=mock_collection)

        with patch("app.db.mongodb.get_database", return_value=mock_db):
            await migrate_synthetic_reference_classes()
            await migrate_synthetic_reference_classes()

        assert mock_collection.update_many.await_count == 2
        # Both calls use the same $exists: False filter (idempotent guard)
        for call_args in mock_collection.update_many.call_args_list:
            filter_arg = call_args[0][0]
            assert filter_arg["data_source"] == {"$exists": False}


# ---------------------------------------------------------------------------
# CalibrationService.get_metrics — below threshold
# ---------------------------------------------------------------------------


class TestGetMetricsBelowThreshold:
    """CalibrationService.get_metrics when outcome count < CALIBRATION_THRESHOLD."""

    @pytest.mark.asyncio
    async def test_get_metrics_below_threshold(self):
        """Returns below_threshold response when fewer than 10 real outcomes exist."""
        mock_col = _make_mock_feedback_collection(count=5)

        svc = CalibrationService()
        with patch(
            "app.services.calibration_service.get_tenant_collection",
            return_value=mock_col,
        ):
            result = await svc.get_metrics(tenant_id="tenant-abc")

        assert result["below_threshold"] is True
        assert result["outcome_count"] == 5
        assert result["threshold"] == CALIBRATION_THRESHOLD
        # Should NOT contain full metrics when below threshold
        assert "mean_variance_pct" not in result
        assert "accuracy_buckets" not in result


# ---------------------------------------------------------------------------
# CalibrationService.get_metrics — above threshold
# ---------------------------------------------------------------------------


class TestGetMetricsAboveThreshold:
    """CalibrationService.get_metrics when outcome count >= CALIBRATION_THRESHOLD."""

    @pytest.mark.asyncio
    async def test_get_metrics_above_threshold(self):
        """Returns full metrics when 10+ real outcomes exist."""
        # Simulate 12 feedback docs with aggregation results
        mock_col = _make_mock_feedback_collection(count=12)
        mock_col.aggregate = AsyncMock(
            return_value=MagicMock(
                to_list=AsyncMock(
                    return_value=[
                        {
                            "_id": "residential_pool",
                            "variances": [10.0, 15.0, 5.0, 20.0],
                            "outcome_count": 4,
                        },
                        {
                            "_id": "commercial_office",
                            "variances": [8.0, 12.0, 25.0, 30.0],
                            "outcome_count": 4,
                        },
                        {
                            "_id": None,
                            "variances": [22.0, 18.0, 35.0, 40.0],
                            "outcome_count": 4,
                        },
                    ]
                )
            )
        )

        svc = CalibrationService()
        with patch(
            "app.services.calibration_service.get_tenant_collection",
            return_value=mock_col,
        ):
            result = await svc.get_metrics(tenant_id="tenant-abc")

        assert result["below_threshold"] is False
        assert result["outcome_count"] == 12
        assert result["threshold"] == CALIBRATION_THRESHOLD
        assert isinstance(result["mean_variance_pct"], float)
        assert "accuracy_buckets" in result
        buckets = result["accuracy_buckets"]
        assert set(buckets.keys()) == {
            "within_10_pct",
            "within_20_pct",
            "within_30_pct",
            "beyond_30_pct",
        }
        total = sum(buckets.values())
        assert abs(total - 1.0) < 0.01, f"Buckets should sum to ~1.0, got {total}"
        assert isinstance(result["by_reference_class"], list)
        assert "date_range" in result

    @pytest.mark.asyncio
    async def test_get_metrics_date_range_6months(self):
        """Pipeline includes submitted_at $gte filter when date_range='6months'."""
        mock_col = _make_mock_feedback_collection(count=12)

        svc = CalibrationService()
        with patch(
            "app.services.calibration_service.get_tenant_collection",
            return_value=mock_col,
        ):
            await svc.get_metrics(tenant_id="tenant-abc", date_range="6months")

        # Verify count_documents was called with a date filter
        count_call_filter = mock_col.count_documents.call_args[0][0]
        assert "submitted_at" in count_call_filter
        assert "$gte" in count_call_filter["submitted_at"]

    @pytest.mark.asyncio
    async def test_get_metrics_date_range_1year(self):
        """Pipeline includes submitted_at $gte filter when date_range='1year'."""
        mock_col = _make_mock_feedback_collection(count=12)

        svc = CalibrationService()
        with patch(
            "app.services.calibration_service.get_tenant_collection",
            return_value=mock_col,
        ):
            await svc.get_metrics(tenant_id="tenant-abc", date_range="1year")

        count_call_filter = mock_col.count_documents.call_args[0][0]
        assert "submitted_at" in count_call_filter
        assert "$gte" in count_call_filter["submitted_at"]


# ---------------------------------------------------------------------------
# CALB-04: $lookup tenant isolation
# ---------------------------------------------------------------------------


class TestLookupTenantIsolation:
    """CALB-04: Every $lookup in the pipeline explicitly filters tenant_id."""

    def test_lookup_pipeline_includes_tenant_id(self):
        """_build_pipeline $lookup uses let/pipeline with tenant_id in inner $match."""
        svc = CalibrationService()
        pipeline = svc._build_pipeline(date_filter={}, tenant_id="tenant-xyz")

        # Find the $lookup stage
        lookup_stage = None
        for stage in pipeline:
            if "$lookup" in stage:
                lookup_stage = stage["$lookup"]
                break

        assert lookup_stage is not None, "Pipeline must contain a $lookup stage"
        assert "let" in lookup_stage, "$lookup must use let/pipeline syntax"
        assert "pipeline" in lookup_stage, "$lookup must have inner pipeline"

        # Check that 'tenant' variable is defined in let
        assert "tenant" in lookup_stage["let"], "let must define 'tenant' variable"

        # Find the inner $match stage with tenant_id check
        inner_pipeline = lookup_stage["pipeline"]
        inner_match = None
        for stage in inner_pipeline:
            if "$match" in stage:
                inner_match = stage["$match"]
                break

        assert inner_match is not None, "Inner pipeline must have $match"
        # The inner match must include a $expr check that references $$tenant
        match_str = str(inner_match)
        assert "$$tenant" in match_str, (
            "Inner $match must reference $$tenant for CALB-04 isolation"
        )


# ---------------------------------------------------------------------------
# Helper functions
# ---------------------------------------------------------------------------


class TestComputeAccuracyBuckets:
    """Tests for _compute_accuracy_buckets helper."""

    def test_accuracy_buckets_exclusive_slices(self):
        """_compute_accuracy_buckets([5, 15, 25, 35]) returns exclusive slices summing to 1.0."""
        buckets = _compute_accuracy_buckets([5.0, 15.0, 25.0, 35.0])
        assert buckets["within_10_pct"] == 0.25   # 5% — in [0,10]
        assert buckets["within_20_pct"] == 0.25   # 15% — in (10,20]
        assert buckets["within_30_pct"] == 0.25   # 25% — in (20,30]
        assert buckets["beyond_30_pct"] == 0.25   # 35% — >30
        total = sum(buckets.values())
        assert abs(total - 1.0) < 0.001

    def test_accuracy_buckets_all_within_10(self):
        """All variances <= 10 → within_10_pct = 1.0."""
        buckets = _compute_accuracy_buckets([2.0, 5.0, 8.0, 10.0])
        assert buckets["within_10_pct"] == 1.0
        assert buckets["within_20_pct"] == 0.0
        assert buckets["within_30_pct"] == 0.0
        assert buckets["beyond_30_pct"] == 0.0

    def test_accuracy_buckets_empty_list(self):
        """Empty variance list returns zero buckets."""
        buckets = _compute_accuracy_buckets([])
        assert all(v == 0.0 for v in buckets.values())


class TestComputeVariance:
    """Tests for _compute_variance helper."""

    def test_compute_variance_basic(self):
        """abs(actual - estimated) / actual * 100."""
        result = _compute_variance(actual_cost=100.0, estimated_p50=120.0)
        assert result == pytest.approx(20.0)

    def test_compute_variance_underestimate(self):
        """Underestimate also gives positive variance."""
        result = _compute_variance(actual_cost=100.0, estimated_p50=80.0)
        assert result == pytest.approx(20.0)

    def test_compute_variance_exact_match(self):
        """Zero variance when estimate equals actual."""
        result = _compute_variance(actual_cost=100.0, estimated_p50=100.0)
        assert result == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# Reference class breakdown — limited data flag
# ---------------------------------------------------------------------------


class TestReferenceClassBreakdown:
    """Tests for by_reference_class breakdown with limited_data flag."""

    @pytest.mark.asyncio
    async def test_reference_class_breakdown_limited_data(self):
        """Reference classes with fewer than 5 outcomes have limited_data: True."""
        mock_col = _make_mock_feedback_collection(count=12)
        mock_col.aggregate = AsyncMock(
            return_value=MagicMock(
                to_list=AsyncMock(
                    return_value=[
                        {
                            "_id": "residential_pool",
                            "variances": [10.0, 15.0, 5.0],  # 3 outcomes — limited
                            "outcome_count": 3,
                        },
                        {
                            "_id": "commercial_office",
                            "variances": [8.0, 12.0, 25.0, 30.0, 18.0],  # 5 outcomes — not limited
                            "outcome_count": 5,
                        },
                    ]
                )
            )
        )

        svc = CalibrationService()
        with patch(
            "app.services.calibration_service.get_tenant_collection",
            return_value=mock_col,
        ):
            result = await svc.get_metrics(tenant_id="tenant-abc")

        by_rc = {r["reference_class"]: r for r in result["by_reference_class"]}
        assert by_rc["residential_pool"]["limited_data"] is True
        assert by_rc["commercial_office"]["limited_data"] is False


# ---------------------------------------------------------------------------
# CalibrationService.get_trend
# ---------------------------------------------------------------------------


class TestGetTrend:
    """Tests for CalibrationService.get_trend monthly aggregation."""

    @pytest.mark.asyncio
    async def test_get_trend_returns_monthly_aggregation(self):
        """get_trend returns monthly time-series sorted chronologically."""
        mock_col = _make_mock_feedback_collection(count=12)
        # Simulate aggregate returning 3 monthly buckets
        mock_col.aggregate = AsyncMock(
            return_value=MagicMock(
                to_list=AsyncMock(
                    return_value=[
                        {"period": "2026-01", "mean_variance_pct": 18.3, "outcome_count": 4},
                        {"period": "2026-02", "mean_variance_pct": 14.1, "outcome_count": 5},
                        {"period": "2026-03", "mean_variance_pct": 11.7, "outcome_count": 3},
                    ]
                )
            )
        )

        svc = CalibrationService()
        with patch(
            "app.services.calibration_service.get_tenant_collection",
            return_value=mock_col,
        ):
            result = await svc.get_trend(tenant_id="tenant-abc", months=6)

        assert result["below_threshold"] is False
        assert "trend" in result
        trend = result["trend"]
        assert len(trend) == 3
        for point in trend:
            assert "period" in point
            assert "mean_variance_pct" in point
            assert "outcome_count" in point
            assert isinstance(point["mean_variance_pct"], float)
            assert isinstance(point["outcome_count"], int)
        # Assert chronologically sorted (oldest first)
        periods = [p["period"] for p in trend]
        assert periods == sorted(periods)

    @pytest.mark.asyncio
    async def test_get_trend_below_threshold_returns_empty(self):
        """get_trend returns below_threshold response when fewer than 10 outcomes."""
        mock_col = _make_mock_feedback_collection(count=5)

        svc = CalibrationService()
        with patch(
            "app.services.calibration_service.get_tenant_collection",
            return_value=mock_col,
        ):
            result = await svc.get_trend(tenant_id="tenant-abc", months=6)

        assert result["below_threshold"] is True
        assert result["outcome_count"] == 5
        assert result["threshold"] == CALIBRATION_THRESHOLD
        assert result["trend"] == []

    @pytest.mark.asyncio
    async def test_get_trend_uses_tenant_scoped_collection(self):
        """get_trend calls get_tenant_collection with the correct tenant_id (CALB-04)."""
        mock_col = _make_mock_feedback_collection(count=5)

        svc = CalibrationService()
        with patch(
            "app.services.calibration_service.get_tenant_collection",
            return_value=mock_col,
        ) as mock_get_col:
            await svc.get_trend(tenant_id="my-special-tenant", months=6)

        mock_get_col.assert_called_once()
        call_args = mock_get_col.call_args
        # Verify correct tenant_id was passed
        assert "my-special-tenant" in call_args[0] or call_args[1].get("tenant_id") == "my-special-tenant"

    def test_build_trend_pipeline_structure(self):
        """_build_trend_pipeline returns pipeline with $match, $project, $group, $sort, $project."""
        from datetime import datetime, timezone

        since = datetime(2026, 1, 1, tzinfo=timezone.utc)
        svc = CalibrationService()
        pipeline = svc._build_trend_pipeline(since=since)

        stage_keys = [list(stage.keys())[0] for stage in pipeline]
        assert "$match" in stage_keys
        assert "$project" in stage_keys
        assert "$group" in stage_keys
        assert "$sort" in stage_keys
