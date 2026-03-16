"""
Tests for widget API endpoints: branding config, lead capture, CORS behavior, security.

Tests:
- GET /api/v1/widget/branding/{prefix} returns 200 with branding fields
- GET /api/v1/widget/branding/{prefix} returns 404 for unknown prefix
- Branding defaults are used when tenant has no branding settings
- Custom branding values from tenant settings are returned
- Branding response never exposes sensitive fields
- Branding endpoint works without auth header (public)
- company_name from tenant appears in branding response
- Lead capture endpoint requires auth (WSEC-02)
- Analytics endpoint requires auth (WSEC-02)
- Branding endpoint does NOT require auth (BRND-04)
- Lead capture saves lead to DB
- CORS allows static ALLOWED_ORIGINS
- CORS allows tenant-registered origins via cache

Mock strategy:
- Mock widget_service functions (get_branding_by_prefix, save_lead) to isolate DB
- Mock get_current_tenant dependency for auth-required endpoints
- Use FastAPI dependency_overrides for clean isolation
- Disable rate limiter for all tests (Valkey not available in test environment)
"""

import pytest
from unittest.mock import AsyncMock, MagicMock, patch
from httpx import AsyncClient, ASGITransport

from app.models.widget import BrandingConfigResponse, LeadCaptureResponse
from app.models.tenant import Tenant


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_tenant(tenant_id: str = "11111111-1111-1111-1111-111111111111") -> Tenant:
    """Create a minimal Tenant for dependency override."""
    return Tenant(
        tenant_id=tenant_id,
        company_name="Test Contractors LLC",
        email="test@example.com",
        hashed_password="$2b$12$hashed",
        hashed_api_key="$2b$12$hashed_api",
        api_key_last6="abc123",
        tier="trial",
        email_verified=True,
    )


def make_branding_response(
    company_name: str = "Test Contractors LLC",
    primary_color: str = "#2563eb",
    secondary_color: str = "#f3f4f6",
    accent_color: str = "#1d4ed8",
    logo_url: str | None = None,
    welcome_message: str = "Hi! Tell me about your project and I'll help estimate the cost.",
    button_text: str = "Get an Estimate",
) -> BrandingConfigResponse:
    """Create a BrandingConfigResponse for mocking."""
    return BrandingConfigResponse(
        primary_color=primary_color,
        secondary_color=secondary_color,
        accent_color=accent_color,
        logo_url=logo_url,
        welcome_message=welcome_message,
        button_text=button_text,
        company_name=company_name,
    )


VALID_PREFIX = "1" * 32  # 32-char hex prefix (simplified for tests)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture(autouse=True)
def disable_rate_limiter():
    """Disable the rate limiter for all widget tests.

    The rate limiter requires a live Valkey connection in production.
    Disable it globally for tests to avoid connection errors.
    """
    from app.core.rate_limit import limiter

    original = limiter.enabled
    limiter.enabled = False
    yield
    limiter.enabled = original


