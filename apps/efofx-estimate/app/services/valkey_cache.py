"""
Distributed LLM response cache backed by Valkey.

Provides tenant-scoped caching with graceful fallback — Valkey outages
never surface as errors to callers. Cache keys use the format:

    efofx:llm:{tenant_id}:{input_hash}

This ensures cache entries are isolated per tenant (INFR-02) and that a
cached response for Tenant A cannot be served to Tenant B.

Usage::

    from app.services.valkey_cache import _cache as valkey_cache

    input_hash = ValkeyCache.make_input_hash(messages, model)
    cached = await valkey_cache.get(tenant_id, input_hash)
    if cached is None:
        # live call...
        await valkey_cache.set(tenant_id, input_hash, result_json)
"""

import hashlib
import json
import logging
import time
from typing import Optional

import valkey.asyncio as valkey_async
from valkey.exceptions import ConnectionError as ValkeyConnectionError, TimeoutError as ValkeyTimeoutError

from app.core.config import settings

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Warning cooldown — prevent log flooding on sustained Valkey outages
# ---------------------------------------------------------------------------

_last_warn_at: float = 0.0
_WARN_COOLDOWN_SECONDS: int = 60


# ---------------------------------------------------------------------------
# ValkeyCache
# ---------------------------------------------------------------------------


class ValkeyCache:
    """Distributed LLM response cache backed by Valkey.

    - Keys are tenant-scoped: ``efofx:llm:{tenant_id}:{input_hash}``
    - Connection is lazy — created on first use, not at init
    - All Valkey errors are caught and logged (with cooldown); callers receive
      ``None`` on get and a silent no-op on set (INFR-03)
    """

    def __init__(self) -> None:
        self._client: Optional[valkey_async.Valkey] = None

    def _get_client(self) -> valkey_async.Valkey:
        """Lazy-initialize the async Valkey client.

        Returns the cached client on subsequent calls. Client configuration:
        - decode_responses=True: keys/values returned as str (not bytes)
        - socket_timeout=2.0: per-command timeout prevents hung workers
        - socket_connect_timeout=2.0: connection attempt timeout
        """
        if self._client is None:
            self._client = valkey_async.Valkey.from_url(
                settings.VALKEY_URL,
                decode_responses=True,
                socket_timeout=2.0,
                socket_connect_timeout=2.0,
            )
        return self._client

    def _make_key(self, tenant_id: str, input_hash: str) -> str:
        """Build the canonical cache key for a tenant+input combination.

        Format: ``efofx:llm:{tenant_id}:{input_hash}``
        """
        return f"efofx:llm:{tenant_id}:{input_hash}"

    @staticmethod
    def make_input_hash(messages: list[dict], model: str) -> str:
        """SHA-256 hash of (messages + model) for deterministic cache key.

        Produces the same output as the former standalone ``_make_cache_key``
        function in ``llm_service.py``. Uses ``sort_keys=True`` for order
        stability and ``ensure_ascii=True`` for consistent encoding.
        """
        payload = {"messages": messages, "model": model}
        serialized = json.dumps(payload, sort_keys=True, ensure_ascii=True)
        return hashlib.sha256(serialized.encode("utf-8")).hexdigest()

    def _maybe_warn(self, exc: Exception) -> None:
        """Log a Valkey-unavailable warning, throttled to once per cooldown period.

        Prevents log flooding when Valkey is unreachable for an extended period.
        The warning is suppressed if one was already emitted within the last
        ``_WARN_COOLDOWN_SECONDS`` seconds.
        """
        global _last_warn_at
        now = time.monotonic()
        if now - _last_warn_at > _WARN_COOLDOWN_SECONDS:
            logger.warning("Valkey unavailable — falling back to live LLM call: %s", exc)
            _last_warn_at = now

    async def get(self, tenant_id: str, input_hash: str) -> Optional[str]:
        """Retrieve a cached LLM response for the given tenant+input.

        Returns:
            The cached JSON string on a hit, ``None`` on a miss or any
            Valkey error (connection refused, timeout, etc.).
        """
        key = self._make_key(tenant_id, input_hash)
        try:
            client = self._get_client()
            value = await client.get(key)
            if value is not None:
                logger.debug("Valkey cache HIT (key=%s...)", key[:24])
            else:
                logger.debug("Valkey cache MISS (key=%s...)", key[:24])
            return value
        except (ValkeyConnectionError, ValkeyTimeoutError) as exc:
            self._maybe_warn(exc)
            return None

    async def set(self, tenant_id: str, input_hash: str, value: str) -> None:
        """Store an LLM response in the cache under the given tenant+input key.

        Silently no-ops on any Valkey error — callers should not need to
        handle caching failures.

        Args:
            tenant_id:  Tenant identifier (for key scoping).
            input_hash: SHA-256 hash of the LLM input (from ``make_input_hash``).
            value:      Serialized JSON string of the LLM response.
        """
        key = self._make_key(tenant_id, input_hash)
        try:
            client = self._get_client()
            await client.set(key, value, ex=settings.VALKEY_CACHE_TTL)
        except (ValkeyConnectionError, ValkeyTimeoutError) as exc:
            self._maybe_warn(exc)

    async def close(self) -> None:
        """Close the underlying Valkey connection, if open.

        Safe to call even if the client was never initialized.
        Called during application lifespan shutdown.
        """
        if self._client is not None:
            await self._client.aclose()
            self._client = None


# ---------------------------------------------------------------------------
# Module-level singleton
# ---------------------------------------------------------------------------

_cache = ValkeyCache()
