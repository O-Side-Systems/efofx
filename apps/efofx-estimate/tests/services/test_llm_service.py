"""
Unit tests for LLMService BYOK key injection.

Verifies that LLMService accepts a per-request BYOK api_key, stores it on the
instance, passes it to AsyncOpenAI at construction time, and flows it through
to actual API calls — without making real network requests.
"""

import pytest
from unittest.mock import patch, AsyncMock, MagicMock

from app.core.config import settings
from app.models.estimation import (
    EstimationOutput,
    CostCategoryEstimate,
    AdjustmentFactor,
)


# ---------------------------------------------------------------------------
# Shared fixture
# ---------------------------------------------------------------------------

@pytest.fixture
def mock_openai():
    """Patch AsyncOpenAI at the module level and yield (MockClass, mock_instance)."""
    with patch("app.services.llm_service.AsyncOpenAI") as MockClient:
        mock_instance = AsyncMock()
        MockClient.return_value = mock_instance
        yield MockClient, mock_instance


# ---------------------------------------------------------------------------
# Helper: minimal valid EstimationOutput
# ---------------------------------------------------------------------------

def _make_estimation_output() -> EstimationOutput:
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


# ---------------------------------------------------------------------------
# Test 1: BYOK key passed to AsyncOpenAI constructor
# ---------------------------------------------------------------------------

class TestLLMServiceUsesProvidedApiKey:
    """LLMService(api_key=X) constructs AsyncOpenAI with api_key=X."""

    def test_llmservice_uses_provided_api_key(self, mock_openai):
        """When api_key is provided, AsyncOpenAI is constructed with that key."""
        from app.services.llm_service import LLMService

        MockClient, _ = mock_openai
        byok_key = "sk-test-byok-key-12345"

        LLMService(api_key=byok_key)

        MockClient.assert_called_once_with(api_key=byok_key)


# ---------------------------------------------------------------------------
# Test 2: Fallback to settings key when no api_key provided
# ---------------------------------------------------------------------------

class TestLLMServiceFallbackToSettingsKey:
    """LLMService() without api_key falls back to settings.OPENAI_API_KEY."""

    def test_llmservice_falls_back_to_settings_key(self, mock_openai):
        """When no api_key is given, AsyncOpenAI is constructed with settings key."""
        from app.services.llm_service import LLMService

        MockClient, _ = mock_openai

        LLMService()

        MockClient.assert_called_once_with(api_key=settings.OPENAI_API_KEY)


# ---------------------------------------------------------------------------
# Test 3: BYOK key flows through to actual API calls
# ---------------------------------------------------------------------------

class TestLLMServiceByokKeyUsedInApiCall:
    """The mocked client created with the BYOK key is the one used for API calls."""

    async def test_llmservice_byok_key_used_in_api_call(self, mock_openai):
        """generate_response() uses the client initialised with the BYOK key."""
        from app.services.llm_service import LLMService

        MockClient, mock_instance = mock_openai

        # Set up mock response for chat.completions.create
        mock_choice = MagicMock()
        mock_choice.message.content = "general"
        mock_response = MagicMock()
        mock_response.choices = [mock_choice]
        mock_instance.chat.completions.create = AsyncMock(return_value=mock_response)

        byok_key = "sk-test-key-for-api-call"
        service = LLMService(api_key=byok_key)

        # Confirm the service's client IS the mock_instance returned by our MockClient
        assert service.client is mock_instance

        result = await service.generate_response("test prompt")

        # Confirm the mock client was used for the API call
        mock_instance.chat.completions.create.assert_called_once()
        assert result == "general"


# ---------------------------------------------------------------------------
# Test 4: generate_estimation with BYOK key returns EstimationOutput
# ---------------------------------------------------------------------------

class TestLLMServiceGenerateEstimationWithByokKey:
    """generate_estimation() with a BYOK key returns the mocked EstimationOutput."""

    async def test_llmservice_generate_estimation_with_byok_key(self, mock_openai):
        """generate_estimation uses the BYOK-keyed client and returns EstimationOutput."""
        from app.services.llm_service import LLMService

        MockClient, mock_instance = mock_openai
        expected_output = _make_estimation_output()

        # Set up mock response for beta.chat.completions.parse
        mock_choice = MagicMock()
        mock_choice.message.parsed = expected_output
        mock_choice.message.refusal = None
        mock_parse_response = MagicMock()
        mock_parse_response.choices = [mock_choice]
        mock_instance.beta.chat.completions.parse = AsyncMock(
            return_value=mock_parse_response
        )

        service = LLMService(api_key="sk-byok-key-for-estimation")
        result = await service.generate_estimation(
            description="Install a 15x30 pool with spa",
            reference_class="residential_pool",
            region="SoCal - Coastal",
        )

        mock_instance.beta.chat.completions.parse.assert_called_once()
        assert result is expected_output
        assert result.total_cost_p50 == 45000.0
        assert result.confidence_score == 80.0


# ---------------------------------------------------------------------------
# Test 5: api_key stored on instance
# ---------------------------------------------------------------------------

class TestLLMServiceApiKeyStoredOnInstance:
    """LLMService stores the resolved api_key on self.api_key."""

    def test_llmservice_api_key_stored_on_instance(self, mock_openai):
        """service.api_key equals the api_key provided at construction."""
        from app.services.llm_service import LLMService

        service = LLMService(api_key="sk-custom-key")
        assert service.api_key == "sk-custom-key"

    def test_llmservice_fallback_api_key_stored_on_instance(self, mock_openai):
        """When no api_key given, service.api_key equals settings.OPENAI_API_KEY."""
        from app.services.llm_service import LLMService

        service = LLMService()
        assert service.api_key == settings.OPENAI_API_KEY
