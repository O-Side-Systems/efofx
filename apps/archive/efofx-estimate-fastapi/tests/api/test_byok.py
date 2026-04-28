"""
Tests for BYOK (Bring Your Own Key) OpenAI key management endpoints.

Tests cover PUT /auth/openai-key (store/rotate) and GET /auth/openai-key/status.
OpenAI API calls are mocked with unittest.mock.patch to avoid real network calls.

Uses per-test Motor client pattern (avoids event-loop conflicts in asyncio strict mode).
"""

from datetime import datetime, timedelta, timezone
from unittest.mock import AsyncMock, MagicMock, patch

import jwt  # PyJWT 2.x
import pytest
import pytest_asyncio
from httpx import ASGITransport, AsyncClient
from motor.motor_asyncio import AsyncIOMotorClient
from openai import AuthenticationError

from app.core.config import settings
from app.core.constants import DB_COLLECTIONS
from app.utils.crypto import decrypt_openai_key

# Dedicated test database — never contaminates real data
TEST_DB = "efofx_estimate_test_byok"


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest_asyncio.fixture
async def mongo_client():
    """Create an isolated Motor client for each test."""
    client = AsyncIOMotorClient(settings.MONGO_URI)
    yield client
    await client.drop_database(TEST_DB)
    client.close()


@pytest_asyncio.fixture
async def test_app(mongo_client):
    """Set up the FastAPI app with the test database injected."""
    import app.db.mongodb as _mdb

    db = mongo_client[TEST_DB]
    _mdb._client = mongo_client
    _mdb._database = db

    from app.main import app
    yield app

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
    "company_name": "BYOK Test Builders",
    "email": "byok-contractor@example.com",
    "password": "secure-password-123",
}

VALID_OPENAI_KEY = "sk-proj-test1234567890"


async def _register_and_verify(client, mongo_client, payload=None):
    """Register and verify a tenant. Returns (api_key, tenant_id)."""
    payload = payload or VALID_REGISTER_PAYLOAD.copy()
    with patch(
        "app.services.auth_service.send_verification_email",
        new_callable=AsyncMock,
    ):
        reg_response = await client.post("/api/v1/auth/register", json=payload)
    assert reg_response.status_code == 201
    tenant_id = reg_response.json()["tenant_id"]
    api_key = reg_response.json()["api_key"]

    db = mongo_client[TEST_DB]
    token_doc = await db[DB_COLLECTIONS["VERIFICATION_TOKENS"]].find_one(
        {"tenant_id": tenant_id}
    )
    if token_doc:
        await client.get(f"/api/v1/auth/verify?token={token_doc['token']}")

    return api_key, tenant_id


def _make_bearer(api_key: str) -> dict:
    """Build Authorization header dict."""
    return {"Authorization": f"Bearer {api_key}"}


def _make_valid_jwt(tenant_id: str) -> str:
    """Mint a valid short-lived JWT for testing authenticated endpoints."""
    now = datetime.now(timezone.utc)
    payload = {
        "sub": tenant_id,
        "tenant_id": tenant_id,
        "role": "owner",
        "iat": now,
        "exp": now + timedelta(minutes=20),
    }
    return jwt.encode(payload, settings.JWT_SECRET_KEY, algorithm="HS256")


def _mock_openai_success():
    """Return a context manager that mocks AsyncOpenAI to succeed on models.list()."""
    mock_client = MagicMock()
    mock_client.models = MagicMock()
    mock_client.models.list = AsyncMock(return_value=MagicMock())
    return patch("app.services.byok_service.AsyncOpenAI", return_value=mock_client)


