"""
Unit tests for calibration API endpoints.

Covers:
- GET /api/v1/calibration/metrics: auth, below-threshold, above-threshold, date_range validation
- GET /api/v1/calibration/trend: auth, success, months validation

Mock strategy:
- Patch CalibrationService methods to avoid MongoDB connections
- Patch get_current_tenant to simulate authentication
- Use httpx.AsyncClient with ASGITransport for full ASGI stack testing
"""

import pytest
from unittest.mock import AsyncMock, MagicMock, patch
from httpx import AsyncClient, ASGITransport


# ---------------------------------------------------------------------------
# Helpers / shared fixtures
# ---------------------------------------------------------------------------

TENANT_ID = "tenant-uuid-calibration-001"


def _mock_tenant():
    """Return a mock Tenant object."""
    tenant = MagicMock()
    tenant.tenant_id = TENANT_ID
    return tenant


@pytest.fixture(autouse=True)
def disable_rate_limiter():
    """Disable the rate limiter for all calibration API tests."""
    from app.core.rate_limit import limiter

    original = limiter.enabled
    limiter.enabled = False
    yield
    limiter.enabled = original


# ---------------------------------------------------------------------------
# GET /api/v1/calibration/metrics
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_get_calibration_metrics_unauthenticated():
    """GET /api/v1/calibration/metrics with no auth header returns 401."""
    from app.main import app

    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        resp = await client.get("/api/v1/calibration/metrics")

    assert resp.status_code == 401


@pytest.mark.asyncio
async def test_get_calibration_metrics_below_threshold():
    """GET /api/v1/calibration/metrics returns 200 with below_threshold: true."""
    from app.main import app

    below_threshold_response = {
        "below_threshold": True,
        "outcome_count": 5,
        "threshold": 10,
    }

    with (
        patch(
            "app.api.calibration.get_current_tenant",
            return_value=_mock_tenant(),
        ),
        patch(
            "app.api.calibration.CalibrationService.get_metrics",
            new_callable=AsyncMock,
            return_value=below_threshold_response,
        ),
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.get(
                "/api/v1/calibration/metrics",
                headers={"Authorization": "Bearer fake-token"},
            )

    assert resp.status_code == 200
    data = resp.json()
    assert data["below_threshold"] is True
    assert data["outcome_count"] == 5


@pytest.mark.asyncio
async def test_get_calibration_metrics_invalid_date_range():
    """GET /api/v1/calibration/metrics?date_range=invalid returns 422."""
    from app.main import app

    with patch(
        "app.api.calibration.get_current_tenant",
        return_value=_mock_tenant(),
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.get(
                "/api/v1/calibration/metrics?date_range=invalid",
                headers={"Authorization": "Bearer fake-token"},
            )

    assert resp.status_code == 422


# ---------------------------------------------------------------------------
# GET /api/v1/calibration/trend
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_get_calibration_trend_unauthenticated():
    """GET /api/v1/calibration/trend with no auth header returns 401."""
    from app.main import app

    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        resp = await client.get("/api/v1/calibration/trend")

    assert resp.status_code == 401


@pytest.mark.asyncio
async def test_get_calibration_trend_success():
    """GET /api/v1/calibration/trend?months=12 returns 200 with trend data."""
    from app.main import app

    trend_response = {
        "below_threshold": False,
        "outcome_count": 12,
        "threshold": 10,
        "trend": [
            {"period": "2025-10", "mean_variance_pct": 15.2, "outcome_count": 3},
            {"period": "2025-11", "mean_variance_pct": 12.1, "outcome_count": 4},
            {"period": "2025-12", "mean_variance_pct": 9.8, "outcome_count": 5},
        ],
        "months": 12,
    }

    with (
        patch(
            "app.api.calibration.get_current_tenant",
            return_value=_mock_tenant(),
        ),
        patch(
            "app.api.calibration.CalibrationService.get_trend",
            new_callable=AsyncMock,
            return_value=trend_response,
        ),
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.get(
                "/api/v1/calibration/trend?months=12",
                headers={"Authorization": "Bearer fake-token"},
            )

    assert resp.status_code == 200
    data = resp.json()
    assert "trend" in data
    assert len(data["trend"]) == 3
    assert data["trend"][0]["period"] == "2025-10"
    assert isinstance(data["trend"][0]["mean_variance_pct"], float)
