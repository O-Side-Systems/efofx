"""
Unit tests for TenantAwareCollection (ISOL-01, ISOL-02, ISOL-04).

Verifies that every MongoDB operation automatically injects tenant_id
into filters and documents. Uses AsyncMock — no real DB needed.
"""

import pytest
from unittest.mock import AsyncMock, MagicMock, call

from app.db.tenant_collection import TenantAwareCollection


@pytest.fixture
def mock_collection():
    col = AsyncMock()
    col.find = MagicMock()  # find() returns cursor, not awaitable
    return col


# ---------------------------------------------------------------------------
# Construction / validation
# ---------------------------------------------------------------------------


def test_empty_tenant_id_raises(mock_collection):
    """Creating TenantAwareCollection with empty string tenant_id raises ValueError."""
    with pytest.raises(ValueError, match="tenant_id is required"):
        TenantAwareCollection(mock_collection, "")


def test_none_tenant_id_raises(mock_collection):
    """Creating TenantAwareCollection with None tenant_id raises ValueError."""
    with pytest.raises((ValueError, TypeError)):
        TenantAwareCollection(mock_collection, None)


def test_tenant_id_property(mock_collection):
    """tenant_id property returns the wrapped tenant_id."""
    col = TenantAwareCollection(mock_collection, "tenant_abc")
    assert col.tenant_id == "tenant_abc"


# ---------------------------------------------------------------------------
# find_one
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_find_one_injects_tenant_id(mock_collection):
    """find_one({}) with tenant_id='t1' queries {'tenant_id': 't1'}."""
    mock_collection.find_one.return_value = None
    col = TenantAwareCollection(mock_collection, "t1")

    await col.find_one({})

    mock_collection.find_one.assert_called_once_with({"tenant_id": "t1"})


@pytest.mark.asyncio
async def test_find_one_injects_tenant_id_no_filter(mock_collection):
    """find_one(None) should scope to tenant_id only."""
    mock_collection.find_one.return_value = None
    col = TenantAwareCollection(mock_collection, "t1")

    await col.find_one(None)

    mock_collection.find_one.assert_called_once_with({"tenant_id": "t1"})


@pytest.mark.asyncio
async def test_find_one_merges_with_existing_filter(mock_collection):
    """find_one({'status': 'active'}) with tenant_id='t1' uses $and."""
    mock_collection.find_one.return_value = None
    col = TenantAwareCollection(mock_collection, "t1")

    await col.find_one({"status": "active"})

    mock_collection.find_one.assert_called_once_with(
        {"$and": [{"tenant_id": "t1"}, {"status": "active"}]}
    )


# ---------------------------------------------------------------------------
# find
# ---------------------------------------------------------------------------


def test_find_injects_tenant_id(mock_collection):
    """find({}) with tenant_id='t1' scopes to tenant_id only."""
    col = TenantAwareCollection(mock_collection, "t1")

    col.find({})

    mock_collection.find.assert_called_once_with({"tenant_id": "t1"})


def test_find_merges_with_existing_filter(mock_collection):
    """find({'category': 'residential'}) uses $and with tenant_id."""
    col = TenantAwareCollection(mock_collection, "t1")

    col.find({"category": "residential"})

    mock_collection.find.assert_called_once_with(
        {"$and": [{"tenant_id": "t1"}, {"category": "residential"}]}
    )


# ---------------------------------------------------------------------------
# insert_one
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_insert_one_adds_tenant_id(mock_collection):
    """insert_one({'name': 'test'}) adds tenant_id before insert."""
    mock_collection.insert_one.return_value = MagicMock(inserted_id="abc")
    col = TenantAwareCollection(mock_collection, "t1")

    await col.insert_one({"name": "test"})

    mock_collection.insert_one.assert_called_once_with({"name": "test", "tenant_id": "t1"})


@pytest.mark.asyncio
async def test_insert_one_does_not_overwrite_tenant_id_from_doc(mock_collection):
    """
    Even if document already has tenant_id set, the wrapper overwrites it with its own.
    This prevents tenant injection attacks where caller sets a different tenant_id.
    """
    mock_collection.insert_one.return_value = MagicMock(inserted_id="abc")
    col = TenantAwareCollection(mock_collection, "t1")

    await col.insert_one({"name": "test", "tenant_id": "injected_tenant"})

    # The wrapper should override with its own tenant_id
    mock_collection.insert_one.assert_called_once_with({"name": "test", "tenant_id": "t1"})


# ---------------------------------------------------------------------------
# update_one
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_update_one_scoped(mock_collection):
    """update_one filter is scoped to tenant_id."""
    mock_collection.update_one.return_value = MagicMock(modified_count=1)
    col = TenantAwareCollection(mock_collection, "t1")

    await col.update_one({"name": "test"}, {"$set": {"status": "done"}})

    mock_collection.update_one.assert_called_once_with(
        {"$and": [{"tenant_id": "t1"}, {"name": "test"}]},
        {"$set": {"status": "done"}},
    )


@pytest.mark.asyncio
async def test_update_one_empty_filter_scoped(mock_collection):
    """update_one({}, ...) scopes to tenant_id only."""
    mock_collection.update_one.return_value = MagicMock(modified_count=1)
    col = TenantAwareCollection(mock_collection, "t1")

    await col.update_one({}, {"$set": {"status": "done"}})

    mock_collection.update_one.assert_called_once_with(
        {"tenant_id": "t1"},
        {"$set": {"status": "done"}},
    )


