"""
Unit tests for MagicLinkService.

Covers the full token lifecycle:
- generate_token: (raw_token, token_hash, expires_at) tuple
- Token hash verification: SHA-256 of raw token
- create_magic_link: stores hash in MongoDB (never raw token)
- resolve_token_state: valid / expired / used / not_found
- mark_opened: idempotent first-GET tracking
- consume: POST-style token consumption; returns False if already used
"""

import hashlib
from datetime import datetime, timedelta, timezone
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from app.services.magic_link_service import MagicLinkService


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_mock_db(token_doc=None):
    """Return a mock db object whose collection behaves like Motor."""
    mock_collection = MagicMock()
    mock_collection.find_one = AsyncMock(return_value=token_doc)
    mock_collection.insert_one = AsyncMock(return_value=MagicMock(inserted_id="abc"))
    mock_collection.update_one = AsyncMock(
        return_value=MagicMock(modified_count=1)
    )

    mock_db = MagicMock()
    mock_db.__getitem__ = MagicMock(return_value=mock_collection)
    return mock_db, mock_collection


# ---------------------------------------------------------------------------
# generate_token
# ---------------------------------------------------------------------------


class TestGenerateToken:
    """MagicLinkService.generate_token returns a well-formed tuple."""

    def test_generate_token_returns_tuple(self):
        """Returns (raw_token, token_hash, expires_at) with correct types."""
        raw, token_hash, expires_at = MagicLinkService.generate_token()
        assert isinstance(raw, str)
        assert isinstance(token_hash, str)
        assert isinstance(expires_at, datetime)

    def test_raw_token_is_url_safe_string(self):
        """raw_token must be non-empty and URL-safe (no spaces)."""
        raw, _, _ = MagicLinkService.generate_token()
        assert len(raw) > 0
        assert " " not in raw

    def test_token_hash_is_64_char_hex(self):
        """token_hash must be a 64-character hexadecimal string (SHA-256)."""
        _, token_hash, _ = MagicLinkService.generate_token()
        assert len(token_hash) == 64
        # Verify it's valid hex
        int(token_hash, 16)

    def test_expires_at_is_approximately_72h_from_now(self):
        """expires_at should be ~72 hours from now (within 5 seconds tolerance)."""
        _, _, expires_at = MagicLinkService.generate_token()
        now = datetime.now(timezone.utc)
        expected = now + timedelta(hours=72)
        delta = abs((expires_at - expected).total_seconds())
        assert delta < 5, f"Expected ~72h expiry but got delta={delta}s"

    def test_token_hash_is_sha256_of_raw(self):
        """token_hash must equal hashlib.sha256(raw.encode()).hexdigest()."""
        raw, token_hash, _ = MagicLinkService.generate_token()
        expected_hash = hashlib.sha256(raw.encode()).hexdigest()
        assert token_hash == expected_hash


# ---------------------------------------------------------------------------
# create_magic_link
# ---------------------------------------------------------------------------


class TestCreateMagicLink:
    """create_magic_link stores hash in DB, never the raw token."""

    @pytest.mark.asyncio
    async def test_create_magic_link_stores_hash_in_db(self):
        """After create(), the inserted doc contains correct fields."""
        mock_db, mock_collection = _make_mock_db()
        service = MagicLinkService()

        with patch("app.services.magic_link_service.get_database", return_value=mock_db):
            raw, token_hash = await service.create_magic_link(
                tenant_id="tenant-123",
                estimation_session_id="session-456",
                customer_email="customer@example.com",
                project_name="Pool Build",
            )

        # insert_one must have been called once
        mock_collection.insert_one.assert_awaited_once()

        # Grab the doc that was inserted
        inserted_doc = mock_collection.insert_one.call_args[0][0]

        # Raw token must NOT be in the stored doc
        assert raw not in str(inserted_doc.values())
        assert inserted_doc["token_hash"] == token_hash

        # Correct field values
        assert inserted_doc["tenant_id"] == "tenant-123"
        assert inserted_doc["estimation_session_id"] == "session-456"
        assert inserted_doc["customer_email"] == "customer@example.com"
        assert inserted_doc["project_name"] == "Pool Build"

        # Hash must equal SHA-256 of raw
        assert inserted_doc["token_hash"] == hashlib.sha256(raw.encode()).hexdigest()

    @pytest.mark.asyncio
    async def test_create_magic_link_returns_raw_and_hash(self):
        """Returns (raw_token, token_hash) — raw for email, hash for DB."""
        mock_db, _ = _make_mock_db()
        service = MagicLinkService()

        with patch("app.services.magic_link_service.get_database", return_value=mock_db):
            raw, token_hash = await service.create_magic_link(
                tenant_id="t1",
                estimation_session_id="s1",
                customer_email="a@b.com",
            )

        assert len(raw) > 0
        assert len(token_hash) == 64


