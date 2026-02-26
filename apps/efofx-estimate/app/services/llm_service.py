"""
LLM service for efOfX Estimation Service.

This module provides integration with OpenAI for natural language processing
and estimation generation using OpenAI v2 structured outputs.
"""

import logging
from typing import Optional, Dict, Any

from openai import AsyncOpenAI, OpenAIError, RateLimitError, APITimeoutError

from app.core.config import settings
from app.models.estimation import EstimationOutput

logger = logging.getLogger(__name__)


class LLMService:
    """Service for LLM integration and text generation."""

    def __init__(self) -> None:
        self.client = AsyncOpenAI(api_key=settings.OPENAI_API_KEY)
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
    ) -> EstimationOutput:
        """Generate structured estimation using OpenAI v2 structured output.

        Uses client.beta.chat.completions.parse() with EstimationOutput Pydantic model
        as response_format for guaranteed schema conformance. Returns a fully typed
        EstimationOutput — no hardcoded stubs or fallback values.
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

        try:
            completion = await self.client.beta.chat.completions.parse(
                model=settings.OPENAI_MODEL,  # gpt-4o-mini (required for structured outputs)
                messages=[
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": user_prompt},
                ],
                response_format=EstimationOutput,
            )
            result = completion.choices[0].message.parsed

            if result is None:
                # LLM refused or failed to produce a parseable response
                refusal = completion.choices[0].message.refusal
                raise ValueError(f"LLM refused or failed to parse structured output: {refusal}")

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
