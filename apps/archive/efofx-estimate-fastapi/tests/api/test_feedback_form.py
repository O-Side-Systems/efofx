"""
Tests for customer feedback form endpoints.

Tests:
- GET /feedback/form/{token} renders form for valid tokens (mark_opened called)
- GET /feedback/form/{token} renders expired page for expired/not_found tokens
- GET /feedback/form/{token} renders thank-you page for used tokens
- POST /feedback/form/{token} stores feedback and renders thank-you for valid tokens
- POST /feedback/form/{token} renders thank-you without double-storing for used tokens
- POST /feedback/form/{token} handles race-condition double-submit (consume returns False)

Mock strategy:
- Patch MagicLinkService methods (resolve_token_state, mark_opened, consume)
- Patch _load_estimate_context to isolate DB calls
- Patch FeedbackService.store_feedback_with_snapshot to isolate DB writes
- Disable rate limiter (Valkey not available in test environment)
- Use httpx.AsyncClient with ASGITransport for full ASGI stack testing
"""

import pytest
from unittest.mock import AsyncMock, patch
from httpx import AsyncClient, ASGITransport


# ---------------------------------------------------------------------------
# Helpers / shared fixtures
# ---------------------------------------------------------------------------


VALID_TOKEN = "test-raw-token-abc123"

TOKEN_DOC = {
    "token_hash": "hashed",
    "tenant_id": "tenant-uuid-001",
    "estimation_session_id": "session-uuid-001",
    "customer_email": "customer@example.com",
    "project_name": "Backyard Pool",
}

ESTIMATE_CTX = {
    "company_name": "Poolside Pros",
    "logo_url": None,
    "primary_color": "#2563eb",
    "accent_color": "#1d4ed8",
    "secondary_color": "#f3f4f6",
    "project_name": "Backyard Pool",
    "customer_email": "customer@example.com",
    "total_cost_p50": 50000.0,
    "total_cost_p80": 65000.0,
    "timeline_weeks_p50": 8,
    "timeline_weeks_p80": 12,
    "cost_breakdown": [{"category": "Labor", "p50_cost": 30000, "p80_cost": 40000}],
    "assumptions": ["Standard soil conditions"],
    "reference_class_id": "residential_pool",
    "estimate_data": {
        "total_cost_p50": 50000.0,
        "total_cost_p80": 65000.0,
        "timeline_weeks_p50": 8,
        "timeline_weeks_p80": 12,
        "cost_breakdown": [],
        "assumptions": [],
        "confidence_score": 0.85,
    },
}


@pytest.fixture(autouse=True)
def disable_rate_limiter():
    """Disable the rate limiter for all feedback form tests."""
    from app.core.rate_limit import limiter

    original = limiter.enabled
    limiter.enabled = False
    yield
    limiter.enabled = original


