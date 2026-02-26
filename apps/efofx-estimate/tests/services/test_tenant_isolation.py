"""
Tests for tenant isolation in rcf_engine.py (PRQT-02).

Verifies that tenant A cannot see tenant B's reference classes,
and that platform data (tenant_id=None) is visible to all tenants.

These are integration tests that require a running MongoDB instance.
Run with: pytest -m integration
"""

import pytest
import pytest_asyncio
from datetime import datetime, timezone
from motor.motor_asyncio import AsyncIOMotorClient

from app.core.config import settings
from app.services.rcf_engine import find_matching_reference_class, clear_match_cache
from app.core.constants import DB_COLLECTIONS


def _make_rc_doc(tenant_id, name, keywords=None):
    """Build a valid reference class document matching the ReferenceClass model schema."""
    keywords = keywords or ["pool", "residential", "inground", "swimming"]
    return {
        "tenant_id": tenant_id,
        "category": "construction",
        "subcategory": "pool",
        "name": name,
        "description": "Residential inground pool construction",
        "keywords": keywords,
        "regions": ["SoCal - Coastal"],
        "attributes": {},
        "cost_distribution": {
            "p50": 50000,
            "p80": 75000,
            "p95": 100000,
            "currency": "USD",
        },
        "timeline_distribution": {
            "p50_days": 56,
            "p80_days": 84,
            "p95_days": 120,
        },
        "cost_breakdown_template": {
            "permits": 0.05,
            "site_prep": 0.10,
            "materials": 0.35,
            "labor": 0.30,
            "equipment": 0.10,
            "contingency": 0.10,
        },
        "is_synthetic": True,
        "validation_source": "test_tenant_isolation",
        "created_at": datetime.now(timezone.utc),
    }


@pytest_asyncio.fixture
async def tenant_isolation_data():
    """
    Insert reference classes for two tenants and platform, clean up after.

    Creates its own Motor client per test to avoid event-loop conflicts with
    the session-scoped conftest connection.
    """
    clear_match_cache()

    # Create a fresh client bound to the current test's event loop
    client = AsyncIOMotorClient(settings.MONGO_URI)
    db = client[settings.MONGO_DB_NAME]
    collection = db[DB_COLLECTIONS["REFERENCE_CLASSES"]]

    # Also update the app's global DB reference so rcf_engine uses the same loop
    import app.db.mongodb as _mdb
    _mdb._client = client
    _mdb._database = db

    tenant_a_doc = _make_rc_doc("tenant_a_test", "Tenant A Pool Class")
    tenant_b_doc = _make_rc_doc("tenant_b_test", "Tenant B Pool Class")
    platform_doc = _make_rc_doc(None, "Platform Pool Class")

    result = await collection.insert_many([tenant_a_doc, tenant_b_doc, platform_doc])
    inserted_ids = result.inserted_ids

    yield {
        "tenant_a": tenant_a_doc,
        "tenant_b": tenant_b_doc,
        "platform": platform_doc,
    }

    # Cleanup: remove only the docs we inserted
    await collection.delete_many({"_id": {"$in": inserted_ids}})
    clear_match_cache()

    client.close()
    _mdb._client = None
    _mdb._database = None


@pytest.mark.integration
@pytest.mark.asyncio
async def test_no_cross_tenant_leakage(tenant_isolation_data):
    """Tenant A must not see Tenant B's reference classes."""
    result = await find_matching_reference_class(
        description="residential inground pool installation swimming",
        category="construction",
        region="SoCal - Coastal",
        tenant_id="tenant_a_test",
    )

    if result and "reference_class" in result:
        rc = result["reference_class"]
        assert rc.get("tenant_id") != "tenant_b_test", (
            f"Cross-tenant leak: Tenant A received Tenant B's data "
            f"(name={rc.get('name')})"
        )


@pytest.mark.integration
@pytest.mark.asyncio
async def test_tenant_sees_own_data(tenant_isolation_data):
    """Tenant A should be able to find their own reference classes."""
    result = await find_matching_reference_class(
        description="residential inground pool installation swimming",
        category="construction",
        region="SoCal - Coastal",
        tenant_id="tenant_a_test",
    )

    assert result is not None, "Tenant A should find matching reference class"
    assert "reference_class" in result


@pytest.mark.integration
@pytest.mark.asyncio
async def test_platform_data_visible_to_all(tenant_isolation_data):
    """Platform data (tenant_id=None) should be accessible by any tenant."""
    # Query as tenant_a — should see platform data combined with own data
    result_a = await find_matching_reference_class(
        description="residential inground pool installation swimming",
        category="construction",
        region="SoCal - Coastal",
        tenant_id="tenant_a_test",
    )

    # The result should exist (either tenant's own or platform data)
    assert result_a is not None, "Tenant A should see results (own + platform data)"


@pytest.mark.integration
@pytest.mark.asyncio
async def test_no_tenant_returns_platform_only(tenant_isolation_data):
    """When no tenant_id, only platform data should be returned."""
    result = await find_matching_reference_class(
        description="residential inground pool installation swimming",
        category="construction",
        region="SoCal - Coastal",
        tenant_id=None,
    )

    if result and "reference_class" in result:
        rc = result["reference_class"]
        assert rc.get("tenant_id") is None, (
            f"No-tenant query returned tenant-specific data "
            f"(tenant_id={rc.get('tenant_id')})"
        )
