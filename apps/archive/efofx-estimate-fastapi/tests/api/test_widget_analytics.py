"""
Tests for widget analytics event recording and retrieval.

Requirements verified:
- WSEC-02: POST /widget/analytics requires API key auth — 401 without auth
- WFTR-04: widget_view, chat_start, estimate_complete events tracked per tenant per day
- Analytics documents contain no PII — only tenant_id, date, and event counts
- record_analytics_event uses $inc upsert for daily counter bucketing
- GET /widget/analytics requires auth, returns daily counters
- Invalid event_type values are rejected with 400

Mock strategy:
- AsyncMock MongoDB operations to isolate from real DB
- FastAPI dependency_overrides for get_current_tenant (auth required)
- Disable rate limiter (Valkey not available in test environment)
- Patch at point of use (app.api.widget.*) not at definition
"""

import pytest
from unittest.mock import AsyncMock, MagicMock, patch, call
from httpx import AsyncClient, ASGITransport

from app.models.tenant import Tenant


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_tenant(tenant_id: str = "22222222-2222-2222-2222-222222222222") -> Tenant:
    """Create a minimal Tenant for dependency override."""
    return Tenant(
        tenant_id=tenant_id,
        company_name="Analytics Test Corp",
        email="analytics@example.com",
        hashed_password="$2b$12$hashed",
        hashed_api_key="$2b$12$hashed_api",
        api_key_last6="def456",
        tier="trial",
        email_verified=True,
    )


VALID_EVENT_TYPES = ["widget_view", "chat_start", "estimate_complete"]


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture(autouse=True)
def disable_rate_limiter():
    """Disable the rate limiter for all analytics tests.

    The rate limiter requires a live Valkey connection in production.
    Disable it globally for tests to avoid connection errors.
    """
    from app.core.rate_limit import limiter

    original = limiter.enabled
    limiter.enabled = False
    yield
    limiter.enabled = original


# ---------------------------------------------------------------------------
# Test 1: POST /widget/analytics requires auth (WSEC-02)
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_analytics_event_requires_auth():
    """POST /widget/analytics without auth returns 401 (WSEC-02).

    All widget API calls except branding must require API key authentication.
    """
    from app.main import app

    payload = {"event_type": "widget_view"}

    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        resp = await client.post("/api/v1/widget/analytics", json=payload)

    assert resp.status_code in (401, 403), (
        f"Expected 401/403 but got {resp.status_code} — "
        "analytics endpoint must reject unauthenticated requests"
    )


# ---------------------------------------------------------------------------
# Test 2: All valid event types return 204
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
@pytest.mark.parametrize("event_type", VALID_EVENT_TYPES)
async def test_analytics_event_valid_types(event_type: str):
    """POST /widget/analytics with each valid event_type returns 204 (WFTR-04).

    All three event types (widget_view, chat_start, estimate_complete) must be
    accepted and recorded without error.
    """
    from app.main import app
    from app.core.security import get_current_tenant

    tenant = make_tenant()

    async def mock_get_tenant():
        return tenant

    with patch(
        "app.api.widget.record_analytics_event",
        new_callable=AsyncMock,
        return_value=None,
    ):
        app.dependency_overrides[get_current_tenant] = mock_get_tenant
        try:
            async with AsyncClient(
                transport=ASGITransport(app=app), base_url="http://test"
            ) as client:
                resp = await client.post(
                    "/api/v1/widget/analytics",
                    json={"event_type": event_type},
                )
        finally:
            app.dependency_overrides.pop(get_current_tenant, None)

    assert resp.status_code == 204, (
        f"Expected 204 for event_type='{event_type}' but got {resp.status_code}"
    )


# ---------------------------------------------------------------------------
# Test 3: Invalid event type returns 400
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_analytics_event_invalid_type():
    """POST /widget/analytics with unknown event_type returns 400.

    Only widget_view, chat_start, estimate_complete are valid.
    Arbitrary strings (e.g. 'hacked', 'DROP TABLE') must be rejected.
    """
    from app.main import app
    from app.core.security import get_current_tenant

    tenant = make_tenant()

    async def mock_get_tenant():
        return tenant

    app.dependency_overrides[get_current_tenant] = mock_get_tenant
    try:
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.post(
                "/api/v1/widget/analytics",
                json={"event_type": "hacked"},
            )
    finally:
        app.dependency_overrides.pop(get_current_tenant, None)

    assert resp.status_code == 400, (
        f"Expected 400 for invalid event_type but got {resp.status_code}"
    )
    data = resp.json()
    assert "event_type" in data["detail"].lower() or "invalid" in data["detail"].lower()


# ---------------------------------------------------------------------------
# Test 4: Daily bucketing — $inc upsert creates counter document
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_analytics_daily_bucketing():
    """record_analytics_event creates a daily counter document via $inc upsert (WFTR-04).

    Each event recorded should increment the counter for the current date.
    The function uses upsert=True so first call creates the document.
    """
    from app.services.widget_service import record_analytics_event

    mock_col = AsyncMock()
    mock_col.update_one = AsyncMock(return_value=MagicMock(modified_count=1))

    with patch(
        "app.services.widget_service.get_tenant_collection",
        return_value=mock_col,
    ):
        await record_analytics_event("test-tenant-id", "widget_view")

    # Verify update_one was called with $inc and upsert=True
    mock_col.update_one.assert_called_once()
    call_args = mock_col.update_one.call_args

    # First positional arg is the filter — must include "date" key
    filter_doc = call_args[0][0]
    assert "date" in filter_doc, "Filter must include 'date' for daily bucketing"

    # Second positional arg is the update — must use $inc
    update_doc = call_args[0][1]
    assert "$inc" in update_doc, "Update must use $inc for counter increments"
    assert "widget_view" in update_doc["$inc"], "widget_view must be in $inc fields"

    # kwargs must include upsert=True
    assert call_args[1].get("upsert") is True, "upsert=True required for daily bucketing"