def _mock_openai_auth_error():
    """Return a context manager that mocks AsyncOpenAI to raise AuthenticationError."""
    mock_client = MagicMock()

    # Construct a minimal AuthenticationError (httpx Response not required for the mock)
    auth_error = AuthenticationError.__new__(AuthenticationError)
    auth_error.message = "Incorrect API key"
    auth_error.body = {"error": {"message": "Incorrect API key"}}
    auth_error.status_code = 401
    auth_error.request = MagicMock()
    auth_error.response = MagicMock(status_code=401)

    mock_client.models = MagicMock()
    mock_client.models.list = AsyncMock(side_effect=auth_error)
    return patch("app.services.byok_service.AsyncOpenAI", return_value=mock_client)


def _mock_openai_connection_error():
    """Return a context manager that mocks AsyncOpenAI to raise a generic exception."""
    mock_client = MagicMock()
    mock_client.models = MagicMock()
    mock_client.models.list = AsyncMock(side_effect=Exception("Connection refused"))
    return patch("app.services.byok_service.AsyncOpenAI", return_value=mock_client)


# ---------------------------------------------------------------------------
# PUT /auth/openai-key tests
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_store_openai_key_success(client, mongo_client):
    """Authenticated verified tenant stores key -> 200, returns masked key."""
    api_key, tenant_id = await _register_and_verify(client, mongo_client)

    with _mock_openai_success():
        response = await client.put(
            "/api/v1/auth/openai-key",
            json={"openai_key": VALID_OPENAI_KEY},
            headers=_make_bearer(api_key),
        )

    assert response.status_code == 200
    data = response.json()
    assert "masked_key" in data
    assert data["masked_key"].startswith("sk-...")
    assert data["masked_key"] == f"sk-...{VALID_OPENAI_KEY[-6:]}"
    assert "message" in data
    assert "stored" in data["message"].lower()


@pytest.mark.asyncio
async def test_store_openai_key_invalid(client, mongo_client):
    """Mock OpenAI to raise AuthenticationError -> 400 'Invalid OpenAI API key'."""
    api_key, _ = await _register_and_verify(client, mongo_client)

    with _mock_openai_auth_error():
        response = await client.put(
            "/api/v1/auth/openai-key",
            json={"openai_key": "sk-proj-invalidkey00"},
            headers=_make_bearer(api_key),
        )

    assert response.status_code == 400
    assert "Invalid OpenAI API key" in response.json()["detail"]


@pytest.mark.asyncio
async def test_store_openai_key_openai_unavailable(client, mongo_client):
    """Mock OpenAI to raise connection error -> 503."""
    api_key, _ = await _register_and_verify(client, mongo_client)

    with _mock_openai_connection_error():
        response = await client.put(
            "/api/v1/auth/openai-key",
            json={"openai_key": "sk-proj-somekeyhere1"},
            headers=_make_bearer(api_key),
        )

    assert response.status_code == 503
    assert "Could not validate" in response.json()["detail"]


@pytest.mark.asyncio
async def test_key_rotation(client, mongo_client):
    """Store key A, then store key B -> B replaces A; decrypt returns B."""
    api_key, tenant_id = await _register_and_verify(client, mongo_client)

    key_a = "sk-proj-aaaabbbb1234"
    key_b = "sk-proj-bbbbcccc5678"

    # Store key A
    with _mock_openai_success():
        resp_a = await client.put(
            "/api/v1/auth/openai-key",
            json={"openai_key": key_a},
            headers=_make_bearer(api_key),
        )
    assert resp_a.status_code == 200

    # Store key B (rotation)
    with _mock_openai_success():
        resp_b = await client.put(
            "/api/v1/auth/openai-key",
            json={"openai_key": key_b},
            headers=_make_bearer(api_key),
        )
    assert resp_b.status_code == 200
    assert resp_b.json()["masked_key"] == f"sk-...{key_b[-6:]}"

    # Verify DB has B's ciphertext (not A's)
    db = mongo_client[TEST_DB]
    tenant_doc = await db[DB_COLLECTIONS["TENANTS"]].find_one({"tenant_id": tenant_id})
    assert tenant_doc["encrypted_openai_key"] is not None
    # Decrypt and confirm it is key B
    master_key = settings.MASTER_ENCRYPTION_KEY.encode()
    decrypted = decrypt_openai_key(master_key, tenant_id, tenant_doc["encrypted_openai_key"])
    assert decrypted == key_b


