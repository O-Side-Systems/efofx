"""
Tests for auth API endpoints: registration, email verification, profile management,
login (JWT), and token refresh.

Uses per-test Motor client pattern (avoids event-loop conflicts with session-scoped
fixtures in asyncio strict mode). SMTP is mocked to prevent real email sends.
"""

import hashlib
from datetime import datetime, timedelta, timezone
from unittest.mock import AsyncMock, patch

import jwt  # PyJWT 2.x
import pytest
import pytest_asyncio
from httpx import ASGITransport, AsyncClient
from motor.motor_asyncio import AsyncIOMotorClient

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
# Helper functions
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


async def _register_and_verify(client, mongo_client, payload=None):
    """Register a tenant and verify their email. Returns (api_key, tenant_id)."""
    reg_response = await _register(client, payload)
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


async def _login(client, email=None, password=None):
    """Log in and return the HTTP response."""
    return await client.post(
        "/api/v1/auth/login",
        json={
            "email": email or VALID_REGISTER_PAYLOAD["email"],
            "password": password or VALID_REGISTER_PAYLOAD["password"],
        },
    )


async def _login_and_get_tokens(client, mongo_client):
    """Register, verify, and log in. Returns (access_token, refresh_token, tenant_id)."""
    _, tenant_id = await _register_and_verify(client, mongo_client)
    login_response = await _login(client)
    assert login_response.status_code == 200
    data = login_response.json()
    return data["access_token"], data["refresh_token"], tenant_id


# ---------------------------------------------------------------------------
# Registration tests (preserved from plan 02-01)
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
# Email verification tests (preserved from plan 02-01)
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
# API key behavior tests (preserved from plan 02-01)
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
async def test_unverified_tenant_api_key_blocked(client):
    """Unverified tenant calling profile endpoint with API key returns 403."""
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
# Profile management tests (preserved from plan 02-01)
# ---------------------------------------------------------------------------


@pytest_asyncio.fixture
async def verified_tenant(client, mongo_client):
    """Fixture: register and verify a tenant, return (api_key, tenant_id)."""
    api_key, tenant_id = await _register_and_verify(client, mongo_client)
    return api_key, tenant_id


@pytest.mark.asyncio
async def test_update_profile(client, verified_tenant):
    """PATCH /auth/profile with valid API key -> 200, updated fields."""
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


# ---------------------------------------------------------------------------
# Login tests
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_login_success(client, mongo_client):
    """Register, verify email, login -> 200 with access_token and refresh_token."""
    await _register_and_verify(client, mongo_client)
    response = await _login(client)

    assert response.status_code == 200
    data = response.json()
    assert "access_token" in data
    assert "refresh_token" in data
    assert data["token_type"] == "bearer"
    assert data["expires_in"] == 1200  # 20 minutes in seconds
    assert data["access_token"]  # non-empty
    assert data["refresh_token"]  # non-empty


@pytest.mark.asyncio
async def test_login_wrong_password(client, mongo_client):
    """Register, verify, login with wrong password -> 401 'Invalid credentials'."""
    await _register_and_verify(client, mongo_client)
    response = await _login(client, password="wrong-password-!!")

    assert response.status_code == 401
    assert response.json()["detail"] == "Invalid credentials"


@pytest.mark.asyncio
async def test_login_nonexistent_email(client):
    """Login with unknown email -> 401 'Invalid credentials' (no enumeration)."""
    response = await _login(client, email="nobody@nowhere.example")

    assert response.status_code == 401
    assert response.json()["detail"] == "Invalid credentials"


@pytest.mark.asyncio
async def test_login_unverified_tenant(client):
    """Register (no verify), login -> 403 'Email not verified'."""
    await _register(client)
    response = await _login(client)

    assert response.status_code == 403
    assert "not verified" in response.json()["detail"].lower()


@pytest.mark.asyncio
async def test_login_inactive_tenant(client, mongo_client):
    """Deactivated tenant login -> 401 'Invalid credentials'."""
    _, tenant_id = await _register_and_verify(client, mongo_client)

    # Deactivate the tenant directly in DB
    db = mongo_client[TEST_DB]
    await db[DB_COLLECTIONS["TENANTS"]].update_one(
        {"tenant_id": tenant_id},
        {"$set": {"is_active": False}},
    )

    response = await _login(client)
    assert response.status_code == 401
    assert response.json()["detail"] == "Invalid credentials"


# ---------------------------------------------------------------------------
# JWT claims validation tests
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_jwt_claims_complete(client, mongo_client):
    """Decode the access_token from login, verify it contains required claims."""
    await _register_and_verify(client, mongo_client)
    login_response = await _login(client)
    access_token = login_response.json()["access_token"]

    payload = jwt.decode(
        access_token,
        settings.JWT_SECRET_KEY,
        algorithms=["HS256"],
    )

    assert "sub" in payload
    assert "tenant_id" in payload
    assert "role" in payload
    assert "exp" in payload
    assert "iat" in payload
    assert payload["role"] == "owner"
    # For single-user tenants, sub == tenant_id
    assert payload["sub"] == payload["tenant_id"]


@pytest.mark.asyncio
async def test_expired_token_returns_401(client, mongo_client):
    """Create token with past expiry, call protected endpoint -> 401 'Token expired'."""
    _, tenant_id = await _register_and_verify(client, mongo_client)

    # Mint a token that is already expired
    past = datetime.now(timezone.utc) - timedelta(minutes=5)
    payload = {
        "sub": tenant_id,
        "tenant_id": tenant_id,
        "role": "owner",
        "iat": past - timedelta(minutes=20),
        "exp": past,
    }
    expired_token = jwt.encode(payload, settings.JWT_SECRET_KEY, algorithm="HS256")

    response = await client.get(
        "/api/v1/auth/profile",
        headers={"Authorization": f"Bearer {expired_token}"},
    )
    assert response.status_code == 401
    assert response.json()["detail"] == "Token expired"


