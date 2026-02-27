"""
Unit tests for LLMService BYOK hardening, error classification, and caching.

Covers:
- BYOK-only constructor (no settings fallback)
- classify_openai_error mapping all OpenAI exception types
- _make_cache_key determinism and variation
- generate_estimation cache hit/miss/bypass behavior
- get_llm_service FastAPI dependency: BYOK decryption and 402 gate
"""

import pytest
from unittest.mock import patch, AsyncMock, MagicMock

from app.models.estimation import (
    EstimationOutput,
    CostCategoryEstimate,
    AdjustmentFactor,
)


# ---------------------------------------------------------------------------
# Shared helpers
# ---------------------------------------------------------------------------


def _make_estimation_output() -> EstimationOutput:
    """Build a minimal valid EstimationOutput for mocking."""
    return EstimationOutput(
        total_cost_p50=45000.0,
        total_cost_p80=55000.0,
        timeline_weeks_p50=8,
        timeline_weeks_p80=10,
        cost_breakdown=[
            CostCategoryEstimate(
                category="Materials",
                p50_cost=20000.0,
                p80_cost=24000.0,
                percentage_of_total=0.44,
            )
        ],
        adjustment_factors=[
            AdjustmentFactor(
                name="Urban premium",
                multiplier=1.15,
                reason="Coastal area cost uplift",
            )
        ],
        confidence_score=80.0,
        assumptions=["Standard site conditions"],
        summary="Estimated cost for pool installation in coastal region.",
    )


@pytest.fixture(autouse=True)
def clear_response_cache():
    """Clear the module-level _response_cache before each test."""
    import app.services.llm_service as llm_module
    llm_module._response_cache.clear()
    yield
    llm_module._response_cache.clear()


@pytest.fixture
def mock_openai():
    """Patch AsyncOpenAI at the module level and yield (MockClass, mock_instance)."""
    with patch("app.services.llm_service.AsyncOpenAI") as MockClient:
        mock_instance = AsyncMock()
        MockClient.return_value = mock_instance
        yield MockClient, mock_instance


# ---------------------------------------------------------------------------
# Constructor tests
# ---------------------------------------------------------------------------


class TestLLMServiceConstructor:
    """LLMService constructor requirements."""

    def test_llm_service_requires_api_key(self, mock_openai):
        """LLMService() with no args raises TypeError — api_key is required."""
        from app.services.llm_service import LLMService

        with pytest.raises(TypeError):
            LLMService()  # type: ignore[call-arg]

    def test_llm_service_creates_client_with_provided_key(self, mock_openai):
        """LLMService(api_key=X) constructs AsyncOpenAI with api_key=X."""
        from app.services.llm_service import LLMService

        MockClient, _ = mock_openai
        byok_key = "sk-test-byok-key-12345"

        LLMService(api_key=byok_key)

        MockClient.assert_called_once_with(api_key=byok_key)


# ---------------------------------------------------------------------------
# Error classification tests
# ---------------------------------------------------------------------------


class TestClassifyOpenAIError:
    """classify_openai_error maps all OpenAI exceptions correctly."""

    def test_classify_error_authentication(self):
        """AuthenticationError -> ("invalid_key", 402)."""
        from app.services.llm_service import classify_openai_error
        from openai import AuthenticationError

        exc = AuthenticationError.__new__(AuthenticationError)
        result = classify_openai_error(exc)
        assert result == ("invalid_key", 402)

    def test_classify_error_quota_exhausted(self):
        """RateLimitError with 'insufficient_quota' -> ("quota_exhausted", 402)."""
        from app.services.llm_service import classify_openai_error
        from openai import RateLimitError

        # Build a RateLimitError whose str() contains "insufficient_quota"
        exc = RateLimitError.__new__(RateLimitError)
        exc.args = ("insufficient_quota error from openai",)
        result = classify_openai_error(exc)
        assert result == ("quota_exhausted", 402)

    def test_classify_error_rate_limit_transient(self):
        """RateLimitError without 'insufficient_quota' -> ("transient", 503)."""
        from app.services.llm_service import classify_openai_error
        from openai import RateLimitError

        exc = RateLimitError.__new__(RateLimitError)
        exc.args = ("rate limit exceeded",)
        result = classify_openai_error(exc)
        assert result == ("transient", 503)

    def test_classify_error_timeout(self):
        """APITimeoutError -> ("transient", 503)."""
        from app.services.llm_service import classify_openai_error
        from openai import APITimeoutError

        exc = APITimeoutError.__new__(APITimeoutError)
        result = classify_openai_error(exc)
        assert result == ("transient", 503)

    def test_classify_error_connection(self):
        """APIConnectionError -> ("transient", 503)."""
        from app.services.llm_service import classify_openai_error
        from openai import APIConnectionError

        exc = APIConnectionError.__new__(APIConnectionError)
        result = classify_openai_error(exc)
        assert result == ("transient", 503)

    def test_classify_error_unknown(self):
        """Generic OpenAIError -> ("unknown", 500)."""
        from app.services.llm_service import classify_openai_error
        from openai import OpenAIError

        exc = OpenAIError("something unexpected")
        result = classify_openai_error(exc)
        assert result == ("unknown", 500)


# ---------------------------------------------------------------------------
# Cache key tests
# ---------------------------------------------------------------------------


