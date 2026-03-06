"""
CalibrationService — Tenant-scoped accuracy metrics for the calibration dashboard.

Computes mean variance, accuracy buckets (exclusive slices), per-reference-class
breakdown, and monthly trend data from FeedbackDocuments joined with ReferenceClasses.

CALB-01: Synthetic reference classes (data_source="synthetic") are excluded from
         every $lookup inner pipeline filter.
CALB-02: Returns mean_variance_pct, accuracy_buckets, by_reference_class breakdown,
         and get_trend() monthly time-series with period, mean_variance_pct, outcome_count.
CALB-03: Both get_metrics() and get_trend() enforce a minimum of 10 real outcomes.
CALB-04: Every $lookup uses let/pipeline syntax with explicit tenant_id in inner $match
         — TenantAwareCollection only scopes the source collection.
"""

import logging
from datetime import datetime, timedelta, timezone
from typing import Optional

from app.core.constants import DB_COLLECTIONS
from app.db.mongodb import get_tenant_collection

logger = logging.getLogger(__name__)

CALIBRATION_THRESHOLD = 10  # CALB-03: Minimum real outcomes before metrics display


# ---------------------------------------------------------------------------
# Module-level helpers (independently testable)
# ---------------------------------------------------------------------------


def _compute_variance(actual_cost: float, estimated_p50: float) -> float:
    """Compute percentage variance between actual and estimated cost.

    Returns abs(actual_cost - estimated_p50) / actual_cost * 100.
    Handles zero actual_cost gracefully by returning 0.0.
    """
    if actual_cost == 0:
        return 0.0
    return abs(actual_cost - estimated_p50) / actual_cost * 100


def _compute_accuracy_buckets(variances: list[float]) -> dict:
    """Compute exclusive accuracy bucket proportions from a list of variance percentages.

    Buckets (exclusive slices):
    - within_10_pct:  variance in [0, 10]
    - within_20_pct:  variance in (10, 20]
    - within_30_pct:  variance in (20, 30]
    - beyond_30_pct:  variance > 30

    Returns proportions (0.0 – 1.0). Returns all zeros if variances list is empty.
    """
    if not variances:
        return {
            "within_10_pct": 0.0,
            "within_20_pct": 0.0,
            "within_30_pct": 0.0,
            "beyond_30_pct": 0.0,
        }

    total = len(variances)
    within_10 = sum(1 for v in variances if v <= 10)
    within_20 = sum(1 for v in variances if 10 < v <= 20)
    within_30 = sum(1 for v in variances if 20 < v <= 30)
    beyond_30 = sum(1 for v in variances if v > 30)

    return {
        "within_10_pct": within_10 / total,
        "within_20_pct": within_20 / total,
        "within_30_pct": within_30 / total,
        "beyond_30_pct": beyond_30 / total,
    }


# ---------------------------------------------------------------------------
# CalibrationService
# ---------------------------------------------------------------------------


