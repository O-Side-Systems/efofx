"""
Unit tests for ValkeyCache service.

Covers:
- make_input_hash determinism and variation
- get/set with FakeAsyncRedis backend
- Tenant isolation: Tenant A key not readable by Tenant B (INFR-02)
- Graceful fallback on ValkeyConnectionError and ValkeyTimeoutError (INFR-03)
- Warning cooldown: logger.warning called at most once per cooldown period
"""

import pytest
import pytest_asyncio
from unittest.mock import patch, AsyncMock

import fakeredis
from valkey.exceptions import ConnectionError as ValkeyConnectionError, TimeoutError as ValkeyTimeoutError

from app.services.valkey_cache import ValkeyCache


# ---------------------------------------------------------------------------
# Shared fixture
# ---------------------------------------------------------------------------


@pytest_asyncio.fixture
async def cache_with_fake():
    """ValkeyCache with injected FakeAsyncRedis client."""
    cache = ValkeyCache()
    cache._client = fakeredis.FakeAsyncRedis(server_type="valkey", decode_responses=True)
    yield cache
    await cache.close()


# ---------------------------------------------------------------------------
# make_input_hash
# ---------------------------------------------------------------------------


class TestValkeyMakeInputHash:
    """ValkeyCache.make_input_hash produces deterministic and varied keys."""

    def test_make_input_hash_deterministic(self):
        """Same messages + model always produce the same hash of length 64."""
        messages = [{"role": "user", "content": "hello"}]
        hash1 = ValkeyCache.make_input_hash(messages, "gpt-4o-mini")
        hash2 = ValkeyCache.make_input_hash(messages, "gpt-4o-mini")
        assert hash1 == hash2
        assert len(hash1) == 64  # SHA-256 hex digest

    def test_make_input_hash_varies_with_model(self):
        """Different model produces a different hash."""
        messages = [{"role": "user", "content": "hello"}]
        hash1 = ValkeyCache.make_input_hash(messages, "gpt-4o-mini")
        hash2 = ValkeyCache.make_input_hash(messages, "gpt-4o")
        assert hash1 != hash2

    def test_make_input_hash_varies_with_messages(self):
        """Different messages produce a different hash."""
        messages1 = [{"role": "user", "content": "hello"}]
        messages2 = [{"role": "user", "content": "goodbye"}]
        hash1 = ValkeyCache.make_input_hash(messages1, "gpt-4o-mini")
        hash2 = ValkeyCache.make_input_hash(messages2, "gpt-4o-mini")
        assert hash1 != hash2


# ---------------------------------------------------------------------------
# get / set with FakeAsyncRedis
# ---------------------------------------------------------------------------


class TestValkeyCacheGetSet:
    """ValkeyCache get/set round-trip using FakeAsyncRedis."""

    async def test_cache_set_get_returns_stored_value(self, cache_with_fake):
        """set a value, get returns it."""
        await cache_with_fake.set("tenant-a", "abc123", '{"result": "cached"}')
        result = await cache_with_fake.get("tenant-a", "abc123")
        assert result == '{"result": "cached"}'

    async def test_cache_get_miss_returns_none(self, cache_with_fake):
        """get on empty cache returns None."""
        result = await cache_with_fake.get("tenant-a", "nonexistent")
        assert result is None

    def test_cache_key_format(self):
        """_make_key returns the canonical efofx:llm:{tenant_id}:{input_hash} format."""
        cache = ValkeyCache()
        key = cache._make_key("tenant-a", "abc123")
        assert key == "efofx:llm:tenant-a:abc123"


# ---------------------------------------------------------------------------
# Tenant isolation (INFR-02)
# ---------------------------------------------------------------------------


class TestValkeyCacheTenantIsolation:
    """Cache entries are isolated per tenant."""

    async def test_tenant_a_key_not_readable_by_tenant_b(self, cache_with_fake):
        """Set under tenant-A, get under tenant-B returns None (same input_hash)."""
        input_hash = "deadbeefdeadbeef"
        await cache_with_fake.set("tenant-a", input_hash, '{"data": "secret"}')

        result_b = await cache_with_fake.get("tenant-b", input_hash)
        assert result_b is None

        # Sanity: tenant-a can still read it
        result_a = await cache_with_fake.get("tenant-a", input_hash)
        assert result_a == '{"data": "secret"}'


# ---------------------------------------------------------------------------
# Graceful fallback (INFR-03)
# ---------------------------------------------------------------------------


class TestValkeyCacheFallback:
    """Valkey errors never propagate — callers receive None or silent no-op."""

    async def test_get_returns_none_on_connection_error(self):
        """get() returns None when ValkeyConnectionError is raised."""
        cache = ValkeyCache()
        mock_client = AsyncMock()
        mock_client.get = AsyncMock(side_effect=ValkeyConnectionError("refused"))
        cache._client = mock_client

        result = await cache.get("tenant-a", "abc123")
        assert result is None

    async def test_get_returns_none_on_timeout_error(self):
        """get() returns None when ValkeyTimeoutError is raised."""
        cache = ValkeyCache()
        mock_client = AsyncMock()
        mock_client.get = AsyncMock(side_effect=ValkeyTimeoutError("timed out"))
        cache._client = mock_client

        result = await cache.get("tenant-a", "abc123")
        assert result is None

    async def test_set_silently_noop_on_connection_error(self):
        """set() completes without raising when ValkeyConnectionError is raised."""
        cache = ValkeyCache()
        mock_client = AsyncMock()
        mock_client.set = AsyncMock(side_effect=ValkeyConnectionError("refused"))
        cache._client = mock_client

        # Must not raise — silent no-op
        await cache.set("tenant-a", "abc123", '{"data": "value"}')


# ---------------------------------------------------------------------------
# Warning cooldown
# ---------------------------------------------------------------------------


class TestValkeyCacheWarningCooldown:
    """Warning is throttled to once per _WARN_COOLDOWN_SECONDS."""

    async def test_warning_logged_on_first_failure(self):
        """logger.warning is called once on the first connection error."""
        import app.services.valkey_cache as cache_mod

        # Reset cooldown so the first failure always triggers a warning
        cache_mod._last_warn_at = 0.0

        cache = ValkeyCache()
        mock_client = AsyncMock()
        mock_client.get = AsyncMock(side_effect=ValkeyConnectionError("refused"))
        cache._client = mock_client

        with patch.object(cache_mod.logger, "warning") as mock_warning:
            await cache.get("tenant-a", "abc123")
            mock_warning.assert_called_once()

    async def test_warning_suppressed_within_cooldown(self):
        """Two errors within 1 second: logger.warning called only once."""
        import time
        import app.services.valkey_cache as cache_mod

        # Simulate a recent warning (just now)
        cache_mod._last_warn_at = time.monotonic()

        cache = ValkeyCache()
        mock_client = AsyncMock()
        mock_client.get = AsyncMock(side_effect=ValkeyConnectionError("refused"))
        cache._client = mock_client

        with patch.object(cache_mod.logger, "warning") as mock_warning:
            # First call: warning suppressed (within cooldown)
            await cache.get("tenant-a", "abc123")
            # Second call: still suppressed
            await cache.get("tenant-a", "abc123")
            mock_warning.assert_not_called()
