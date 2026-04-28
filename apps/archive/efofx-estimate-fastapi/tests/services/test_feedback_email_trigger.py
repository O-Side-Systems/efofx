"""
Unit tests for feedback email trigger endpoint.

POST /api/v1/feedback/request-email/{session_id}

Tests:
- Success: creates magic link and queues email via BackgroundTasks
- Session not found: returns 404
- Unauthenticated: returns 401
- Email template renders with tenant branding and estimate context

Mock strategy:
- Mock db.find_one (session document lookup) to isolate DB
- Mock MagicLinkService.create_magic_link to skip DB token creation
- Mock FeedbackEmailService.send_email to skip actual email dispatch
- Use FastAPI dependency_overrides for get_current_tenant
- Disable rate limiter for all tests (Valkey not available in test environment)
"""

from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from httpx import ASGITransport, AsyncClient

from app.models.tenant import Tenant


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_tenant(
    tenant_id: str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
    company_name: str = "Test Pools Inc",
    settings: dict | None = None,
) -> Tenant:
    """Create a minimal Tenant for dependency override."""
    return Tenant(
        tenant_id=tenant_id,
        company_name=company_name,
        email="contractor@example.com",
        hashed_password="$2b$12$hashed",
        hashed_api_key="$2b$12$hashed_api",
        api_key_last6="abc123",
        tier="trial",
        email_verified=True,
        settings=settings or {},
    )


def make_session_doc(
    session_id: str = "session-001",
    tenant_id: str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
    estimation_output: dict | None = None,
) -> dict:
    """Create a minimal estimation session document."""
    return {
        "session_id": session_id,
        "tenant_id": tenant_id,
        "description": "Pool installation project",
        "estimation_output": estimation_output or {
            "total_cost_p50": 58000,
            "total_cost_p80": 72000,
            "timeline_weeks_p50": 8,
            "timeline_weeks_p80": 11,
            "cost_breakdown": [
                {"category": "Materials", "p50_cost": 20000, "p80_cost": 25000},
                {"category": "Labor", "p50_cost": 18000, "p80_cost": 22000},
            ],
            "assumptions": ["Standard soil conditions", "No permit delays"],
        },
    }


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture(autouse=True)
def disable_rate_limiter():
    """Disable the rate limiter for all tests.

    The rate limiter requires a live Valkey connection in production.
    Disable it globally for tests to avoid connection errors.
    """
    from app.core.rate_limit import limiter

    original = limiter.enabled
    limiter.enabled = False
    yield
    limiter.enabled = original


# ---------------------------------------------------------------------------
# test_request_feedback_email_success
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_request_feedback_email_success():
    """POST /feedback/request-email/{session_id} returns 200 with token_hash.

    Mocks:
    - db.find_one returns a valid session document
    - MagicLinkService.create_magic_link returns (raw_token, token_hash)
    - FeedbackEmailService.send_email is mocked (no actual dispatch)
    """
    from app.core.security import get_current_tenant
    from app.main import app

    tenant = make_tenant()
    session_doc = make_session_doc()

    # Build mock DB that returns the session document
    mock_collection = MagicMock()
    mock_collection.find_one = AsyncMock(return_value=session_doc)
    mock_db = MagicMock()
    mock_db.__getitem__ = MagicMock(return_value=mock_collection)

    async def mock_get_tenant() -> Tenant:
        return tenant

    with (
        patch("app.api.feedback_email.get_database", return_value=mock_db),
        patch(
            "app.api.feedback_email.MagicLinkService.create_magic_link",
            new_callable=AsyncMock,
            return_value=("raw-token-abc123", "hash-abc123-hash"),
        ),
        patch(
            "app.services.feedback_email_service.FeedbackEmailService.send_email",
            new_callable=AsyncMock,
            return_value="resend-message-id",
        ),
    ):
        app.dependency_overrides[get_current_tenant] = mock_get_tenant
        try:
            async with AsyncClient(
                transport=ASGITransport(app=app), base_url="http://test"
            ) as client:
                resp = await client.post(
                    "/api/v1/feedback/request-email/session-001",
                    json={
                        "customer_email": "customer@example.com",
                        "project_name": "Pool Installation",
                    },
                )
        finally:
            app.dependency_overrides.pop(get_current_tenant, None)

    assert resp.status_code == 200, f"Expected 200, got {resp.status_code}: {resp.text}"
    data = resp.json()
    assert data["message"] == "Feedback email queued"
    assert data["token_hash"] == "hash-abc123-hash"


