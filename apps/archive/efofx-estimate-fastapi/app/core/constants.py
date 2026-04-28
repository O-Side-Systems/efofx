"""
Constants and enums for efOfX Estimation Service.

This module defines all application-wide constants, enums, and configuration
values used throughout the estimation service.

Pure enums (EstimationStatus, ReferenceClassCategory, CostBreakdownCategory,
Region) are re-exported from efofx_shared.core.constants so all existing
`from app.core.constants import Region` imports continue to work unchanged.
"""

from efofx_shared.core.constants import (
    CostBreakdownCategory,
    EstimationStatus,
    ReferenceClassCategory,
    Region,
)

__all__ = [
    "EstimationStatus",
    "ReferenceClassCategory",
    "CostBreakdownCategory",
    "Region",
    "API_MESSAGES",
    "ESTIMATION_CONFIG",
    "LLM_PROMPTS",
    "DB_COLLECTIONS",
    "HTTP_STATUS",
    "FILE_UPLOAD_CONFIG",
]

# API Response Messages
API_MESSAGES = {
    "ESTIMATION_STARTED": "Estimation session started successfully",
    "ESTIMATION_UPDATED": "Estimation session updated successfully",
    "ESTIMATION_COMPLETED": "Estimation completed successfully",
    "ESTIMATION_NOT_FOUND": "Estimation session not found",
    "ESTIMATION_EXPIRED": "Estimation session has expired",
    "INVALID_INPUT": "Invalid input provided",
    "UNAUTHORIZED": "Unauthorized access",
    "RATE_LIMITED": "Rate limit exceeded",
    "LLM_ERROR": "Language model processing error",
    "DB_ERROR": "Database operation failed",
    # BYOK / LLM error messages
    "BYOK_INVALID_KEY": "Invalid OpenAI API key. Update your key in Settings.",
    "BYOK_QUOTA_EXHAUSTED": "OpenAI quota exhausted. Recharge your OpenAI account.",
    "LLM_TRANSIENT_ERROR": "We're having trouble generating a response. Please try again in a moment.",  # noqa: E501
    "LLM_UNKNOWN_ERROR": "An unexpected error occurred during AI processing.",
}


# Estimation Configuration
ESTIMATION_CONFIG = {
    "MAX_CHAT_MESSAGES": 50,
    "MAX_PROJECT_DESCRIPTION_LENGTH": 2000,
    "MIN_PROJECT_DESCRIPTION_LENGTH": 10,
    "DEFAULT_CONFIDENCE_THRESHOLD": 0.7,
    "MAX_REFERENCE_PROJECTS": 10,
    "MIN_REFERENCE_PROJECTS": 3,
}


# LLM Prompt Templates
LLM_PROMPTS = {
    "PROJECT_CLASSIFICATION": """
    Analyze the following project description and classify it into the most appropriate reference class.  # noqa: E501

    Project Description: {description}
    Region: {region}

    Available Reference Classes: {reference_classes}

    Please provide:
    1. Primary reference class
    2. Confidence score (0-1)
    3. Reasoning for classification
    """,
    "ESTIMATION_GENERATION": """
    Based on the following project details and reference data, generate a comprehensive estimate.  # noqa: E501

    Project Details:
    - Description: {description}
    - Region: {region}
    - Reference Class: {reference_class}

    Reference Projects: {reference_projects}

    Please provide:
    1. Total estimated cost
    2. Estimated timeline (weeks)
    3. Recommended team size
    4. Cost breakdown by category
    5. Key assumptions and risks
    """,
}


# Database Collections
DB_COLLECTIONS = {
    "TENANTS": "tenants",
    "REFERENCE_CLASSES": "reference_classes",
    "REFERENCE_PROJECTS": "reference_projects",
    "ESTIMATES": "estimates",
    "FEEDBACK": "feedback",
    "CHAT_SESSIONS": "chat_sessions",
    "VERIFICATION_TOKENS": "verification_tokens",
    "REFRESH_TOKENS": "refresh_tokens",
    "WIDGET_LEADS": "widget_leads",
    "WIDGET_ANALYTICS": "widget_analytics",
    "FEEDBACK_TOKENS": "feedback_tokens",
}


# HTTP Status Codes
HTTP_STATUS = {
    "OK": 200,
    "CREATED": 201,
    "BAD_REQUEST": 400,
    "UNAUTHORIZED": 401,
    "FORBIDDEN": 403,
    "NOT_FOUND": 404,
    "RATE_LIMITED": 429,
    "INTERNAL_ERROR": 500,
}


# File Upload Configuration
FILE_UPLOAD_CONFIG = {
    "MAX_SIZE_MB": 10,
    "ALLOWED_EXTENSIONS": [".jpg", ".jpeg", ".png", ".webp"],
    "UPLOAD_DIR": "uploads",
    "TEMP_DIR": "temp",
}