class TestMakeCacheKey:
    """_make_cache_key produces deterministic and varied keys."""

    def test_make_cache_key_deterministic(self):
        """Same messages + model always produce the same key."""
        from app.services.llm_service import _make_cache_key

        messages = [{"role": "user", "content": "hello"}]
        key1 = _make_cache_key(messages, "gpt-4o-mini")
        key2 = _make_cache_key(messages, "gpt-4o-mini")
        assert key1 == key2
        assert len(key1) == 64  # SHA-256 hex digest

    def test_make_cache_key_varies_with_model(self):
        """Different model produces a different key."""
        from app.services.llm_service import _make_cache_key

        messages = [{"role": "user", "content": "hello"}]
        key1 = _make_cache_key(messages, "gpt-4o-mini")
        key2 = _make_cache_key(messages, "gpt-4o")
        assert key1 != key2

    def test_make_cache_key_varies_with_messages(self):
        """Different messages produce a different key."""
        from app.services.llm_service import _make_cache_key

        messages1 = [{"role": "user", "content": "hello"}]
        messages2 = [{"role": "user", "content": "goodbye"}]
        key1 = _make_cache_key(messages1, "gpt-4o-mini")
        key2 = _make_cache_key(messages2, "gpt-4o-mini")
        assert key1 != key2


# ---------------------------------------------------------------------------
# Caching behavior in generate_estimation
# ---------------------------------------------------------------------------


class TestGenerateEstimationCaching:
    """generate_estimation caches results and respects use_cache=False."""

    async def test_generate_estimation_caches_result(self, mock_openai):
        """Second call with same args uses cache — OpenAI called only once."""
        from app.services.llm_service import LLMService

        _, mock_instance = mock_openai
        expected_output = _make_estimation_output()

        mock_choice = MagicMock()
        mock_choice.message.parsed = expected_output
        mock_choice.message.refusal = None
        mock_parse_response = MagicMock()
        mock_parse_response.choices = [mock_choice]
        mock_instance.beta.chat.completions.parse = AsyncMock(
            return_value=mock_parse_response
        )

        service = LLMService(api_key="sk-byok-key")

        result1 = await service.generate_estimation(
            description="Pool install",
            reference_class="residential_pool",
            region="SoCal - Coastal",
            use_cache=True,
        )
        result2 = await service.generate_estimation(
            description="Pool install",
            reference_class="residential_pool",
            region="SoCal - Coastal",
            use_cache=True,
        )

        # OpenAI should be called exactly once (second is cache hit)
        mock_instance.beta.chat.completions.parse.assert_called_once()
        assert result1.total_cost_p50 == result2.total_cost_p50

    async def test_generate_estimation_cache_bypass(self, mock_openai):
        """With use_cache=False, OpenAI is called even if cache has an entry."""
        from app.services.llm_service import LLMService, _response_cache, _make_cache_key
        from app.core.config import settings

        _, mock_instance = mock_openai
        expected_output = _make_estimation_output()

        mock_choice = MagicMock()
        mock_choice.message.parsed = expected_output
        mock_choice.message.refusal = None
        mock_parse_response = MagicMock()
        mock_parse_response.choices = [mock_choice]
        mock_instance.beta.chat.completions.parse = AsyncMock(
            return_value=mock_parse_response
        )

        service = LLMService(api_key="sk-byok-key")

        # First call with cache enabled — populates cache
        await service.generate_estimation(
            description="Pool install",
            reference_class="residential_pool",
            region="SoCal - Coastal",
            use_cache=True,
        )

        # Second call with cache DISABLED — should call OpenAI again
        await service.generate_estimation(
            description="Pool install",
            reference_class="residential_pool",
            region="SoCal - Coastal",
            use_cache=False,
        )

        # OpenAI should be called twice (bypass ignores cache)
        assert mock_instance.beta.chat.completions.parse.call_count == 2


# ---------------------------------------------------------------------------
# get_llm_service FastAPI dependency tests
# ---------------------------------------------------------------------------


class TestGetLLMService:
    """get_llm_service dependency decrypts BYOK key and returns LLMService."""

    async def test_get_llm_service_calls_decrypt(self):
        """get_llm_service calls decrypt_tenant_openai_key with tenant.tenant_id."""
        from app.services.llm_service import get_llm_service

        mock_tenant = MagicMock()
        mock_tenant.tenant_id = "test-tenant-uuid-1234"

        with patch("app.services.llm_service.decrypt_tenant_openai_key", new_callable=AsyncMock) as mock_decrypt, \
             patch("app.services.llm_service.AsyncOpenAI"):
            mock_decrypt.return_value = "sk-decrypted-key"

            await get_llm_service(tenant=mock_tenant)

            mock_decrypt.assert_called_once_with("test-tenant-uuid-1234")

    async def test_get_llm_service_returns_llm_with_key(self):
        """get_llm_service returns an LLMService with the decrypted key."""
        from app.services.llm_service import get_llm_service, LLMService

        mock_tenant = MagicMock()
        mock_tenant.tenant_id = "test-tenant-uuid-5678"

        with patch("app.services.llm_service.decrypt_tenant_openai_key", new_callable=AsyncMock) as mock_decrypt, \
             patch("app.services.llm_service.AsyncOpenAI"):
            mock_decrypt.return_value = "sk-decrypted-test-key"

            result = await get_llm_service(tenant=mock_tenant)

            assert isinstance(result, LLMService)
            assert result.api_key == "sk-decrypted-test-key"

    async def test_get_llm_service_raises_402_no_key(self):
        """When decrypt raises 402, get_llm_service propagates it."""
        from fastapi import HTTPException
        from app.services.llm_service import get_llm_service

        mock_tenant = MagicMock()
        mock_tenant.tenant_id = "test-tenant-no-key"

        with patch("app.services.llm_service.decrypt_tenant_openai_key", new_callable=AsyncMock) as mock_decrypt:
            mock_decrypt.side_effect = HTTPException(
                status_code=402,
                detail="OpenAI API key required. Add your key in Settings.",
            )

            with pytest.raises(HTTPException) as exc_info:
                await get_llm_service(tenant=mock_tenant)

            assert exc_info.value.status_code == 402