@pytest.mark.asyncio
async def test_missing_token_returns_401(client):
    """Call protected endpoint with no Authorization header -> 401 'Authentication required'."""
    response = await client.get("/api/v1/auth/profile")
    assert response.status_code == 401
    assert response.json()["detail"] == "Authentication required"


@pytest.mark.asyncio
async def test_invalid_token_returns_401(client):
    """Call protected endpoint with garbage token -> 401 'Invalid token'."""
    response = await client.get(
        "/api/v1/auth/profile",
        headers={"Authorization": "Bearer this.is.garbage"},
    )
    assert response.status_code == 401
    assert response.json()["detail"] == "Invalid token"


@pytest.mark.asyncio
async def test_unverified_tenant_jwt_blocked(client, mongo_client):
    """Create valid JWT for unverified tenant, call protected endpoint -> 403."""
    # Register but do NOT verify email
    reg_response = await _register(client)
    assert reg_response.status_code == 201
    tenant_id = reg_response.json()["tenant_id"]

    # Manually mint a valid JWT for the unverified tenant
    now = datetime.now(timezone.utc)
    payload = {
        "sub": tenant_id,
        "tenant_id": tenant_id,
        "role": "owner",
        "iat": now,
        "exp": now + timedelta(minutes=20),
    }
    valid_token = jwt.encode(payload, settings.JWT_SECRET_KEY, algorithm="HS256")

    response = await client.get(
        "/api/v1/auth/profile",
        headers={"Authorization": f"Bearer {valid_token}"},
    )
    assert response.status_code == 403


# ---------------------------------------------------------------------------
# Token refresh tests
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_refresh_token_success(client, mongo_client):
    """Login, use refresh_token to get new access_token -> 200."""
    access_token, refresh_token, _ = await _login_and_get_tokens(client, mongo_client)

    response = await client.post(
        "/api/v1/auth/refresh",
        json={"refresh_token": refresh_token},
    )
    assert response.status_code == 200
    data = response.json()
    assert "access_token" in data
    assert data["token_type"] == "bearer"
    assert data["expires_in"] == 1200
    # New access token must be non-empty and decodable with the correct claims
    assert data["access_token"]
    payload = jwt.decode(
        data["access_token"],
        settings.JWT_SECRET_KEY,
        algorithms=["HS256"],
    )
    assert "tenant_id" in payload
    assert "sub" in payload
    assert "role" in payload


@pytest.mark.asyncio
async def test_refresh_token_rotation(client, mongo_client):
    """After refresh, old refresh_token no longer works -> 401."""
    _, refresh_token, _ = await _login_and_get_tokens(client, mongo_client)

    # First use succeeds and rotates the token
    first_response = await client.post(
        "/api/v1/auth/refresh",
        json={"refresh_token": refresh_token},
    )
    assert first_response.status_code == 200

    # Second use of the SAME old refresh token must fail
    second_response = await client.post(
        "/api/v1/auth/refresh",
        json={"refresh_token": refresh_token},
    )
    assert second_response.status_code == 401


@pytest.mark.asyncio
async def test_refresh_token_invalid(client):
    """POST /auth/refresh with invalid token -> 401."""
    response = await client.post(
        "/api/v1/auth/refresh",
        json={"refresh_token": "totally-invalid-refresh-token"},
    )
    assert response.status_code == 401


@pytest.mark.asyncio
async def test_refresh_token_expired(client, mongo_client):
    """Expired refresh token -> 401."""
    _, refresh_token, _ = await _login_and_get_tokens(client, mongo_client)

    # Manually expire the refresh token in DB
    token_hash = hashlib.sha256(refresh_token.encode()).hexdigest()
    db = mongo_client[TEST_DB]
    past = datetime.now(timezone.utc) - timedelta(days=1)
    await db[DB_COLLECTIONS["REFRESH_TOKENS"]].update_one(
        {"token_hash": token_hash},
        {"$set": {"expires_at": past}},
    )

    response = await client.post(
        "/api/v1/auth/refresh",
        json={"refresh_token": refresh_token},
    )
    assert response.status_code == 401


# ---------------------------------------------------------------------------
# API key auth tests
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_api_key_auth_success(client, mongo_client):
    """Use raw API key from registration as Bearer token on protected endpoint -> 200."""
    api_key, tenant_id = await _register_and_verify(client, mongo_client)

    response = await client.get(
        "/api/v1/auth/profile",
        headers={"Authorization": f"Bearer {api_key}"},
    )
    assert response.status_code == 200
    data = response.json()
    assert data["tenant_id"] == tenant_id


@pytest.mark.asyncio
async def test_api_key_auth_invalid(client, mongo_client):
    """Use wrong API key -> 401."""
    # Register so there is at least one tenant to avoid empty DB edge case
    await _register_and_verify(client, mongo_client)

    # Construct a fake key with valid structure but wrong secret suffix
    fake_key = "sk_live_" + "a" * 32 + "_fakerandombytes"
    response = await client.get(
        "/api/v1/auth/profile",
        headers={"Authorization": f"Bearer {fake_key}"},
    )
    assert response.status_code == 401
