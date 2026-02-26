"""
Tests for auth API endpoints: registration, email verification, profile management.

Uses per-test Motor client pattern (avoids event-loop conflicts with session-scoped
fixtures in asyncio strict mode). SMTP is mocked to prevent real email sends.
"""

import pytest
import pytest_asyncio
from unittest.mock import AsyncMock, patch
from datetime import datetime, timezone, timedelta
from motor.motor_asyncio import AsyncIOMotorClient
from httpx import AsyncClient, ASGITransport

from app.core.config import settings
from app.core.constants import DB_COLLECTIONS

# Use a dedicated test database name to avoid contaminating real data
TEST_DB = "efofx_estimate_test_auth"


@pytest_asyncio.fixture
async def mongo_client():
    """Create an isolated Motor client for each test."""
    client = AsyncIOMotorClient(settings.MONGO_URI)
    yield client
    # Drop test database after each test for isolation
    await client.drop_database(TEST_DB)
    client.close()


@pytest_asyncio.fixture
async def test_app(mongo_client):
    """Set up the FastAPI app with the test database injected."""
    import app.db.mongodb as _mdb

    db = mongo_client[TEST_DB]
    _mdb._client = mongo_client
    _mdb._database = db

    # Yield the FastAPI app
    from app.main import app
    yield app

    # Restore (reset for next test)
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
# Helper fixtures
# ---------------------------------------------------------------------------

