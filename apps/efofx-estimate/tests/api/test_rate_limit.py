"""
Tests for rate limiting behavior.

Uses in-memory storage for the rate limiter by patching the global limiter
instance's internal storage before each test. This avoids needing a live
Valkey/Redis connection in CI.

Test patterns:
- IP-based brute-force protection on /auth/login (5/15minutes)
- Per-IP protection on /auth/register (10/hour)
- Per-tenant tier-based limits (trial: 20/min, paid: 100/min) config
- 429 response format: {"error": "rate_limit_exceeded", "message": "...", "retry_after": N}
- Retry-After header on 429 responses
- X-RateLimit-Limit / X-RateLimit-Remaining headers on normal responses
"""
import pytest
import pytest_asyncio
from unittest.mock import AsyncMock, MagicMock, patch
from httpx import ASGITransport, AsyncClient
from motor.motor_asyncio import AsyncIOMotorClient
from limits.storage import MemoryStorage
from limits.strategies import FixedWindowRateLimiter

from app.core.config import settings
from app.core.constants import DB_COLLECTIONS

TEST_DB = "efofx_estimate_test_ratelimit"


# ---------------------------------------------------------------------------
# Shared: patch global limiter to use in-memory storage
# ---------------------------------------------------------------------------


@pytest.fixture(autouse=True)
def use_memory_rate_limiter():
    """Patch the global rate limiter to use in-memory storage for all tests.

    This ensures tests don't connect to Valkey/Redis and each test starts
    from a clean state.

    The @limiter.limit decorator captures the global limiter object at
    decoration time, so we must patch the storage on the EXISTING instance
    (not create a new one).
    """
    from app.core.rate_limit import limiter

    # Save original storage/limiter
    original_storage = limiter._storage
    original_limiter = limiter._limiter

    # Swap in memory-backed storage
    mem_storage = MemoryStorage()
    limiter._storage = mem_storage
    limiter._limiter = FixedWindowRateLimiter(mem_storage)

    yield

    # Restore original storage after each test
    limiter._storage = original_storage
    limiter._limiter = original_limiter


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest_asyncio.fixture
async def mongo_client():
    """Isolated Motor client per test."""
    client = AsyncIOMotorClient(settings.MONGO_URI)
    yield client
    await client.drop_database(TEST_DB)
    client.close()


@pytest_asyncio.fixture
async def test_app(mongo_client):
    """FastAPI app with test DB injected."""
    import app.db.mongodb as _mdb

    db = mongo_client[TEST_DB]
    _mdb._client = mongo_client
    _mdb._database = db

    from app.main import app
    yield app

    # Cleanup
    _mdb._client = None
    _mdb._database = None


@pytest_asyncio.fixture
async def client(test_app):
    """Async HTTP client for the test app."""
    async with AsyncClient(
        transport=ASGITransport(app=test_app), base_url="http://test"
    ) as ac:
        yield ac


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

VALID_REGISTER_PAYLOAD = {
    "company_name": "Rate Test Builders",
    "email": "ratelimit@test.example",
    "password": "secure-password-123",
}


async def _register(client, payload=None):
    """Register a tenant with mocked email sending."""
    payload = payload or VALID_REGISTER_PAYLOAD.copy()
    with patch(
        "app.services.auth_service.send_verification_email",
        new_callable=AsyncMock,
    ):
        response = await client.post("/api/v1/auth/register", json=payload)
    return response


async def _register_and_verify(client, mongo_client):
    """Register and verify a tenant. Returns (api_key, tenant_id)."""
    reg_response = await _register(client)
    assert reg_response.status_code == 201, f"Registration failed: {reg_response.json()}"
    tenant_id = reg_response.json()["tenant_id"]
    api_key = reg_response.json()["api_key"]

    db = mongo_client[TEST_DB]
    token_doc = await db[DB_COLLECTIONS["VERIFICATION_TOKENS"]].find_one(
        {"tenant_id": tenant_id}
    )
    if token_doc:
        await client.get(f"/api/v1/auth/verify?token={token_doc['token']}")

    return api_key, tenant_id


# ---------------------------------------------------------------------------
# Login brute-force protection tests
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_login_rate_limit_ip(client):
    """6th login attempt from same IP within 15 minutes returns 429."""
    login_payload = {"email": "nobody@example.com", "password": "anypassword"}

    responses = []
    for i in range(6):
        r = await client.post("/api/v1/auth/login", json=login_payload)
        responses.append(r.status_code)

    # First 5 should NOT be 429 (they'll be 401 or 403)
    for i, status in enumerate(responses[:5]):
        assert status != 429, f"Request {i+1} should not be rate-limited, got {status}"

    # 6th request should be rate-limited
    assert responses[5] == 429, (
        f"6th login request should be rate-limited (429), got {responses[5]}"
    )