# ---------------------------------------------------------------------------
# delete_one
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_delete_one_scoped(mock_collection):
    """delete_one filter is scoped to tenant_id."""
    mock_collection.delete_one.return_value = MagicMock(deleted_count=1)
    col = TenantAwareCollection(mock_collection, "t1")

    await col.delete_one({"name": "test"})

    mock_collection.delete_one.assert_called_once_with(
        {"$and": [{"tenant_id": "t1"}, {"name": "test"}]}
    )


# ---------------------------------------------------------------------------
# count_documents
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_count_documents_scoped(mock_collection):
    """count_documents({}) counts only tenant_id='t1' documents."""
    mock_collection.count_documents.return_value = 5
    col = TenantAwareCollection(mock_collection, "t1")

    count = await col.count_documents({})

    mock_collection.count_documents.assert_called_once_with({"tenant_id": "t1"})
    assert count == 5


@pytest.mark.asyncio
async def test_count_documents_with_filter(mock_collection):
    """count_documents with additional filter uses $and."""
    mock_collection.count_documents.return_value = 3
    col = TenantAwareCollection(mock_collection, "t1")

    count = await col.count_documents({"is_active": True})

    mock_collection.count_documents.assert_called_once_with(
        {"$and": [{"tenant_id": "t1"}, {"is_active": True}]}
    )
    assert count == 3


# ---------------------------------------------------------------------------
# Platform data mode (allow_platform_data=True)
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_platform_data_mode_empty_filter(mock_collection):
    """
    With allow_platform_data=True and no extra filter, query uses $or:
    [{tenant_id: 't1'}, {tenant_id: None}].
    """
    mock_collection.find_one.return_value = None
    col = TenantAwareCollection(mock_collection, "t1", allow_platform_data=True)

    await col.find_one({})

    mock_collection.find_one.assert_called_once_with(
        {"$or": [{"tenant_id": "t1"}, {"tenant_id": None}]}
    )


@pytest.mark.asyncio
async def test_platform_data_with_existing_filter(mock_collection):
    """
    With allow_platform_data=True and extra filter, $or is combined with $and:
    {'$and': [{'$or': [...]}, {extra_filter}]}.
    """
    mock_collection.find_one.return_value = None
    col = TenantAwareCollection(mock_collection, "t1", allow_platform_data=True)

    await col.find_one({"category": "residential"})

    mock_collection.find_one.assert_called_once_with(
        {
            "$and": [
                {"$or": [{"tenant_id": "t1"}, {"tenant_id": None}]},
                {"category": "residential"},
            ]
        }
    )


def test_platform_data_mode_find(mock_collection):
    """find() in platform data mode uses $or filter."""
    col = TenantAwareCollection(mock_collection, "t1", allow_platform_data=True)

    col.find({"category": "residential"})

    mock_collection.find.assert_called_once_with(
        {
            "$and": [
                {"$or": [{"tenant_id": "t1"}, {"tenant_id": None}]},
                {"category": "residential"},
            ]
        }
    )


# ---------------------------------------------------------------------------
# aggregate
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_aggregate_injects_match(mock_collection):
    """aggregate() prepends a $match stage with tenant_id filter."""
    mock_collection.aggregate = MagicMock()  # aggregate returns cursor, not awaitable
    col = TenantAwareCollection(mock_collection, "t1")

    pipeline = [{"$group": {"_id": "$region", "count": {"$sum": 1}}}]
    await col.aggregate(pipeline)

    expected_pipeline = [
        {"$match": {"tenant_id": "t1"}},
        {"$group": {"_id": "$region", "count": {"$sum": 1}}},
    ]
    mock_collection.aggregate.assert_called_once_with(expected_pipeline)


@pytest.mark.asyncio
async def test_aggregate_platform_mode_injects_or_match(mock_collection):
    """aggregate() in platform data mode prepends $or $match."""
    mock_collection.aggregate = MagicMock()
    col = TenantAwareCollection(mock_collection, "t1", allow_platform_data=True)

    pipeline = [{"$group": {"_id": "$region", "count": {"$sum": 1}}}]
    await col.aggregate(pipeline)

    expected_pipeline = [
        {"$match": {"$or": [{"tenant_id": "t1"}, {"tenant_id": None}]}},
        {"$group": {"_id": "$region", "count": {"$sum": 1}}},
    ]
    mock_collection.aggregate.assert_called_once_with(expected_pipeline)


# ---------------------------------------------------------------------------
# update_many / delete_many
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_update_many_scoped(mock_collection):
    """update_many filter is scoped to tenant_id."""
    mock_collection.update_many.return_value = MagicMock(modified_count=3)
    col = TenantAwareCollection(mock_collection, "t1")

    await col.update_many({"status": "old"}, {"$set": {"status": "archived"}})

    mock_collection.update_many.assert_called_once_with(
        {"$and": [{"tenant_id": "t1"}, {"status": "old"}]},
        {"$set": {"status": "archived"}},
    )


@pytest.mark.asyncio
async def test_delete_many_scoped(mock_collection):
    """delete_many filter is scoped to tenant_id."""
    mock_collection.delete_many.return_value = MagicMock(deleted_count=5)
    col = TenantAwareCollection(mock_collection, "t1")

    await col.delete_many({"is_expired": True})

    mock_collection.delete_many.assert_called_once_with(
        {"$and": [{"tenant_id": "t1"}, {"is_expired": True}]}
    )