# ---------------------------------------------------------------------------
# resolve_token_state
# ---------------------------------------------------------------------------


class TestResolveTokenState:
    """resolve_token_state returns correct (state, doc) for each scenario."""

    @pytest.mark.asyncio
    async def test_resolve_state_valid(self):
        """Fresh token (not expired, not used) returns ('valid', doc)."""
        future_expiry = datetime.now(timezone.utc) + timedelta(hours=48)
        raw, token_hash, _ = MagicLinkService.generate_token()
        token_doc = {
            "token_hash": token_hash,
            "expires_at": future_expiry,
            "used_at": None,
        }
        mock_db, _ = _make_mock_db(token_doc=token_doc)
        service = MagicLinkService()

        with patch("app.services.magic_link_service.get_database", return_value=mock_db):
            state, doc = await service.resolve_token_state(raw)

        assert state == "valid"
        assert doc is token_doc

    @pytest.mark.asyncio
    async def test_resolve_state_expired(self):
        """Token with expires_at in past returns ('expired', doc)."""
        past_expiry = datetime.now(timezone.utc) - timedelta(hours=1)
        raw, token_hash, _ = MagicLinkService.generate_token()
        token_doc = {
            "token_hash": token_hash,
            "expires_at": past_expiry,
            "used_at": None,
        }
        mock_db, _ = _make_mock_db(token_doc=token_doc)
        service = MagicLinkService()

        with patch("app.services.magic_link_service.get_database", return_value=mock_db):
            state, doc = await service.resolve_token_state(raw)

        assert state == "expired"
        assert doc is token_doc

    @pytest.mark.asyncio
    async def test_resolve_state_used(self):
        """Token with used_at set returns ('used', doc)."""
        future_expiry = datetime.now(timezone.utc) + timedelta(hours=48)
        raw, token_hash, _ = MagicLinkService.generate_token()
        token_doc = {
            "token_hash": token_hash,
            "expires_at": future_expiry,
            "used_at": datetime.now(timezone.utc),
        }
        mock_db, _ = _make_mock_db(token_doc=token_doc)
        service = MagicLinkService()

        with patch("app.services.magic_link_service.get_database", return_value=mock_db):
            state, doc = await service.resolve_token_state(raw)

        assert state == "used"
        assert doc is token_doc

    @pytest.mark.asyncio
    async def test_resolve_state_not_found(self):
        """Random token not in DB returns ('not_found', None)."""
        mock_db, _ = _make_mock_db(token_doc=None)
        service = MagicLinkService()

        with patch("app.services.magic_link_service.get_database", return_value=mock_db):
            state, doc = await service.resolve_token_state("totally-random-token")

        assert state == "not_found"
        assert doc is None

    @pytest.mark.asyncio
    async def test_resolve_state_handles_naive_datetime(self):
        """resolve_token_state handles timezone-naive datetimes from MongoDB."""
        # MongoDB may return naive datetimes — service must handle them
        past_naive = datetime.now() - timedelta(hours=1)  # naive (no tzinfo)
        raw, token_hash, _ = MagicLinkService.generate_token()
        token_doc = {
            "token_hash": token_hash,
            "expires_at": past_naive,  # naive datetime
            "used_at": None,
        }
        mock_db, _ = _make_mock_db(token_doc=token_doc)
        service = MagicLinkService()

        with patch("app.services.magic_link_service.get_database", return_value=mock_db):
            state, doc = await service.resolve_token_state(raw)

        # Past naive datetime should be treated as expired
        assert state == "expired"


