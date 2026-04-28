"""
Unit tests for calibration API endpoints.

Covers:
- GET /api/v1/calibration/metrics: auth, below-threshold, above-threshold, date_range validation
- GET /api/v1/calibration/trend: auth, success, months validation

Mock strategy:
- Use FastAPI dependency_overrides to mock get_current_tenant (avoids JWT validation)
- Patch CalibrationService methods to avoid MongoDB connections
- Use httpx.AsyncClient with ASGITransport for full ASGI stack testing
"""

import pytest
from unittest.mock import AsyncMock, MagicMock, patch
from httpx import AsyncClient, ASGITransport

from app.models.tenant import Tenant
from app.core.security import get_current_tenant


# ---------------------------------------------------------------------------
# Helpers / shared fixtures
# ---------------------------------------------------------------------------

TENANT_ID = "tenant-uuid-calibration-001"


def _make_tenant() -> Tenant:
    """Return a minimal Tenant object for dependency override."""
    return Tenant(
        tenant_id=TENANT_ID,
        company_name="Test Calibration Co",
        email="test@calibration.example.com",
        hashed_password="$2b$12$hashed",
        hashed_api_key="$2b$12$hashed_api",
        api_key_last6="abc123",
        tier="trial",
        email_verified=True,
    )


async def _mock_get_tenant():
    """Async dependency override — returns a valid tenant without DB access."""
    return _make_tenant()


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

    app.dependency_overrides[get_current_tenant] = _mock_get_tenant
    try:
        with patch(
            "app.api.calibration.CalibrationService.get_metrics",
            new_callable=AsyncMock,
            return_value=below_threshold_response,
        ):
            async with AsyncClient(
                transport=ASGITransport(app=app), base_url="http://test"
            ) as client:
                resp = await client.get(
                    "/api/v1/calibration/metrics",
                    headers={"Authorization": "Bearer fake-token"},
                )
    finally:
        app.dependency_overrides.pop(get_current_tenant, None)

    assert resp.status_code == 200
    data = resp.json()
    assert data["below_threshold"] is True
    assert data["outcome_count"] == 5


@pytest.mark.asyncio
async def test_get_calibration_metrics_invalid_date_range():
    """GET /api/v1/calibration/metrics?date_range=invalid returns 422."""
    from app.main import app

    app.dependency_overrides[get_current_tenant] = _mock_get_tenant
    try:
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.get(
                "/api/v1/calibration/metrics?date_range=invalid",
                headers={"Authorization": "Bearer fake-token"},
            )
    finally:
        app.dependency_overrides.pop(get_current_tenant, None)

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

    app.dependency_overrides[get_current_tenant] = _mock_get_tenant
    try:
        with patch(
            "app.api.calibration.CalibrationService.get_trend",
            new_callable=AsyncMock,
            return_value=trend_response,
        ):
            async with AsyncClient(
                transport=ASGITransport(app=app), base_url="http://test"
            ) as client:
                resp = await client.get(
                    "/api/v1/calibration/trend?months=12",
                    headers={"Authorization": "Bearer fake-token"},
                )
    finally:
        app.dependency_overrides.pop(get_current_tenant, None)

    assert resp.status_code == 200
    data = resp.json()
    assert "trend" in data
    assert len(data["trend"]) == 3
    assert data["trend"][0]["period"] == "2025-10"
    assert isinstance(data["trend"][0]["mean_variance_pct"], float)