# ---------------------------------------------------------------------------
# Branding endpoint tests
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_branding_returns_200_with_valid_prefix():
    """GET /branding/{prefix} returns 200 with all 7 branding fields."""
    from app.main import app

    branding = make_branding_response()

    with patch(
        "app.api.widget.get_branding_by_prefix",
        new_callable=AsyncMock,
        return_value=branding,
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.get(f"/api/v1/widget/branding/{VALID_PREFIX}")

    assert resp.status_code == 200
    data = resp.json()
    # Verify all 7 required branding fields are present
    assert "primary_color" in data
    assert "secondary_color" in data
    assert "accent_color" in data
    assert "logo_url" in data
    assert "welcome_message" in data
    assert "button_text" in data
    assert "company_name" in data


@pytest.mark.asyncio
async def test_branding_returns_404_for_unknown_prefix():
    """GET /branding/{prefix} returns 404 when tenant not found."""
    from app.main import app

    with patch(
        "app.api.widget.get_branding_by_prefix",
        new_callable=AsyncMock,
        return_value=None,
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.get(f"/api/v1/widget/branding/{VALID_PREFIX}")

    assert resp.status_code == 404
    assert "not found" in resp.json()["detail"].lower()


@pytest.mark.asyncio
async def test_branding_uses_defaults_when_no_settings():
    """Branding response uses defaults when tenant has no branding in settings."""
    from app.main import app

    # Default values from BrandingConfig
    default_branding = make_branding_response(
        primary_color="#2563eb",
        secondary_color="#f3f4f6",
        accent_color="#1d4ed8",
        welcome_message="Hi! Tell me about your project and I'll help estimate the cost.",
        button_text="Get an Estimate",
    )

    with patch(
        "app.api.widget.get_branding_by_prefix",
        new_callable=AsyncMock,
        return_value=default_branding,
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.get(f"/api/v1/widget/branding/{VALID_PREFIX}")

    assert resp.status_code == 200
    data = resp.json()
    assert data["primary_color"] == "#2563eb"
    assert data["secondary_color"] == "#f3f4f6"
    assert data["button_text"] == "Get an Estimate"


@pytest.mark.asyncio
async def test_branding_uses_custom_values_from_settings():
    """Branding response uses custom values when tenant has configured branding."""
    from app.main import app

    custom_branding = make_branding_response(
        primary_color="#FF5733",
        secondary_color="#C70039",
        accent_color="#900C3F",
        logo_url="https://example.com/logo.png",
        welcome_message="Welcome to ACME Contracting!",
        button_text="Request My Free Estimate",
        company_name="ACME Contracting",
    )

    with patch(
        "app.api.widget.get_branding_by_prefix",
        new_callable=AsyncMock,
        return_value=custom_branding,
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.get(f"/api/v1/widget/branding/{VALID_PREFIX}")

    assert resp.status_code == 200
    data = resp.json()
    assert data["primary_color"] == "#FF5733"
    assert data["logo_url"] == "https://example.com/logo.png"
    assert data["welcome_message"] == "Welcome to ACME Contracting!"
    assert data["button_text"] == "Request My Free Estimate"
    assert data["company_name"] == "ACME Contracting"


@pytest.mark.asyncio
async def test_branding_never_exposes_sensitive_fields():
    """Branding response must NOT contain sensitive data (keys, passwords, email)."""
    from app.main import app

    branding = make_branding_response()

    with patch(
        "app.api.widget.get_branding_by_prefix",
        new_callable=AsyncMock,
        return_value=branding,
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.get(f"/api/v1/widget/branding/{VALID_PREFIX}")

    assert resp.status_code == 200
    data = resp.json()

    # These sensitive fields must NEVER appear in the public branding response
    sensitive_fields = [
        "hashed_api_key",
        "hashed_password",
        "encrypted_openai_key",
        "email",
        "password",
        "api_key",
    ]
    for field in sensitive_fields:
        assert field not in data, f"Sensitive field '{field}' found in branding response"


@pytest.mark.asyncio
async def test_branding_no_auth_required():
    """GET /branding/{prefix} returns 200 without Authorization header."""
    from app.main import app

    branding = make_branding_response()

    with patch(
        "app.api.widget.get_branding_by_prefix",
        new_callable=AsyncMock,
        return_value=branding,
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            # Explicitly send NO Authorization header
            resp = await client.get(
                f"/api/v1/widget/branding/{VALID_PREFIX}",
                headers={},  # No auth header
            )

    # Must be 200, NOT 401
    assert resp.status_code == 200, (
        f"Expected 200 but got {resp.status_code} — branding endpoint must not require auth"
    )


@pytest.mark.asyncio
async def test_branding_includes_company_name():
    """Branding response includes company_name from tenant."""
    from app.main import app

    branding = make_branding_response(company_name="Sunshine Plumbing Inc.")

    with patch(
        "app.api.widget.get_branding_by_prefix",
        new_callable=AsyncMock,
        return_value=branding,
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.get(f"/api/v1/widget/branding/{VALID_PREFIX}")

    assert resp.status_code == 200
    assert resp.json()["company_name"] == "Sunshine Plumbing Inc."


# ---------------------------------------------------------------------------
# Lead capture endpoint tests
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_lead_capture_requires_auth():
    """POST /widget/lead returns 401 when no Authorization header is provided."""
    from app.main import app

    payload = {
        "session_id": "sess_test001",
        "name": "Jane Doe",
        "email": "jane@example.com",
        "phone": "555-555-5555",
    }

    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        resp = await client.post("/api/v1/widget/lead", json=payload)

    # No auth header — must get 401 or 403
    assert resp.status_code in (401, 403), (
        f"Expected 401/403 but got {resp.status_code} — lead endpoint must require auth"
    )


@pytest.mark.asyncio
async def test_lead_capture_saves_lead():
    """POST /widget/lead with valid API key saves a lead document."""
    from app.main import app
    from app.core.security import get_current_tenant

    tenant = make_tenant()

    payload = {
        "session_id": "sess_lead001",
        "name": "John Smith",
        "email": "john@example.com",
        "phone": "555-123-4567",
    }

    async def mock_get_tenant():
        return tenant

    with patch(
        "app.api.widget.save_lead",
        new_callable=AsyncMock,
        return_value="507f1f77bcf86cd799439011",
    ):
        app.dependency_overrides[get_current_tenant] = mock_get_tenant
        try:
            async with AsyncClient(
                transport=ASGITransport(app=app), base_url="http://test"
            ) as client:
                resp = await client.post("/api/v1/widget/lead", json=payload)
        finally:
            app.dependency_overrides.pop(get_current_tenant, None)

    assert resp.status_code == 201
    data = resp.json()
    assert data["session_id"] == "sess_lead001"
    assert "captured" in data["message"].lower() or "success" in data["message"].lower()


# ---------------------------------------------------------------------------
# Security tests (WSEC-02) — auth enforcement on protected endpoints
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_lead_capture_without_auth_returns_401():
    """POST /widget/lead without auth returns 401 (WSEC-02).

    Lead capture is an authenticated endpoint — no API key means no access.
    Verifies WSEC-02: all widget API calls (except branding) reject unauthenticated requests.
    """
    from app.main import app

    payload = {
        "session_id": "sess_wsec02",
        "name": "Test User",
        "email": "test@example.com",
        "phone": "555-000-0000",
    }

    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        resp = await client.post("/api/v1/widget/lead", json=payload)

    assert resp.status_code in (401, 403), (
        f"Expected 401/403 but got {resp.status_code} — lead endpoint must require auth (WSEC-02)"
    )


@pytest.mark.asyncio
async def test_analytics_without_auth_returns_401():
    """POST /widget/analytics without auth returns 401 (WSEC-02).

    Analytics is an authenticated endpoint — no API key means no access.
    """
    from app.main import app

    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        resp = await client.post(
            "/api/v1/widget/analytics",
            json={"event_type": "widget_view"},
        )

    assert resp.status_code in (401, 403), (
        f"Expected 401/403 but got {resp.status_code} — analytics endpoint must require auth (WSEC-02)"
    )


@pytest.mark.asyncio
async def test_branding_endpoint_does_not_require_auth():
    """GET /widget/branding/{prefix} is public — no auth required (BRND-04).

    The widget must be able to fetch branding before the user interacts.
    This endpoint must return 200 without any Authorization header.
    """
    from app.main import app

    branding = make_branding_response()

    with patch(
        "app.api.widget.get_branding_by_prefix",
        new_callable=AsyncMock,
        return_value=branding,
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.get(f"/api/v1/widget/branding/{VALID_PREFIX}")

    assert resp.status_code == 200, (
        f"Expected 200 but got {resp.status_code} — branding endpoint must be public (BRND-04)"
    )


# ---------------------------------------------------------------------------
# CORS tests
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_analytics_without_auth_returns_401():
    """POST /widget/analytics returns 401 when no Authorization header is provided (WSEC-02)."""
    from app.main import app

    payload = {"event_type": "widget_view"}

    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        resp = await client.post("/api/v1/widget/analytics", json=payload)

    # No auth header — must get 401 or 403
    assert resp.status_code in (401, 403), (
        f"Expected 401/403 but got {resp.status_code} — analytics endpoint must require auth"
    )


@pytest.mark.asyncio
async def test_branding_endpoint_does_not_require_auth():
    """GET /widget/branding/{prefix} is public — no auth required (BRND-04)."""
    from app.main import app

    branding = make_branding_response()

    with patch(
        "app.api.widget.get_branding_by_prefix",
        new_callable=AsyncMock,
        return_value=branding,
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.get(
                f"/api/v1/widget/branding/{VALID_PREFIX}",
                # Explicitly NO Authorization header
            )

    assert resp.status_code == 200, (
        f"Expected 200 but got {resp.status_code} — branding endpoint must be public"
    )


# ---------------------------------------------------------------------------
# CORS tests
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_cors_allows_static_origins():
    """CORS middleware allows requests from static ALLOWED_ORIGINS list."""
    from app.main import app
    from app.core.config import settings

    # Use the first configured ALLOWED_ORIGIN (or a test default)
    origin = settings.ALLOWED_ORIGINS[0] if settings.ALLOWED_ORIGINS else "http://localhost:3000"

    branding = make_branding_response()

    with patch(
        "app.api.widget.get_branding_by_prefix",
        new_callable=AsyncMock,
        return_value=branding,
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.options(
                f"/api/v1/widget/branding/{VALID_PREFIX}",
                headers={
                    "Origin": origin,
                    "Access-Control-Request-Method": "GET",
                },
            )

    # Preflight should succeed for static origins (200 or 204)
    assert resp.status_code in (200, 204)


@pytest.mark.asyncio
async def test_cors_allows_tenant_registered_origin():
    """CORS middleware allows requests from tenant-registered origins after cache update."""
    from app.middleware.cors import _tenant_origins_cache

    tenant_origin = "https://widget.acme-contracting.com"
    tenant_id = "test-tenant-cors-001"

    # Populate the cache directly (simulates what widget_service does after branding fetch)
    _tenant_origins_cache[tenant_id] = [tenant_origin]

    try:
        from app.middleware.cors import TenantAwareCORSMiddleware
        from starlette.testclient import TestClient

        # Create a minimal middleware instance and test is_allowed_origin
        # (Testing the cache logic without a full ASGI call)
        mock_app = MagicMock()
        middleware = TenantAwareCORSMiddleware(mock_app, allow_origins=["http://localhost:3000"])

        # The tenant-registered origin should be allowed
        assert middleware.is_allowed_origin(tenant_origin) is True

        # A random unknown origin should NOT be allowed
        assert middleware.is_allowed_origin("https://random-unknown-site.com") is False

        # The static origin should still be allowed
        assert middleware.is_allowed_origin("http://localhost:3000") is True

    finally:
        # Clean up cache
        _tenant_origins_cache.pop(tenant_id, None)
