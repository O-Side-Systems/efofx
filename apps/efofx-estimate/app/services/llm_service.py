"""
LLM service for efOfX Estimation Service.

This module provides integration with OpenAI for natural language processing
and estimation generation using OpenAI v2 structured outputs.

BYOK Key Injection
------------------
Every LLM call uses the tenant's decrypted BYOK key. There is no fallback to
settings.OPENAI_API_KEY in production code paths. Use the ``get_llm_service``
FastAPI dependency to obtain a scoped LLMService with the tenant's key.

    from app.services.llm_service import get_llm_service

    @router.post("/some-endpoint")
    async def handler(llm: LLMService = Depends(get_llm_service)):
        result = await llm.generate_estimation(...)
"""

import logging
from typing import Optional, Dict, Any

from fastapi import Depends, HTTPException
from openai import (
    AsyncOpenAI,
    AuthenticationError,
    RateLimitError,
    APITimeoutError,
    APIConnectionError,
    OpenAIError,
)

from app.core.config import settings
from app.core.security import get_current_tenant
from app.models.estimation import EstimationOutput
from app.models.tenant import Tenant
from app.services.byok_service import decrypt_tenant_openai_key
from app.services.valkey_cache import _cache as valkey_cache, ValkeyCache

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Error classification
# ---------------------------------------------------------------------------


def classify_openai_error(exc: OpenAIError) -> tuple[str, int]:
    """Classify an OpenAI exception into (error_type, http_status).

    Returns:
        ("invalid_key", 402) — key is invalid or expired
        ("quota_exhausted", 402) — OpenAI quota used up
        ("transient", 503) — timeout/connection/rate-limit, retry eligible
        ("unknown", 500) — unexpected error
    """
    if isinstance(exc, AuthenticationError):
        return ("invalid_key", 402)
    if isinstance(exc, RateLimitError):
        if "insufficient_quota" in str(exc):
            return ("quota_exhausted", 402)
        return ("transient", 503)
    if isinstance(exc, (APITimeoutError, APIConnectionError)):
        return ("transient", 503)
    return ("unknown", 500)


# ---------------------------------------------------------------------------
# LLMService
# ---------------------------------------------------------------------------