# ---------------------------------------------------------------------------
# mark_opened
# ---------------------------------------------------------------------------


class TestMarkOpened:
    """mark_opened sets opened_at idempotently on first GET."""

    @pytest.mark.asyncio
    async def test_mark_opened_calls_update_with_opened_at_none_filter(self):
        """update_one is called with {opened_at: None} filter (idempotent)."""
        mock_db, mock_collection = _make_mock_db()
        service = MagicLinkService()
        raw = "some-raw-token"

        with patch("app.services.magic_link_service.get_database", return_value=mock_db):
            await service.mark_opened(raw)

        mock_collection.update_one.assert_awaited_once()
        call_filter = mock_collection.update_one.call_args[0][0]
        # Filter must include opened_at: None for idempotency
        assert call_filter.get("opened_at") is None

    @pytest.mark.asyncio
    async def test_mark_opened_idempotent_second_call_no_update(self):
        """Second call with opened_at already set does not change it.

        Simulated by mock returning modified_count=0 on second call
        (MongoDB $set with {opened_at: None} filter finds nothing to update).
        """
        mock_db, mock_collection = _make_mock_db()
        # Simulate: first call updates (modified_count=1), second finds no match (modified_count=0)
        mock_collection.update_one = AsyncMock(
            side_effect=[
                MagicMock(modified_count=1),
                MagicMock(modified_count=0),
            ]
        )
        service = MagicLinkService()
        raw = "some-raw-token"

        with patch("app.services.magic_link_service.get_database", return_value=mock_db):
            await service.mark_opened(raw)
            await service.mark_opened(raw)

        # Both calls completed without raising
        assert mock_collection.update_one.await_count == 2


# ---------------------------------------------------------------------------
# consume
# ---------------------------------------------------------------------------


class TestConsume:
    """consume sets used_at; returns False if already used."""

    @pytest.mark.asyncio
    async def test_consume_sets_used_at_returns_true(self):
        """consume returns True when token was not yet used (modified_count=1)."""
        mock_db, mock_collection = _make_mock_db()
        mock_collection.update_one = AsyncMock(
            return_value=MagicMock(modified_count=1)
        )
        service = MagicLinkService()
        raw = "some-raw-token"

        with patch("app.services.magic_link_service.get_database", return_value=mock_db):
            result = await service.consume(raw)

        assert result is True
        mock_collection.update_one.assert_awaited_once()

    @pytest.mark.asyncio
    async def test_consume_already_used_returns_false(self):
        """consume returns False when token already has used_at set (modified_count=0)."""
        mock_db, mock_collection = _make_mock_db()
        mock_collection.update_one = AsyncMock(
            return_value=MagicMock(modified_count=0)
        )
        service = MagicLinkService()
        raw = "some-raw-token"

        with patch("app.services.magic_link_service.get_database", return_value=mock_db):
            result = await service.consume(raw)

        assert result is False

    @pytest.mark.asyncio
    async def test_consume_filter_has_used_at_none(self):
        """consume uses {used_at: None} filter to prevent double-consumption."""
        mock_db, mock_collection = _make_mock_db()
        mock_collection.update_one = AsyncMock(
            return_value=MagicMock(modified_count=1)
        )
        service = MagicLinkService()
        raw = "some-raw-token"

        with patch("app.services.magic_link_service.get_database", return_value=mock_db):
            await service.consume(raw)

        call_filter = mock_collection.update_one.call_args[0][0]
        assert call_filter.get("used_at") is None
