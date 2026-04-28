"""
Pytest configuration and fixtures for efOfX Estimation Service.

This module provides shared fixtures and configuration for all tests.

Session-scoped async fixtures use loop_scope="session" to share a single
event loop for the entire test session (required for test_db and other
session-scoped async fixtures).
"""

import pytest
import pytest_asyncio
from typing import AsyncGenerator
from fastapi.testclient import TestClient
from motor.motor_asyncio import AsyncIOMotorClient

from app.core.config import settings
from app.db.mongodb import connect_to_mongo, close_mongo_connection


@pytest_asyncio.fixture(scope="session", loop_scope="session")
async def test_db():
    """Create test database connection (session-scoped)."""
    # Use test database
    settings.MONGO_DB_NAME = "efofx_estimate_test"
    await connect_to_mongo()
    yield
    await close_mongo_connection()


@pytest_asyncio.fixture(loop_scope="function")
async def client(test_db) -> AsyncGenerator[TestClient, None]:
    """Create test client with database connection."""
    # Import app here to avoid database initialization at module level
    from app.main import app
    with TestClient(app) as test_client:
        yield test_client


@pytest.fixture
def sample_tenant():
    """Sample tenant data for testing."""
    return {
        "name": "Test Construction Co",
        "api_key": "sk_test_123456789",
        "openai_api_key": "sk-openai-test-key",
        "regions": ["SoCal - Coastal", "NorCal - Bay Area"],
        "max_estimations_per_month": 1000,
        "is_active": True,
        "settings": {
            "default_confidence_threshold": 0.7,
            "enable_image_upload": True
        }
    }


@pytest.fixture
def sample_estimation_request():
    """Sample estimation request for testing."""
    return {
        "description": "I want to install a 15x30 foot pool with spa in my backyard.",
        "region": "SoCal - Coastal",
        "reference_class": "residential_pool",
        "confidence_threshold": 0.7
    }


@pytest.fixture
def sample_reference_class():
    """Sample reference class data for testing."""
    return {
        "name": "residential_pool",
        "category": "residential",
        "description": "Residential swimming pool installation",
        "keywords": ["pool", "swimming", "backyard", "residential"],
        "regions": ["SoCal - Coastal", "SoCal - Inland"],
        "base_cost_per_sqft": {
            "SoCal - Coastal": 150.0,
            "SoCal - Inland": 140.0
        },
        "cost_breakdown_template": {
            "materials": 0.40,
            "labor": 0.25,
            "equipment": 0.08,
            "permits": 0.03,
            "design": 0.05,
            "contingency": 0.08,
            "profit_margin": 0.11
        },
        "timeline_multiplier": 1.0,
        "team_size_template": {
            "small": 3,
            "medium": 4,
            "large": 6
        },
        "tuning_factors": {
            "SoCal - Coastal": 1.1,
            "SoCal - Inland": 1.0
        },
        "is_active": True
    }


@pytest.fixture
def sample_reference_project():
    """Sample reference project data for testing."""
    return {
        "project_id": "pool_001",
        "reference_class": "residential_pool",
        "region": "SoCal - Coastal",
        "description": "15x30 foot pool with spa installation",
        "total_cost": 45000.0,
        "timeline_weeks": 8,
        "team_size": 4,
        "cost_breakdown": {
            "materials": 18000.0,
            "labor": 11250.0,
            "equipment": 3600.0,
            "permits": 1350.0,
            "design": 2250.0,
            "contingency": 3600.0,
            "profit_margin": 4950.0
        },
        "quality_score": 0.85,
        "completion_date": "2024-01-15",
        "metadata": {
            "pool_size": "15x30",
            "spa_included": True,
            "decking_material": "concrete",
            "filter_type": "sand"
        },
        "is_active": True
    }


@pytest.fixture
def sample_feedback():
    """Sample feedback data for testing."""
    return {
        "estimation_session_id": "session_123",
        "tenant_id": "tenant_456",
        "feedback_type": "accuracy",
        "rating": 4,
        "comment": "Estimate was very close to actual cost",
        "metadata": {
            "actual_cost": 44000.0,
            "estimated_cost": 45000.0,
            "variance_percentage": 2.2
        }
    }