class LLMService:
    """Service for LLM integration and text generation.

    Every instance is scoped to a single request and uses the tenant's
    decrypted BYOK key. There is no fallback to settings.OPENAI_API_KEY.
    Use the ``get_llm_service`` FastAPI dependency to obtain an instance.
    """

    def __init__(self, api_key: str, tenant_id: str) -> None:
        """api_key and tenant_id are REQUIRED — no fallback to settings.OPENAI_API_KEY."""
        self.api_key = api_key
        self.tenant_id = tenant_id
        self.client = AsyncOpenAI(api_key=self.api_key)
        self.model = settings.OPENAI_MODEL
        self.max_tokens = settings.OPENAI_MAX_TOKENS
        self.temperature = settings.OPENAI_TEMPERATURE

    async def generate_response(self, prompt: str, system_message: Optional[str] = None) -> str:
        """Generate a free-form text response using OpenAI chat completions."""
        try:
            messages: list[Dict[str, str]] = []

            if system_message:
                messages.append({"role": "system", "content": system_message})

            messages.append({"role": "user", "content": prompt})

            response = await self.client.chat.completions.create(
                model=self.model,
                messages=messages,
                max_tokens=self.max_tokens,
                temperature=self.temperature,
            )

            content = response.choices[0].message.content
            return content.strip() if content else ""

        except RateLimitError as e:
            logger.error(f"OpenAI rate limit exceeded: {e}")
            raise
        except APITimeoutError as e:
            logger.error(f"OpenAI API timeout: {e}")
            raise
        except OpenAIError as e:
            logger.error(f"OpenAI API error generating response: {e}")
            raise

    async def classify_project(self, description: str, region: str, reference_classes: list) -> str:
        """Classify project into reference class using LLM."""
        try:
            prompt = f"""Analyze the following project description and classify it into the most appropriate reference class.

Project Description: {description}
Region: {region}

Available Reference Classes: {reference_classes}

Please provide only the reference class name as your response."""

            system_message = (
                "You are an expert construction estimator. Your task is to classify construction "
                "projects into appropriate reference classes based on project descriptions and "
                "regional context. Respond with only the reference class name, nothing else."
            )

            response = await self.generate_response(prompt, system_message)
            return response.strip().lower()

        except OpenAIError as e:
            logger.error(f"Error classifying project: {e}")
            return "general"

    async def generate_estimation(
        self,
        description: str,
        reference_class: str,
        region: str,
        reference_data: Optional[Dict[str, Any]] = None,
        use_cache: bool = True,
    ) -> EstimationOutput:
        """Generate structured estimation using OpenAI v2 structured output.

        Uses client.beta.chat.completions.parse() with EstimationOutput Pydantic model
        as response_format for guaranteed schema conformance. Returns a fully typed
        EstimationOutput — no hardcoded stubs or fallback values.

        Args:
            description:     Project description text.
            reference_class: Reference class name for the project.
            region:          Geographic region string.
            reference_data:  Optional reference data dict.
            use_cache:       When True (default), return cached response if available.
                             When False, bypass cache (useful for forced refresh).
        """
        system_prompt = (
            "You are a project cost estimation expert. Given a project description, "
            "reference class, and region, produce a detailed cost estimate with P50/P80 ranges. "
            "Break costs down by category. Include adjustment factors as named multipliers. "
            "List all assumptions explicitly. Assign a confidence score (0-100) based on "
            "how much information you have to work with. Omit inapplicable cost categories "
            "rather than zero-filling them."
        )

        user_prompt = f"""Project Description: {description}
Reference Class: {reference_class}
Region: {region}"""

        if reference_data:
            user_prompt += f"\nReference Data: {reference_data}"

        messages = [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt},
        ]

        input_hash = ValkeyCache.make_input_hash(messages, settings.OPENAI_MODEL)

        # Cache lookup
        if use_cache:
            cached = await valkey_cache.get(self.tenant_id, input_hash)
            if cached is not None:
                logger.debug("Cache hit for estimation request (key=%s...)", input_hash[:8])
                return EstimationOutput.model_validate_json(cached)

        try:
            completion = await self.client.beta.chat.completions.parse(
                model=settings.OPENAI_MODEL,  # gpt-4o-mini (required for structured outputs)
                messages=messages,
                response_format=EstimationOutput,
            )
            result = completion.choices[0].message.parsed

            if result is None:
                # LLM refused or failed to produce a parseable response
                refusal = completion.choices[0].message.refusal
                raise ValueError(f"LLM refused or failed to parse structured output: {refusal}")

            # Store in cache
            if use_cache:
                await valkey_cache.set(self.tenant_id, input_hash, result.model_dump_json())

            return result

        except RateLimitError as e:
            logger.error(f"OpenAI rate limit exceeded: {e}")
            raise
        except APITimeoutError as e:
            logger.error(f"OpenAI API timeout: {e}")
            raise
        except OpenAIError as e:
            logger.error(f"OpenAI API error generating estimation: {e}")
            raise

    async def stream_chat_completion(
        self,
        messages: list[dict],
        temperature: float | None = None,
    ):
        """Async generator yielding content strings from a streaming chat completion.

        Filters out None chunks (role-only and finish_reason-only).
        Caller wraps in SSE formatting.
        """
        stream = await self.client.chat.completions.create(
            model=self.model,
            messages=messages,
            stream=True,
            temperature=temperature or self.temperature,
        )
        async for chunk in stream:
            content = chunk.choices[0].delta.content
            if content is not None:
                yield content


# ---------------------------------------------------------------------------
# FastAPI dependency
# ---------------------------------------------------------------------------


async def get_llm_service(tenant: Tenant = Depends(get_current_tenant)) -> LLMService:
    """FastAPI dependency: decrypt BYOK key and return scoped LLMService.

    Raises HTTP 402 if tenant has no stored OpenAI key.
    """
    api_key = await decrypt_tenant_openai_key(tenant.tenant_id)
    return LLMService(api_key=api_key, tenant_id=tenant.tenant_id)