VALID_REGISTER_PAYLOAD = {
    "company_name": "Acme Builders",
    "email": "contractor@acme.example",
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


# ---------------------------------------------------------------------------
# Registration tests
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_register_success(client):
    """POST /auth/register with valid data returns 201 with tenant_id and api_key."""
    response = await _register(client)

    assert response.status_code == 201
    data = response.json()
    assert "tenant_id" in data
    assert "api_key" in data
    assert data["api_key"].startswith("sk_live_")
    assert "message" in data
    assert data["tenant_id"]  # non-empty


@pytest.mark.asyncio
async def test_register_duplicate_email(client):
    """Register twice with same email returns 201 with generic message (no enumeration)."""
    await _register(client)

    # Second registration with same email
    response = await _register(client)

    assert response.status_code == 201
    data = response.json()
    # Generic message — same response to prevent enumeration
    assert "message" in data
    assert data["tenant_id"]  # still returns a tenant_id (existing one)


@pytest.mark.asyncio
async def test_register_invalid_email(client):
    """Invalid email format returns 422."""
    payload = {**VALID_REGISTER_PAYLOAD, "email": "not-an-email"}
    response = await client.post("/api/v1/auth/register", json=payload)
    assert response.status_code == 422


@pytest.mark.asyncio
async def test_register_short_password(client):
    """Password shorter than 8 chars returns 422."""
    payload = {**VALID_REGISTER_PAYLOAD, "password": "short"}
    response = await client.post("/api/v1/auth/register", json=payload)
    assert response.status_code == 422


@pytest.mark.asyncio
async def test_register_short_company_name(client):
    """Company name shorter than 2 chars returns 422."""
    payload = {**VALID_REGISTER_PAYLOAD, "company_name": "X"}
    response = await client.post("/api/v1/auth/register", json=payload)
    assert response.status_code == 422


# ---------------------------------------------------------------------------
# Email verification tests
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_email_verification_success(client, mongo_client):
    """Register, extract token from DB, verify via GET /auth/verify?token=... -> email_verified=True."""
    reg_response = await _register(client)
    assert reg_response.status_code == 201
    tenant_id = reg_response.json()["tenant_id"]

    # Extract verification token from DB
    db = mongo_client[TEST_DB]
    token_doc = await db[DB_COLLECTIONS["VERIFICATION_TOKENS"]].find_one(
        {"tenant_id": tenant_id}
    )
    assert token_doc is not None, "Verification token was not stored in DB"

    token = token_doc["token"]

    # Verify email
    verify_response = await client.get(f"/api/v1/auth/verify?token={token}")
    assert verify_response.status_code == 200
    data = verify_response.json()
    assert data["email_verified"] is True
    assert "message" in data

    # Token should be deleted (single-use)
    deleted_doc = await db[DB_COLLECTIONS["VERIFICATION_TOKENS"]].find_one(
        {"tenant_id": tenant_id}
    )
    assert deleted_doc is None, "Verification token should be deleted after use"

    # Tenant should be marked as verified
    tenant_doc = await db[DB_COLLECTIONS["TENANTS"]].find_one(
        {"tenant_id": tenant_id}
    )
    assert tenant_doc["email_verified"] is True


@pytest.mark.asyncio
async def test_email_verification_invalid_token(client):
    """GET /auth/verify with invalid token returns 400."""
    response = await client.get("/api/v1/auth/verify?token=totally_invalid_token_xyz")
    assert response.status_code == 400


@pytest.mark.asyncio
async def test_email_verification_expired_token(client, mongo_client):
    """Expired verification token returns 400."""
    reg_response = await _register(client)
    assert reg_response.status_code == 201
    tenant_id = reg_response.json()["tenant_id"]

    # Manually expire the token
    db = mongo_client[TEST_DB]
    past = datetime.now(timezone.utc) - timedelta(hours=1)
    await db[DB_COLLECTIONS["VERIFICATION_TOKENS"]].update_one(
        {"tenant_id": tenant_id},
        {"$set": {"expires_at": past}},
    )

    token_doc = await db[DB_COLLECTIONS["VERIFICATION_TOKENS"]].find_one(
        {"tenant_id": tenant_id}
    )
    token = token_doc["token"]

    response = await client.get(f"/api/v1/auth/verify?token={token}")
    assert response.status_code == 400


# ---------------------------------------------------------------------------
# API key behavior tests
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_api_key_shown_once(client, mongo_client):
    """Registration response includes api_key; profile endpoint only shows masked version."""
    reg_response = await _register(client)
    assert reg_response.status_code == 201

    api_key = reg_response.json()["api_key"]
    tenant_id = reg_response.json()["tenant_id"]
    assert api_key.startswith("sk_live_")

    # Verify email so we can call profile
    db = mongo_client[TEST_DB]
    token_doc = await db[DB_COLLECTIONS["VERIFICATION_TOKENS"]].find_one(
        {"tenant_id": tenant_id}
    )
    if token_doc:
        await client.get(f"/api/v1/auth/verify?token={token_doc['token']}")

    # Profile endpoint should only return masked key, not the full key
    profile_response = await client.get(
        "/api/v1/auth/profile",
        headers={"Authorization": f"Bearer {api_key}"},
    )
    assert profile_response.status_code == 200
    profile = profile_response.json()
    assert "masked_api_key" in profile
    # Masked key should not equal the full key
    assert profile["masked_api_key"] != api_key
    assert profile["masked_api_key"].startswith("sk-...")
    # Full api_key is NOT in the profile response
    assert api_key not in str(profile)


@pytest.mark.asyncio
async def test_unverified_tenant_blocked(client):
    """Unverified tenant calling profile endpoint returns 403."""
    reg_response = await _register(client)
    assert reg_response.status_code == 201

    api_key = reg_response.json()["api_key"]

    # Don't verify email — try to access profile
    profile_response = await client.get(
        "/api/v1/auth/profile",
        headers={"Authorization": f"Bearer {api_key}"},
    )
    assert profile_response.status_code == 403


# ---------------------------------------------------------------------------
# Profile management tests
# ---------------------------------------------------------------------------


@pytest_asyncio.fixture
async def verified_tenant(client, mongo_client):
    """Fixture: register and verify a tenant, return (api_key, tenant_id)."""
    reg_response = await _register(client)
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


@pytest.mark.asyncio
async def test_update_profile(client, verified_tenant):
    """PATCH /auth/profile with valid JWT -> 200, updated fields."""
    api_key, tenant_id = verified_tenant

    update_payload = {
        "company_name": "New Name Builders",
        "settings": {"branding_color": "#FF5733"},
    }
    response = await client.patch(
        "/api/v1/auth/profile",
        json=update_payload,
        headers={"Authorization": f"Bearer {api_key}"},
    )
    assert response.status_code == 200
    data = response.json()
    assert data["company_name"] == "New Name Builders"


@pytest.mark.asyncio
async def test_get_profile(client, verified_tenant):
    """GET /auth/profile returns correct tenant details."""
    api_key, tenant_id = verified_tenant

    response = await client.get(
        "/api/v1/auth/profile",
        headers={"Authorization": f"Bearer {api_key}"},
    )
    assert response.status_code == 200
    data = response.json()
    assert data["tenant_id"] == tenant_id
    assert data["email"] == VALID_REGISTER_PAYLOAD["email"]
    assert data["tier"] == "trial"
    assert data["email_verified"] is True
    assert "masked_api_key" in data
    assert "has_openai_key" in data
    assert data["has_openai_key"] is False


@pytest.mark.asyncio
async def test_profile_requires_auth(client):
    """GET /auth/profile without auth returns 403 or 401."""
    response = await client.get("/api/v1/auth/profile")
    assert response.status_code in (401, 403)
