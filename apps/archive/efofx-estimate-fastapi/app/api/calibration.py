"""
Calibration dashboard API endpoints.

GET /calibration/metrics — Accuracy metrics for the authenticated tenant.
GET /calibration/trend   — Monthly accuracy trend time-series.

Both endpoints require JWT or API key authentication via get_current_tenant.
Both endpoints enforce the 10-outcome minimum threshold (CALB-03).
"""

from fastapi import APIRouter, Depends, Query

from app.core.security import get_current_tenant
from app.models.tenant import Tenant
from app.services.calibration_service import CalibrationService

calibration_router = APIRouter(prefix="/calibration", tags=["calibration"])


@calibration_router.get("/metrics")
async def get_calibration_metrics(
    tenant: Tenant = Depends(get_current_tenant),
    date_range: str = Query(
        default="all",
        pattern="^(6months|1year|all)$",
        description="Date range filter: '6months', '1year', or 'all'",
    ),
) -> dict:
    """Return calibration accuracy metrics for the authenticated tenant.

    Returns a below_threshold response when fewer than 10 real outcomes exist.
    Otherwise returns mean_variance_pct, accuracy_buckets, and by_reference_class
    breakdown.
    """
    svc = CalibrationService()
    return await svc.get_metrics(tenant.tenant_id, date_range)


@calibration_router.get("/trend")
async def get_calibration_trend(
    tenant: Tenant = Depends(get_current_tenant),
    months: int = Query(
        default=12,
        ge=1,
        le=36,
        description="Number of months to look back (1–36)",
    ),
) -> dict:
    """Return monthly accuracy trend time-series for the authenticated tenant.

    Returns a below_threshold response when fewer than 10 real outcomes exist.
    Otherwise returns a sorted list of monthly data points with period (YYYY-MM),
    mean_variance_pct, and outcome_count.
    """
    svc = CalibrationService()
    return await svc.get_trend(tenant.tenant_id, months)
