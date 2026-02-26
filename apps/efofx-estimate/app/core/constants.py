"""
Constants and enums for efOfX Estimation Service.

This module defines all application-wide constants, enums, and configuration
values used throughout the estimation service.
"""

from enum import Enum
from typing import Dict, List


class EstimationStatus(str, Enum):
    """Status of estimation sessions."""
    INITIATED = "initiated"
    IN_PROGRESS = "in_progress"
    COMPLETED = "completed"
    CANCELLED = "cancelled"
    EXPIRED = "expired"


class ReferenceClassCategory(str, Enum):
    """Categories for reference class classification."""
    RESIDENTIAL = "residential"
    COMMERCIAL = "commercial"
    INDUSTRIAL = "industrial"
    INFRASTRUCTURE = "infrastructure"
    LANDSCAPING = "landscaping"
    RENOVATION = "renovation"
    NEW_CONSTRUCTION = "new_construction"


class CostBreakdownCategory(str, Enum):
    """Categories for cost breakdown in estimates."""
    MATERIALS = "materials"
    LABOR = "labor"
    EQUIPMENT = "equipment"
    PERMITS = "permits"
    DESIGN = "design"
    CONTINGENCY = "contingency"
    PROFIT_MARGIN = "profit_margin"


class Region(str, Enum):
    """Geographic regions for estimation."""
    SOCAL_COASTAL = "SoCal - Coastal"
    SOCAL_INLAND = "SoCal - Inland"
    NORCAL_BAY_AREA = "NorCal - Bay Area"
    NORCAL_CENTRAL = "NorCal - Central"
    ARIZONA_PHOENIX = "Arizona - Phoenix"
    ARIZONA_TUCSON = "Arizona - Tucson"
    NEVADA_LAS_VEGAS = "Nevada - Las Vegas"
    NEVADA_RENO = "Nevada - Reno"


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
    Analyze the following project description and classify it into the most appropriate reference class.
    
    Project Description: {description}
    Region: {region}
    
    Available Reference Classes: {reference_classes}
    
    Please provide:
    1. Primary reference class
    2. Confidence score (0-1)
    3. Reasoning for classification
    """,
    
    "ESTIMATION_GENERATION": """
    Based on the following project details and reference data, generate a comprehensive estimate.
    
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