class CalibrationService:
    """Service for computing calibration accuracy metrics and trend data."""

    def _build_pipeline(self, date_filter: dict, tenant_id: str) -> list[dict]:
        """Build the full metrics aggregation pipeline.

        CALB-04: The $lookup stage uses let/pipeline syntax to explicitly filter
        tenant_id in the inner pipeline. TenantAwareCollection only prepends a
        $match to the source (feedback) collection — it does NOT scope $lookup
        inner pipelines automatically.

        Returns:
            A list of MongoDB aggregation pipeline stages.
        """
        pipeline: list[dict] = []

        # Stage 1: Apply date filter (if any)
        if date_filter:
            pipeline.append({"$match": date_filter})

        # Stage 2: $lookup reference_classes using let/pipeline (CALB-04)
        # Excludes synthetic docs (data_source="synthetic") via CALB-01 tag
        pipeline.append(
            {
                "$lookup": {
                    "from": DB_COLLECTIONS["REFERENCE_CLASSES"],
                    "let": {
                        "rc_id": "$reference_class_id",
                        "tenant": tenant_id,
                    },
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$and": [
                                        {"$eq": ["$name", "$$rc_id"]},
                                        {
                                            "$or": [
                                                {"$eq": ["$tenant_id", "$$tenant"]},
                                                {"$eq": ["$tenant_id", None]},
                                            ]
                                        },
                                        # CALB-01: exclude synthetic reference classes
                                        {"$ne": ["$data_source", "synthetic"]},
                                    ]
                                }
                            }
                        }
                    ],
                    "as": "reference_class_doc",
                }
            }
        )

        # Stage 3: Unwind — preserveNullAndEmptyArrays keeps feedback without matching RC
        pipeline.append(
            {
                "$unwind": {
                    "path": "$reference_class_doc",
                    "preserveNullAndEmptyArrays": True,
                }
            }
        )

        # Stage 4: Group by reference_class_id and collect variances
        pipeline.append(
            {
                "$group": {
                    "_id": "$reference_class_id",
                    "variances": {
                        "$push": {
                            "$multiply": [
                                {
                                    "$divide": [
                                        {
                                            "$abs": {
                                                "$subtract": [
                                                    "$actual_cost",
                                                    "$estimate_snapshot.total_cost_p50",
                                                ]
                                            }
                                        },
                                        "$actual_cost",
                                    ]
                                },
                                100,
                            ]
                        }
                    },
                    "outcome_count": {"$sum": 1},
                }
            }
        )

        return pipeline

    def _build_trend_pipeline(self, since: datetime) -> list[dict]:
        """Build the monthly trend aggregation pipeline.

        Note: This pipeline does NOT use $lookup — it operates solely on the
        feedback collection. Variance is computed from actual_cost vs
        estimate_snapshot.total_cost_p50. TenantAwareCollection handles
        tenant scoping for the source collection.

        Returns:
            A list of MongoDB aggregation pipeline stages.
        """
        pipeline: list[dict] = []

        # Stage 1: Filter to time window
        pipeline.append(
            {"$match": {"submitted_at": {"$gte": since}}}
        )

        # Stage 2: Project variance_pct and period (YYYY-MM string)
        pipeline.append(
            {
                "$project": {
                    "variance_pct": {
                        "$multiply": [
                            {
                                "$divide": [
                                    {
                                        "$abs": {
                                            "$subtract": [
                                                "$actual_cost",
                                                "$estimate_snapshot.total_cost_p50",
                                            ]
                                        }
                                    },
                                    "$actual_cost",
                                ]
                            },
                            100,
                        ]
                    },
                    "period": {
                        "$dateToString": {
                            "format": "%Y-%m",
                            "date": "$submitted_at",
                        }
                    },
                }
            }
        )

        # Stage 3: Group by period (YYYY-MM), compute mean variance and count
        pipeline.append(
            {
                "$group": {
                    "_id": "$period",
                    "mean_variance_pct": {"$avg": "$variance_pct"},
                    "outcome_count": {"$sum": 1},
                }
            }
        )

        # Stage 4: Sort chronologically (oldest first)
        pipeline.append({"$sort": {"_id": 1}})

        # Stage 5: Project final shape — rename _id to period, round mean_variance_pct
        pipeline.append(
            {
                "$project": {
                    "_id": 0,
                    "period": "$_id",
                    "mean_variance_pct": {"$round": ["$mean_variance_pct", 1]},
                    "outcome_count": 1,
                }
            }
        )

        return pipeline

    def _build_date_filter(self, date_range: Optional[str]) -> dict:
        """Build the date filter dict from a date_range string.

        Args:
            date_range: "6months", "1year", "all", or None (all = no filter)

        Returns:
            Empty dict for "all"/None, or {"submitted_at": {"$gte": cutoff}} otherwise.
        """
        now = datetime.now(timezone.utc)
        if date_range == "6months":
            cutoff = now - timedelta(days=182)
        elif date_range == "1year":
            cutoff = now - timedelta(days=365)
        else:
            return {}
        return {"submitted_at": {"$gte": cutoff}}

    async def get_metrics(
        self, tenant_id: str, date_range: Optional[str] = None
    ) -> dict:
        """Compute calibration accuracy metrics for the given tenant.

        CALB-03: Returns below_threshold response when fewer than CALIBRATION_THRESHOLD
        real outcomes exist.

        Args:
            tenant_id: The authenticated tenant's ID.
            date_range: "6months", "1year", or "all"/None for all time.

        Returns:
            dict with below_threshold=True when < 10 outcomes, or full metrics dict.
        """
        feedback_col = get_tenant_collection(
            DB_COLLECTIONS["FEEDBACK"], tenant_id
        )

        # Build date filter and count real outcomes first (CALB-03)
        date_filter = self._build_date_filter(date_range)
        count = await feedback_col.count_documents(date_filter)

        if count < CALIBRATION_THRESHOLD:
            return {
                "below_threshold": True,
                "outcome_count": count,
                "threshold": CALIBRATION_THRESHOLD,
            }

        # Build and execute aggregation pipeline
        pipeline = self._build_pipeline(date_filter, tenant_id)
        cursor = await feedback_col.aggregate(pipeline)
        results = await cursor.to_list(None)

        # Flatten all variances for overall metrics
        all_variances: list[float] = []
        by_reference_class: list[dict] = []

        for group in results:
            variances = [v for v in group.get("variances", []) if v is not None]
            all_variances.extend(variances)
            rc_name = group["_id"] or "Unknown"
            rc_count = group.get("outcome_count", len(variances))
            mean_var = round(sum(variances) / len(variances), 1) if variances else 0.0

            by_reference_class.append(
                {
                    "reference_class": rc_name,
                    "outcome_count": rc_count,
                    "mean_variance_pct": mean_var,
                    "accuracy_buckets": _compute_accuracy_buckets(variances),
                    "limited_data": rc_count < 5,
                }
            )

        overall_mean = (
            round(sum(all_variances) / len(all_variances), 1)
            if all_variances
            else 0.0
        )

        return {
            "below_threshold": False,
            "outcome_count": count,
            "threshold": CALIBRATION_THRESHOLD,
            "mean_variance_pct": overall_mean,
            "accuracy_buckets": _compute_accuracy_buckets(all_variances),
            "by_reference_class": by_reference_class,
            "date_range": date_range or "all",
        }

    async def get_trend(self, tenant_id: str, months: int = 12) -> dict:
        """Compute monthly accuracy trend for the given tenant.

        CALB-03: Returns below_threshold response when fewer than CALIBRATION_THRESHOLD
        real outcomes exist (regardless of the months window).

        Args:
            tenant_id: The authenticated tenant's ID.
            months: Number of months to look back (1–36).

        Returns:
            dict with below_threshold=True when < 10 outcomes, or trend time-series.
        """
        feedback_col = get_tenant_collection(
            DB_COLLECTIONS["FEEDBACK"], tenant_id
        )

        # Threshold check (CALB-03) — count ALL outcomes, not just in window
        count = await feedback_col.count_documents({})

        if count < CALIBRATION_THRESHOLD:
            return {
                "below_threshold": True,
                "outcome_count": count,
                "threshold": CALIBRATION_THRESHOLD,
                "trend": [],
            }

        # Build trend pipeline and execute
        since = datetime.now(timezone.utc) - timedelta(days=months * 30)
        pipeline = self._build_trend_pipeline(since)
        cursor = await feedback_col.aggregate(pipeline)
        trend_results = await cursor.to_list(None)

        # Ensure Python float types (MongoDB may return Decimal128 or int)
        trend = [
            {
                "period": doc["period"],
                "mean_variance_pct": float(doc["mean_variance_pct"]),
                "outcome_count": int(doc["outcome_count"]),
            }
            for doc in trend_results
        ]

        return {
            "below_threshold": False,
            "outcome_count": count,
            "threshold": CALIBRATION_THRESHOLD,
            "trend": trend,
            "months": months,
        }
