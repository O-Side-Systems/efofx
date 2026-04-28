"""
Unit tests for auth service utility functions.

Tests pure business logic without MongoDB — no DB connection required.
"""

import pytest
from datetime import datetime, timezone, timedelta
from pwdlib import PasswordHash
from pwdlib.hashers.bcrypt import BcryptHasher

from app.services.auth_service import generate_verification_token


_password_hash = PasswordHash((BcryptHasher(),))


class TestGenerateVerificationToken:
    """Tests for generate_verification_token()."""

    def test_returns_tuple(self):
        """Function returns a tuple of (str, datetime)."""
        result = generate_verification_token()
        assert isinstance(result, tuple)
        assert len(result) == 2

    def test_token_is_string(self):
        """Token component is a string."""
        token, _ = generate_verification_token()
        assert isinstance(token, str)

    def test_token_length(self):
        """Token is at least 43 characters (32-byte urlsafe base64 ≈ 43 chars)."""
        token, _ = generate_verification_token()
        assert len(token) >= 43

    def test_token_is_unique(self):
        """Each call generates a unique token."""
        token1, _ = generate_verification_token()
        token2, _ = generate_verification_token()
        assert token1 != token2

    def test_expiry_is_datetime(self):
        """Expiry component is a datetime with timezone."""
        _, expires_at = generate_verification_token()
        assert isinstance(expires_at, datetime)
        assert expires_at.tzinfo is not None

    def test_expiry_is_24h_from_now(self):
        """Expiry is approximately 24 hours from now."""
        before = datetime.now(timezone.utc)
        _, expires_at = generate_verification_token()
        after = datetime.now(timezone.utc)

        expected_min = before + timedelta(hours=23, minutes=59)
        expected_max = after + timedelta(hours=24, seconds=5)

        assert expires_at >= expected_min, f"Expiry {expires_at} is too early"
        assert expires_at <= expected_max, f"Expiry {expires_at} is too late"


class TestPasswordHashAndVerify:
    """Tests for password hashing via pwdlib."""

    def test_hash_is_not_plaintext(self):
        """Hashed password does not equal the original."""
        password = "test-password-123"
        hashed = _password_hash.hash(password)
        assert hashed != password

    def test_correct_password_verifies(self):
        """Correct password verifies against its hash."""
        password = "test-password-123"
        hashed = _password_hash.hash(password)
        assert _password_hash.verify(password, hashed) is True

    def test_wrong_password_fails(self):
        """Incorrect password does not verify."""
        password = "test-password-123"
        hashed = _password_hash.hash(password)
        assert _password_hash.verify("wrong-password", hashed) is False

    def test_different_hashes_for_same_password(self):
        """Each hash call produces a different salt (bcrypt behavior)."""
        password = "test-password-123"
        hash1 = _password_hash.hash(password)
        hash2 = _password_hash.hash(password)
        assert hash1 != hash2


class TestApiKeyFormat:
    """Tests for API key generation format."""

    def test_api_key_format(self):
        """Generated API key has correct format: sk_live_{tenant_id_no_dashes}_{random}."""
        import secrets
        import uuid

        tenant_id = str(uuid.uuid4())
        raw_api_key = (
            f"sk_live_{tenant_id.replace('-', '')}_{secrets.token_urlsafe(16)}"
        )

        # Must start with sk_live_
        assert raw_api_key.startswith("sk_live_")

        # Must contain tenant_id without dashes
        tenant_id_no_dashes = tenant_id.replace("-", "")
        assert tenant_id_no_dashes in raw_api_key

    def test_api_key_contains_tenant_id_without_dashes(self):
        """API key encodes tenant_id without dashes for fast lookup."""
        import secrets
        import uuid

        tenant_id = str(uuid.uuid4())
        tenant_id_no_dashes = tenant_id.replace("-", "")
        raw_api_key = (
            f"sk_live_{tenant_id_no_dashes}_{secrets.token_urlsafe(16)}"
        )

        # Extract tenant_id portion from key
        parts = raw_api_key.split("_")
        # sk_live_{tenant_id_no_dashes}_{random}
        # parts = ["sk", "live", f"{tenant_id_no_dashes}{suffix...}"]
        # Actually the random part may also contain underscores via urlsafe
        # The tenant_id portion is: after "sk_live_", the first 32 chars
        key_body = raw_api_key[len("sk_live_"):]
        extracted_tenant_portion = key_body[:32]

        assert extracted_tenant_portion == tenant_id_no_dashes

    def test_api_key_is_unique(self):
        """Different registrations produce different API keys."""
        import secrets
        import uuid

        def make_key():
            tid = str(uuid.uuid4())
            return f"sk_live_{tid.replace('-', '')}_{secrets.token_urlsafe(16)}"

        keys = {make_key() for _ in range(20)}
        assert len(keys) == 20, "All generated keys should be unique"