@pytest.mark.asyncio
async def test_login_rate_limit_returns_proper_429_format(client):
    """429 response has correct JSON body with error, message, retry_after fields."""
    login_payload = {"email": "nobody@example.com", "password": "anypassword"}

    # Send 6 requests to trigger rate limit
    response = None
    for _ in range(6):
        response = await client.post("/api/v1/auth/login", json=login_payload)

    assert response is not None
    assert response.status_code == 429
    body = response.json()
    assert "error" in body, f"Response body missing 'error': {body}"
    assert body["error"] == "rate_limit_exceeded"
    assert "message" in body, f"Response body missing 'message': {body}"
    assert "retry_after" in body, f"Response body missing 'retry_after': {body}"
    assert isinstance(body["retry_after"], int)


@pytest.mark.asyncio
async def test_429_has_retry_after_header(client):
    """Rate-limited response includes Retry-After header."""
    login_payload = {"email": "nobody@example.com", "password": "anypassword"}

    response = None
    for _ in range(6):
        response = await client.post("/api/v1/auth/login", json=login_payload)

    assert response is not None
    assert response.status_code == 429
    # Headers may be case-folded by httpx
    headers = {k.lower(): v for k, v in response.headers.items()}
    assert "retry-after" in headers, (
        f"Expected 'Retry-After' header in 429 response. Got: {list(response.headers.keys())}"
    )


# ---------------------------------------------------------------------------
# Rate limit header tests on non-rate-limited responses
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_rate_limit_headers_present_on_auth_endpoint(client):
    """Auth endpoint responses include X-RateLimit-Limit and X-RateLimit-Remaining headers.

    slowapi automatically injects these headers when the limiter is active.
    """
    login_payload = {"email": "nobody@example.com", "password": "anypassword"}
    response = await client.post("/api/v1/auth/login", json=login_payload)

    # Should not be rate limited on first request
    assert response.status_code != 429

    # slowapi injects X-RateLimit-* headers
    headers = {k.lower(): v for k, v in response.headers.items()}
    has_ratelimit_headers = any(
        h in headers
        for h in ["x-ratelimit-limit", "x-ratelimit-remaining", "x-ratelimit-reset"]
    )
    assert has_ratelimit_headers, (
        f"Expected X-RateLimit-* headers in response. Got headers: {list(response.headers.keys())}"
    )


# ---------------------------------------------------------------------------
# 429 response format validation
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_429_response_format(client):
    """Rate-limited response has proper JSON body structure."""
    login_payload = {"email": "nobody@example.com", "password": "anypassword"}

    # Exhaust the 5/15min login rate limit
    for _ in range(5):
        await client.post("/api/v1/auth/login", json=login_payload)

    # 6th request should hit the rate limit
    response = await client.post("/api/v1/auth/login", json=login_payload)
    assert response.status_code == 429

    body = response.json()
    # Validate the standardized error format
    assert body.get("error") == "rate_limit_exceeded"
    assert "message" in body
    assert "rate limit exceeded" in body["message"].lower()
    assert "retry_after" in body
    assert isinstance(body["retry_after"], (int, float))


# ---------------------------------------------------------------------------
# TIER_LIMITS configuration tests (unit tests — no HTTP)
# ---------------------------------------------------------------------------


def test_tier_limits_configuration():
    """TIER_LIMITS dict contains correct per-tier values (trial 20/min, paid 100/min)."""
    from app.core.rate_limit import TIER_LIMITS

    assert "trial" in TIER_LIMITS
    assert "paid" in TIER_LIMITS
    assert TIER_LIMITS["trial"] == "20/minute"
    assert TIER_LIMITS["paid"] == "100/minute"


def test_get_tier_limit_defaults_to_trial():
    """get_tier_limit returns trial limit when no tier is set on request.state."""
    from app.core.rate_limit import get_tier_limit, TIER_LIMITS

    mock_request = MagicMock()
    # Empty state — no tier attribute
    mock_request.state = MagicMock(spec=[])

    result = get_tier_limit(mock_request)
    assert result == TIER_LIMITS["trial"]


def test_get_tier_limit_paid():
    """get_tier_limit returns paid limit when tier='paid' is set on request.state."""
    from app.core.rate_limit import get_tier_limit, TIER_LIMITS

    mock_request = MagicMock()
    mock_request.state.tier = "paid"

    result = get_tier_limit(mock_request)
    assert result == TIER_LIMITS["paid"]


def test_get_tier_limit_trial():
    """get_tier_limit returns trial limit when tier='trial' is set on request.state."""
    from app.core.rate_limit import get_tier_limit, TIER_LIMITS

    mock_request = MagicMock()
    mock_request.state.tier = "trial"

    result = get_tier_limit(mock_request)
    assert result == TIER_LIMITS["trial"]