# ---------------------------------------------------------------------------
# GET /feedback/form/{token} — valid token
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_get_form_valid_token():
    """GET with valid token renders the feedback form with form fields."""
    from app.main import app

    with (
        patch(
            "app.api.feedback_form.MagicLinkService.resolve_token_state",
            new_callable=AsyncMock,
            return_value=("valid", TOKEN_DOC),
        ),
        patch(
            "app.api.feedback_form.MagicLinkService.mark_opened",
            new_callable=AsyncMock,
        ) as mock_mark_opened,
        patch(
            "app.api.feedback_form._load_estimate_context",
            new_callable=AsyncMock,
            return_value=ESTIMATE_CTX,
        ),
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.get(f"/feedback/form/{VALID_TOKEN}")

    assert resp.status_code == 200
    assert "text/html" in resp.headers["content-type"]
    html = resp.text
    assert "actual_cost" in html
    assert VALID_TOKEN in html
    assert "Backyard Pool" in html
    mock_mark_opened.assert_awaited_once()


# ---------------------------------------------------------------------------
# GET /feedback/form/{token} — expired token
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_get_form_expired_token():
    """GET with expired token renders the expired/not-available page."""
    from app.main import app

    with patch(
        "app.api.feedback_form.MagicLinkService.resolve_token_state",
        new_callable=AsyncMock,
        return_value=("expired", TOKEN_DOC),
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.get(f"/feedback/form/{VALID_TOKEN}")

    assert resp.status_code == 200
    html = resp.text.lower()
    assert "expired" in html or "no longer" in html


# ---------------------------------------------------------------------------
# GET /feedback/form/{token} — not_found token
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_get_form_not_found_token():
    """GET with unknown token renders the expired/not-available page."""
    from app.main import app

    with patch(
        "app.api.feedback_form.MagicLinkService.resolve_token_state",
        new_callable=AsyncMock,
        return_value=("not_found", None),
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.get(f"/feedback/form/unknown-token")

    assert resp.status_code == 200
    html = resp.text.lower()
    assert "expired" in html or "no longer" in html


# ---------------------------------------------------------------------------
# GET /feedback/form/{token} — used token
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_get_form_used_token():
    """GET with used token renders the thank-you page."""
    from app.main import app

    with (
        patch(
            "app.api.feedback_form.MagicLinkService.resolve_token_state",
            new_callable=AsyncMock,
            return_value=("used", TOKEN_DOC),
        ),
        patch(
            "app.api.feedback_form._load_estimate_context",
            new_callable=AsyncMock,
            return_value=ESTIMATE_CTX,
        ),
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.get(f"/feedback/form/{VALID_TOKEN}")

    assert resp.status_code == 200
    html = resp.text.lower()
    assert "thank" in html


# ---------------------------------------------------------------------------
# POST /feedback/form/{token} — valid submission
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_post_form_valid_submission():
    """POST with valid token stores feedback and renders thank-you page."""
    from app.main import app

    form_data = {
        "actual_cost": "55000",
        "actual_timeline": "10",
        "rating": "4",
        "discrepancy_reason_primary": "scope_changed",
        "discrepancy_reason_secondary": "",
        "comment": "Good estimate overall.",
    }

    with (
        patch(
            "app.api.feedback_form.MagicLinkService.resolve_token_state",
            new_callable=AsyncMock,
            return_value=("valid", TOKEN_DOC),
        ),
        patch(
            "app.api.feedback_form.MagicLinkService.consume",
            new_callable=AsyncMock,
            return_value=True,
        ),
        patch(
            "app.api.feedback_form._load_estimate_context",
            new_callable=AsyncMock,
            return_value=ESTIMATE_CTX,
        ),
        patch(
            "app.api.feedback_form.FeedbackService.store_feedback_with_snapshot",
            new_callable=AsyncMock,
            return_value="inserted-doc-id-001",
        ) as mock_store,
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.post(
                f"/feedback/form/{VALID_TOKEN}",
                data=form_data,
            )

    assert resp.status_code == 200
    html = resp.text.lower()
    assert "thank" in html
    mock_store.assert_awaited_once()


# ---------------------------------------------------------------------------
# POST /feedback/form/{token} — already used token
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_post_form_already_used():
    """POST with already-used token renders thank-you without storing again."""
    from app.main import app

    form_data = {
        "actual_cost": "55000",
        "actual_timeline": "10",
        "rating": "4",
        "discrepancy_reason_primary": "scope_changed",
    }

    with (
        patch(
            "app.api.feedback_form.MagicLinkService.resolve_token_state",
            new_callable=AsyncMock,
            return_value=("used", TOKEN_DOC),
        ),
        patch(
            "app.api.feedback_form._load_estimate_context",
            new_callable=AsyncMock,
            return_value=ESTIMATE_CTX,
        ),
        patch(
            "app.api.feedback_form.FeedbackService.store_feedback_with_snapshot",
            new_callable=AsyncMock,
        ) as mock_store,
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.post(
                f"/feedback/form/{VALID_TOKEN}",
                data=form_data,
            )

    assert resp.status_code == 200
    html = resp.text.lower()
    assert "thank" in html
    # Must NOT store feedback again for an already-used token
    mock_store.assert_not_awaited()


# ---------------------------------------------------------------------------
# POST /feedback/form/{token} — double-submit race condition
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_post_form_double_submit_race():
    """POST consume() returns False (race condition) — renders thank-you without storing."""
    from app.main import app

    form_data = {
        "actual_cost": "55000",
        "actual_timeline": "10",
        "rating": "4",
        "discrepancy_reason_primary": "scope_changed",
    }

    with (
        patch(
            "app.api.feedback_form.MagicLinkService.resolve_token_state",
            new_callable=AsyncMock,
            return_value=("valid", TOKEN_DOC),
        ),
        patch(
            "app.api.feedback_form.MagicLinkService.consume",
            new_callable=AsyncMock,
            return_value=False,  # Another request won the race
        ),
        patch(
            "app.api.feedback_form._load_estimate_context",
            new_callable=AsyncMock,
            return_value=ESTIMATE_CTX,
        ),
        patch(
            "app.api.feedback_form.FeedbackService.store_feedback_with_snapshot",
            new_callable=AsyncMock,
        ) as mock_store,
    ):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.post(
                f"/feedback/form/{VALID_TOKEN}",
                data=form_data,
            )

    assert resp.status_code == 200
    html = resp.text.lower()
    assert "thank" in html
    # Must NOT store feedback when consume() fails (race condition guard)
    mock_store.assert_not_awaited()