@pytest.mark.asyncio
async def test_no_key_returns_402(client, mongo_client):
    """Tenant with no stored key gets 402 when decrypt_tenant_openai_key is called."""
    from app.services.byok_service import decrypt_tenant_openai_key

    api_key, tenant_id = await _register_and_verify(client, mongo_client)

    # Inject the test DB for the direct service call
    import app.db.mongodb as _mdb
    db = mongo_client[TEST_DB]
    _mdb._database = db

    with pytest.raises(Exception) as exc_info:
        await decrypt_tenant_openai_key(tenant_id)

    # Should raise HTTPException with status 402
    assert exc_info.value.status_code == 402
    assert "OpenAI API key required" in exc_info.value.detail


@pytest.mark.asyncio
async def test_unauthenticated_cannot_store_key(client):
    """No JWT/API key -> 401."""
    response = await client.put(
        "/api/v1/auth/openai-key",
        json={"openai_key": VALID_OPENAI_KEY},
    )
    assert response.status_code == 401


@pytest.mark.asyncio
async def test_unverified_cannot_store_key(client, mongo_client):
    """Unverified tenant (no email verification) -> 403."""
    with patch(
        "app.services.auth_service.send_verification_email",
        new_callable=AsyncMock,
    ):
        reg_response = await client.post(
            "/api/v1/auth/register",
            json=VALID_REGISTER_PAYLOAD,
        )
    assert reg_response.status_code == 201
    api_key = reg_response.json()["api_key"]

    # Do NOT verify email — use the raw API key
    response = await client.put(
        "/api/v1/auth/openai-key",
        json={"openai_key": VALID_OPENAI_KEY},
        headers=_make_bearer(api_key),
    )
    assert response.status_code == 403


@pytest.mark.asyncio
async def test_store_openai_key_too_short_rejected(client, mongo_client):
    """Key shorter than 10 chars -> 422 (Pydantic validation)."""
    api_key, _ = await _register_and_verify(client, mongo_client)

    response = await client.put(
        "/api/v1/auth/openai-key",
        json={"openai_key": "short"},
        headers=_make_bearer(api_key),
    )
    assert response.status_code == 422


# ---------------------------------------------------------------------------
# GET /auth/openai-key/status tests
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_masked_key_in_status(client, mongo_client):
    """Store a key, GET /auth/openai-key/status -> has_key=True, masked_key correct."""
    api_key, _ = await _register_and_verify(client, mongo_client)

    with _mock_openai_success():
        await client.put(
            "/api/v1/auth/openai-key",
            json={"openai_key": VALID_OPENAI_KEY},
            headers=_make_bearer(api_key),
        )

    status_response = await client.get(
        "/api/v1/auth/openai-key/status",
        headers=_make_bearer(api_key),
    )
    assert status_response.status_code == 200
    data = status_response.json()
    assert data["has_key"] is True
    assert data["masked_key"] is not None
    assert data["masked_key"].startswith("sk-...")
    # Last 6 chars of VALID_OPENAI_KEY
    assert data["masked_key"].endswith(VALID_OPENAI_KEY[-6:])


@pytest.mark.asyncio
async def test_key_status_no_key(client, mongo_client):
    """GET /auth/openai-key/status without storing a key -> has_key=False, masked_key=None."""
    api_key, _ = await _register_and_verify(client, mongo_client)

    status_response = await client.get(
        "/api/v1/auth/openai-key/status",
        headers=_make_bearer(api_key),
    )
    assert status_response.status_code == 200
    data = status_response.json()
    assert data["has_key"] is False
    assert data["masked_key"] is None


@pytest.mark.asyncio
async def test_key_status_unauthenticated(client):
    """GET /auth/openai-key/status without auth -> 401."""
    response = await client.get("/api/v1/auth/openai-key/status")
    assert response.status_code == 401