# ---------------------------------------------------------------------------
# Limiter configuration tests (unit tests — no HTTP)
# ---------------------------------------------------------------------------


def test_limiter_is_enabled():
    """Rate limiter is enabled by default (RATE_LIMIT_ENABLED=True)."""
    from app.core.rate_limit import limiter
    assert limiter.enabled is True


def test_limiter_uses_valkey_url():
    """Rate limiter is configured with the VALKEY_URL from settings."""
    from app.core.rate_limit import limiter
    # _storage_uri is the attribute holding the configured URI
    assert limiter._storage_uri == settings.VALKEY_URL


def test_rate_limit_module_exports():
    """rate_limit module exports all required symbols."""
    from app.core import rate_limit as rl
    assert hasattr(rl, "limiter")
    assert hasattr(rl, "get_tenant_id_for_limit")
    assert hasattr(rl, "rate_limit_exceeded_handler")
    assert hasattr(rl, "TIER_LIMITS")
    assert hasattr(rl, "get_tier_limit")


# ---------------------------------------------------------------------------
# get_tenant_id_for_limit key function tests
# ---------------------------------------------------------------------------


def test_get_tenant_id_for_limit_with_tenant():
    """Returns 'tenant:{id}' when request.state.tenant_id is set."""
    from app.core.rate_limit import get_tenant_id_for_limit

    mock_request = MagicMock()
    mock_request.state.tenant_id = "abc-123"

    result = get_tenant_id_for_limit(mock_request)
    assert result == "tenant:abc-123"


def test_get_tenant_id_for_limit_no_tenant():
    """Returns 'ip:{addr}' when no tenant_id on request.state."""
    from app.core.rate_limit import get_tenant_id_for_limit

    mock_request = MagicMock()
    mock_request.state = MagicMock(spec=[])  # No tenant_id attribute

    with patch("app.core.rate_limit.get_remote_address", return_value="127.0.0.1"):
        result = get_tenant_id_for_limit(mock_request)

    assert result == "ip:127.0.0.1"


# ---------------------------------------------------------------------------
# main.py integration: limiter registered on app.state
# ---------------------------------------------------------------------------


def test_app_state_has_limiter():
    """app.state.limiter is registered in main.py."""
    from app.main import app

    assert hasattr(app.state, "limiter"), "app.state.limiter must be set in main.py"
    assert app.state.limiter is not None


def test_app_has_rate_limit_exception_handler():
    """RateLimitExceeded exception handler is registered on the app."""
    from app.main import app
    from slowapi.errors import RateLimitExceeded

    exception_handlers = getattr(app, "exception_handlers", {})
    assert RateLimitExceeded in exception_handlers, (
        "RateLimitExceeded exception handler must be registered in main.py"
    )


# ---------------------------------------------------------------------------
# auth.py routes have @limiter.limit decorators
# ---------------------------------------------------------------------------


def test_login_route_has_rate_limit():
    """Login route function has @limiter.limit applied (5/15minutes).

    slowapi stores route limits keyed by '{module}.{funcname}' in limiter._route_limits.
    We verify the login endpoint's limits include a 5-request limit.
    """
    from app.core.rate_limit import limiter

    route_key = "app.api.auth.login"
    assert route_key in limiter._route_limits, (
        f"login endpoint must be registered in limiter._route_limits. "
        f"Found keys: {list(limiter._route_limits.keys())}"
    )
    limits = limiter._route_limits[route_key]
    assert len(limits) > 0

    # Verify the limit string contains 5 (per 15 minutes)
    limit_strs = [str(lim.limit) for lim in limits]
    assert any("5" in s for s in limit_strs), (
        f"Login should be limited to 5/15minutes, found: {limit_strs}"
    )


def test_register_route_has_rate_limit():
    """Register route function has @limiter.limit applied (10/hour)."""
    from app.core.rate_limit import limiter

    route_key = "app.api.auth.register"
    assert route_key in limiter._route_limits, (
        f"register endpoint must be registered in limiter._route_limits. "
        f"Found keys: {list(limiter._route_limits.keys())}"
    )
    limits = limiter._route_limits[route_key]
    assert len(limits) > 0

    limit_strs = [str(lim.limit) for lim in limits]
    assert any("10" in s for s in limit_strs), (
        f"Register should be limited to 10/hour, found: {limit_strs}"
    )


def test_refresh_route_has_rate_limit():
    """Refresh route function has @limiter.limit applied (30/minute)."""
    from app.core.rate_limit import limiter

    route_key = "app.api.auth.refresh"
    assert route_key in limiter._route_limits, (
        f"refresh endpoint must be registered in limiter._route_limits. "
        f"Found keys: {list(limiter._route_limits.keys())}"
    )
    limits = limiter._route_limits[route_key]
    assert len(limits) > 0