# ---------------------------------------------------------------------------
# test_request_feedback_email_session_not_found
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_request_feedback_email_session_not_found():
    """POST /feedback/request-email/{session_id} returns 404 for unknown session.

    When db.find_one returns None, the endpoint must raise 404.
    """
    from app.core.security import get_current_tenant
    from app.main import app

    tenant = make_tenant()

    # db.find_one returns None (session not found)
    mock_collection = MagicMock()
    mock_collection.find_one = AsyncMock(return_value=None)
    mock_db = MagicMock()
    mock_db.__getitem__ = MagicMock(return_value=mock_collection)

    async def mock_get_tenant() -> Tenant:
        return tenant

    with patch("app.api.feedback_email.get_database", return_value=mock_db):
        app.dependency_overrides[get_current_tenant] = mock_get_tenant
        try:
            async with AsyncClient(
                transport=ASGITransport(app=app), base_url="http://test"
            ) as client:
                resp = await client.post(
                    "/api/v1/feedback/request-email/nonexistent-session",
                    json={
                        "customer_email": "customer@example.com",
                        "project_name": "Pool Installation",
                    },
                )
        finally:
            app.dependency_overrides.pop(get_current_tenant, None)

    assert resp.status_code == 404, f"Expected 404, got {resp.status_code}: {resp.text}"
    assert "not found" in resp.json()["detail"].lower()


# ---------------------------------------------------------------------------
# test_request_feedback_email_unauthenticated
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_request_feedback_email_unauthenticated():
    """POST /feedback/request-email/{session_id} returns 401 without auth.

    No Authorization header = must reject with 401 or 403.
    """
    from app.main import app

    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        resp = await client.post(
            "/api/v1/feedback/request-email/session-001",
            json={
                "customer_email": "customer@example.com",
                "project_name": "Pool Installation",
            },
        )

    assert resp.status_code in (401, 403), (
        f"Expected 401/403 for unauthenticated request, got {resp.status_code}"
    )


# ---------------------------------------------------------------------------
# test_email_template_renders_with_branding
# ---------------------------------------------------------------------------


def test_email_template_renders_with_branding():
    """Email template renders correctly with tenant branding and estimate context.

    Directly invokes Jinja2 to verify:
    - All required context variables are rendered
    - project_name appears in output
    - cost values are formatted correctly
    - magic_link_url appears in the CTA button href
    - company_name and primary_color appear in output
    """
    from jinja2 import Environment, FileSystemLoader
    import os

    templates_dir = os.path.join(
        os.path.dirname(os.path.dirname(os.path.dirname(__file__))),
        "app",
        "templates",
    )
    env = Environment(loader=FileSystemLoader(templates_dir))
    tpl = env.get_template("feedback_email.html")

    html = tpl.render(
        company_name="Sunshine Pools LLC",
        logo_url="https://example.com/logo.png",
        primary_color="#1e40af",
        accent_color="#1e3a8a",
        project_name="Backyard Pool Build",
        total_cost_p50=45000,
        total_cost_p80=58000,
        timeline_weeks_p50=6,
        timeline_weeks_p80=9,
        cost_breakdown=[
            {"category": "Materials", "p50_cost": 18000, "p80_cost": 22000},
            {"category": "Labor", "p50_cost": 15000, "p80_cost": 18000},
        ],
        assumptions=["Permit included", "Standard soil"],
        magic_link_url="https://efofx.com/feedback/form/test-magic-token",
    )

    # Project name present
    assert "Backyard Pool Build" in html

    # Company branding present
    assert "Sunshine Pools LLC" in html
    assert "#1e40af" in html  # primary_color in header background

    # Logo URL present
    assert "https://example.com/logo.png" in html

    # Cost range formatted
    assert "45,000" in html
    assert "58,000" in html

    # Timeline
    assert "6" in html
    assert "9" in html

    # Cost breakdown categories
    assert "Materials" in html or "materials" in html
    assert "Labor" in html or "labor" in html

    # Assumptions
    assert "Permit included" in html
    assert "Standard soil" in html

    # CTA button
    assert "Share Your Feedback" in html
    assert "test-magic-token" in html

    # Footer
    assert "72 hours" in html
    assert "efOfX" in html


# ---------------------------------------------------------------------------
# test_email_template_renders_without_optional_sections
# ---------------------------------------------------------------------------


def test_email_template_renders_without_optional_sections():
    """Email template renders when cost_breakdown and assumptions are empty.

    Verifies that conditional Jinja2 blocks work correctly
    (no KeyError or TemplateError for missing optional data).
    """
    from jinja2 import Environment, FileSystemLoader
    import os

    templates_dir = os.path.join(
        os.path.dirname(os.path.dirname(os.path.dirname(__file__))),
        "app",
        "templates",
    )
    env = Environment(loader=FileSystemLoader(templates_dir))
    tpl = env.get_template("feedback_email.html")

    # Render with empty optional sections
    html = tpl.render(
        company_name="Test Co",
        logo_url=None,
        primary_color="#2563eb",
        accent_color="#1d4ed8",
        project_name="Test Project",
        total_cost_p50=10000,
        total_cost_p80=15000,
        timeline_weeks_p50=4,
        timeline_weeks_p80=6,
        cost_breakdown=[],  # empty
        assumptions=[],      # empty
        magic_link_url="http://test/feedback/form/abc",
    )

    assert "Test Project" in html
    assert "Share Your Feedback" in html
    # Breakdown table header should not appear when list is empty
    assert "Cost Breakdown" not in html
    assert "Assumptions" not in html
