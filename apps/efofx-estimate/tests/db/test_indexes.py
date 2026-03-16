"""
Integration tests for compound index creation (ISOL-03).

Verifies that create_indexes() creates compound indexes with tenant_id as the
first field on all tenant-scoped collections.

Marked as integration tests — require a running MongoDB instance.
Run with: pytest -m integration
"""

import pytest
import pytest_asyncio
from motor.motor_asyncio import AsyncIOMotorClient

from app.core.config import settings
from app.db.mongodb import create_indexes


@pytest_asyncio.fixture
async def db_with_indexes():
    """
    Connect to the test DB, run create_indexes(), yield db, then clean up.

    Creates its own Motor client per test to avoid event-loop conflicts.
    """
    client = AsyncIOMotorClient(settings.MONGO_URI)
    db = client[settings.MONGO_DB_NAME]

    # Inject into app globals so create_indexes() uses the same loop
    import app.db.mongodb as _mdb
    _orig_client = _mdb._client
    _orig_database = _mdb._database
    _mdb._client = client
    _mdb._database = db

    await create_indexes()
    yield db

    # Restore original state
    _mdb._client = _orig_client
    _mdb._database = _orig_database
    client.close()


def _first_key(index_info: dict, index_name: str) -> str | None:
    """Return the first key of a named index, or None if not found."""
    for name, info in index_info.items():
        if name == index_name:
            keys = info.get("key", [])
            if keys:
                return keys[0][0]  # first (field_name, direction) tuple
    return None


def _has_index_with_first_field(index_info: dict, first_field: str) -> bool:
    """Return True if any compound index has first_field as the leftmost key."""
    for name, info in index_info.items():
        if name == "_id_":
            continue
        keys = info.get("key", [])
        if keys and keys[0][0] == first_field:
            return True
    return False


@pytest.mark.integration
@pytest.mark.asyncio
async def test_estimates_has_tenant_id_compound_index(db_with_indexes):
    """Estimates collection has at least one compound index with tenant_id first."""
    index_info = await db_with_indexes["estimates"].index_information()
    assert _has_index_with_first_field(index_info, "tenant_id"), (
        f"Expected compound index with tenant_id first in estimates.\n"
        f"Indexes found: {list(index_info.keys())}"
    )


@pytest.mark.integration
@pytest.mark.asyncio
async def test_estimates_has_session_id_unique_index(db_with_indexes):
    """Estimates collection has (tenant_id, session_id) unique compound index."""
    index_info = await db_with_indexes["estimates"].index_information()
    found = False
    for name, info in index_info.items():
        keys = [k[0] for k in info.get("key", [])]
        if keys == ["tenant_id", "session_id"] and info.get("unique"):
            found = True
            break
    assert found, (
        "Expected unique (tenant_id, session_id) index on estimates.\n"
        f"Indexes: {list(index_info.keys())}"
    )


@pytest.mark.integration
@pytest.mark.asyncio
async def test_reference_classes_has_tenant_id_compound_index(db_with_indexes):
    """Reference classes collection has at least one compound index with tenant_id first."""
    index_info = await db_with_indexes["reference_classes"].index_information()
    assert _has_index_with_first_field(index_info, "tenant_id"), (
        f"Expected compound index with tenant_id first in reference_classes.\n"
        f"Indexes found: {list(index_info.keys())}"
    )


@pytest.mark.integration
@pytest.mark.asyncio
async def test_reference_projects_has_tenant_id_compound_index(db_with_indexes):
    """Reference projects collection has at least one compound index with tenant_id first."""
    index_info = await db_with_indexes["reference_projects"].index_information()
    assert _has_index_with_first_field(index_info, "tenant_id"), (
        f"Expected compound index with tenant_id first in reference_projects.\n"
        f"Indexes found: {list(index_info.keys())}"
    )


@pytest.mark.integration
@pytest.mark.asyncio
async def test_feedback_has_tenant_id_compound_index(db_with_indexes):
    """Feedback collection has at least one compound index with tenant_id first."""
    index_info = await db_with_indexes["feedback"].index_information()
    assert _has_index_with_first_field(index_info, "tenant_id"), (
        f"Expected compound index with tenant_id first in feedback.\n"
        f"Indexes found: {list(index_info.keys())}"
    )


@pytest.mark.integration
@pytest.mark.asyncio
async def test_chat_sessions_has_tenant_id_compound_index(db_with_indexes):
    """Chat sessions collection has (tenant_id, session_id) unique compound index."""
    index_info = await db_with_indexes["chat_sessions"].index_information()
    found = False
    for name, info in index_info.items():
        keys = [k[0] for k in info.get("key", [])]
        if keys == ["tenant_id", "session_id"] and info.get("unique"):
            found = True
            break
    assert found, (
        "Expected unique (tenant_id, session_id) index on chat_sessions.\n"
        f"Indexes: {list(index_info.keys())}"
    )


@pytest.mark.integration
@pytest.mark.asyncio
async def test_tenants_has_email_unique_index(db_with_indexes):
    """Tenants collection has unique email index (top-level entity, not tenant-scoped)."""
    index_info = await db_with_indexes["tenants"].index_information()
    found = False
    for name, info in index_info.items():
        keys = [k[0] for k in info.get("key", [])]
        if keys == ["email"] and info.get("unique"):
            found = True
            break
    assert found, (
        "Expected unique email index on tenants.\n"
        f"Indexes: {list(index_info.keys())}"
    )