# ---------------------------------------------------------------------------
# Test 5: Increment existing counter
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_analytics_increment_existing():
    """Recording the same event twice on the same day increments the counter twice.

    Both calls should result in update_one being called with $inc — MongoDB
    handles the actual increment, but we verify both API calls go through.
    """
    from app.services.widget_service import record_analytics_event

    mock_col = AsyncMock()
    mock_col.update_one = AsyncMock(return_value=MagicMock(modified_count=1))

    with patch(
        "app.services.widget_service.get_tenant_collection",
        return_value=mock_col,
    ):
        await record_analytics_event("test-tenant-id", "chat_start")
        await record_analytics_event("test-tenant-id", "chat_start")

    # update_one called twice — each event triggers an upsert
    assert mock_col.update_one.call_count == 2, (
        f"Expected 2 update_one calls, got {mock_col.update_one.call_count}"
    )

    # Both calls use the same date filter
    call1_filter = mock_col.update_one.call_args_list[0][0][0]
    call2_filter = mock_col.update_one.call_args_list[1][0][0]
    assert "date" in call1_filter
    assert "date" in call2_filter
    assert call1_filter["date"] == call2_filter["date"], (
        "Both calls on same day should use the same date bucket"
    )


# ---------------------------------------------------------------------------
# Test 6: Analytics documents contain no PII
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_analytics_no_pii():
    """Analytics $inc update must not include PII fields (WFTR-04).

    Only tenant_id (injected by TenantAwareCollection), date, and event counts
    should appear in the update document. No email, name, phone, session_id, etc.
    """
    from app.services.widget_service import record_analytics_event

    mock_col = AsyncMock()
    mock_col.update_one = AsyncMock(return_value=MagicMock(modified_count=1))

    with patch(
        "app.services.widget_service.get_tenant_collection",
        return_value=mock_col,
    ):
        await record_analytics_event("test-tenant-id", "estimate_complete")

    call_args = mock_col.update_one.call_args
    filter_doc = call_args[0][0]
    update_doc = call_args[0][1]

    # PII fields that must NEVER appear in analytics documents
    pii_fields = ["email", "name", "phone", "session_id", "user_agent", "ip_address"]

    for pii_field in pii_fields:
        assert pii_field not in filter_doc, (
            f"PII field '{pii_field}' found in analytics filter document"
        )
        for update_op in update_doc.values():
            if isinstance(update_op, dict):
                assert pii_field not in update_op, (
                    f"PII field '{pii_field}' found in analytics update document"
                )


# ---------------------------------------------------------------------------
# Test 7: GET /widget/analytics requires auth
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_analytics_get_requires_auth():
    """GET /widget/analytics without auth returns 401 (WSEC-02).

    Analytics retrieval must require API key or JWT authentication.
    """
    from app.main import app

    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        resp = await client.get("/api/v1/widget/analytics")

    assert resp.status_code in (401, 403), (
        f"Expected 401/403 but got {resp.status_code} — "
        "GET analytics endpoint must require authentication"
    )


# ---------------------------------------------------------------------------
# Test 8: GET /widget/analytics returns analytics array
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_analytics_get_returns_data():
    """GET /widget/analytics with valid auth returns analytics array.

    The response must contain an 'analytics' array and a 'days' field.
    """
    from app.main import app
    from app.core.security import get_current_tenant

    tenant = make_tenant()

    # Sample analytics documents — no PII fields
    sample_docs = [
        {
            "tenant_id": tenant.tenant_id,
            "date": "2026-02-27",
            "widget_view": 15,
            "chat_start": 8,
            "estimate_complete": 3,
        },
        {
            "tenant_id": tenant.tenant_id,
            "date": "2026-02-26",
            "widget_view": 10,
            "chat_start": 5,
            "estimate_complete": 2,
        },
    ]

    async def mock_get_tenant():
        return tenant

    # Mock the database cursor chain: find().sort() -> to_list()
    mock_cursor = AsyncMock()
    mock_cursor.sort = MagicMock(return_value=mock_cursor)
    mock_cursor.to_list = AsyncMock(return_value=sample_docs)

    mock_db = MagicMock()
    mock_db.__getitem__ = MagicMock(return_value=MagicMock(find=MagicMock(return_value=mock_cursor)))

    with patch("app.api.widget.get_database", return_value=mock_db):
        app.dependency_overrides[get_current_tenant] = mock_get_tenant
        try:
            async with AsyncClient(
                transport=ASGITransport(app=app), base_url="http://test"
            ) as client:
                resp = await client.get("/api/v1/widget/analytics")
        finally:
            app.dependency_overrides.pop(get_current_tenant, None)

    assert resp.status_code == 200, (
        f"Expected 200 but got {resp.status_code}: {resp.text}"
    )
    data = resp.json()

    # Response must contain 'analytics' array and 'days'
    assert "analytics" in data, "Response must contain 'analytics' key"
    assert "days" in data, "Response must contain 'days' key"
    assert isinstance(data["analytics"], list), "'analytics' must be a list"
    assert data["days"] == 30  # Default is 30 days

    # Verify returned documents match expected structure (no PII)
    assert len(data["analytics"]) == 2
    for doc in data["analytics"]:
        # Must have event count fields
        assert "date" in doc
        # Must NOT have PII fields
        pii_fields = ["email", "name", "phone", "session_id"]
        for pii in pii_fields:
            assert pii not in doc, f"PII field '{pii}' found in analytics response"
