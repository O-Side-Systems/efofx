"""
Unit tests for refactored TenantService methods.

Verifies that:
- get_tenant_statistics uses get_tenant_collection() (not deprecated accessors)
- validate_tenant_limits uses get_tenant_collection() with string UUIDs
- get_all_tenant_statistics uses get_collection() for raw cross-tenant access
- No ObjectId usage in any tenant-scoped methods
"""

import pytest
from unittest.mock import AsyncMock, patch, MagicMock, call
import uuid

from app.core.constants import DB_COLLECTIONS


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def make_tenant_doc(tenant_id: str = None, tier: str = "trial", is_active: bool = True) -> dict:
    """Return a minimal tenant document dict (matches MongoDB raw dict, not Tenant model)."""
    return {
        "tenant_id": tenant_id or str(uuid.uuid4()),
        "company_name": "Test Co",
        "email": "test@example.com",
        "hashed_password": "bcrypt-hash",
        "hashed_api_key": "bcrypt-hash",
        "api_key_last6": "abc123",
        "tier": tier,
        "email_verified": True,
        "is_active": is_active,
        "settings": {},
    }


def make_tenant_collection_mock() -> AsyncMock:
    """Return an AsyncMock matching TenantAwareCollection interface."""
    mock = AsyncMock()
    mock.count_documents = AsyncMock(return_value=0)
    # aggregate returns a cursor-like object whose to_list is a coroutine
    agg_cursor = MagicMock()
    agg_cursor.to_list = AsyncMock(return_value=[])
    mock.aggregate = MagicMock(return_value=agg_cursor)
    return mock


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def tenant_id() -> str:
    return str(uuid.uuid4())


@pytest.fixture
def tenant_service():
    """TenantService with mocked top-level tenants collection."""
    with patch("app.services.tenant_service.get_tenants_collection") as mock_col_fn:
        mock_col_fn.return_value = AsyncMock()
        from app.services.tenant_service import TenantService
        service = TenantService()
        yield service


# ---------------------------------------------------------------------------
# get_tenant_statistics
# ---------------------------------------------------------------------------

@pytest.mark.asyncio
async def test_get_tenant_statistics_uses_tenant_collection(tenant_id):
    """get_tenant_statistics calls get_tenant_collection for estimates AND feedback."""
    estimates_mock = make_tenant_collection_mock()
    feedback_mock = make_tenant_collection_mock()

    def side_effect(collection_name, tid, **kwargs):
        if collection_name == DB_COLLECTIONS["ESTIMATES"]:
            return estimates_mock
        if collection_name == DB_COLLECTIONS["FEEDBACK"]:
            return feedback_mock
        raise ValueError(f"Unexpected collection: {collection_name}")

    tenant_doc = make_tenant_doc(tenant_id)

    with patch("app.services.tenant_service.get_tenants_collection"):
        with patch("app.services.tenant_service.get_tenant_collection", side_effect=side_effect) as mock_gtc:
            with patch("app.services.tenant_service.get_collection"):
                from app.services.tenant_service import TenantService
                service = TenantService()
                # Stub get_by_tenant_id so it returns a tenant doc
                service.get_by_tenant_id = AsyncMock(return_value=tenant_doc)

                await service.get_tenant_statistics(tenant_id)

                # Must have been called with ESTIMATES + tenant_id
                mock_gtc.assert_any_call(DB_COLLECTIONS["ESTIMATES"], tenant_id)
                # Must have been called with FEEDBACK + tenant_id
                mock_gtc.assert_any_call(DB_COLLECTIONS["FEEDBACK"], tenant_id)


@pytest.mark.asyncio
async def test_get_tenant_statistics_no_objectid(tenant_id):
    """get_tenant_statistics filter for count_documents does NOT include ObjectId."""
    estimates_mock = make_tenant_collection_mock()
    feedback_mock = make_tenant_collection_mock()

    captured_filters = []

    async def capture_count(filter_doc, **kwargs):
        captured_filters.append(filter_doc)
        return 5

    estimates_mock.count_documents = capture_count
    feedback_mock.count_documents = AsyncMock(return_value=3)

    def side_effect(collection_name, tid, **kwargs):
        if collection_name == DB_COLLECTIONS["ESTIMATES"]:
            return estimates_mock
        return feedback_mock

    tenant_doc = make_tenant_doc(tenant_id)

    with patch("app.services.tenant_service.get_tenants_collection"):
        with patch("app.services.tenant_service.get_tenant_collection", side_effect=side_effect):
            with patch("app.services.tenant_service.get_collection"):
                from app.services.tenant_service import TenantService
                service = TenantService()
                service.get_by_tenant_id = AsyncMock(return_value=tenant_doc)

                await service.get_tenant_statistics(tenant_id)

                # All captured count filters must NOT contain ObjectId values
                for f in captured_filters:
                    assert "tenant_id" not in f, (
                        "TenantAwareCollection auto-injects tenant_id — filter must NOT include it"
                    )
                    for v in f.values():
                        assert "ObjectId" not in str(type(v)), (
                            f"Filter value {v!r} should not be ObjectId"
                        )


@pytest.mark.asyncio
async def test_get_tenant_statistics_returns_correct_shape(tenant_id):
    """get_tenant_statistics returns dict with all required keys."""
    estimates_mock = make_tenant_collection_mock()
    estimates_mock.count_documents = AsyncMock(return_value=12)

    feedback_mock = make_tenant_collection_mock()
    feedback_mock.count_documents = AsyncMock(return_value=5)
    agg_cursor = MagicMock()
    agg_cursor.to_list = AsyncMock(return_value=[{"avg_rating": 4.2}])
    feedback_mock.aggregate = MagicMock(return_value=agg_cursor)

    def side_effect(collection_name, tid, **kwargs):
        if collection_name == DB_COLLECTIONS["ESTIMATES"]:
            return estimates_mock
        return feedback_mock

    tenant_doc = make_tenant_doc(tenant_id, tier="paid")

    with patch("app.services.tenant_service.get_tenants_collection"):
        with patch("app.services.tenant_service.get_tenant_collection", side_effect=side_effect):
            with patch("app.services.tenant_service.get_collection"):
                from app.services.tenant_service import TenantService
                service = TenantService()
                service.get_by_tenant_id = AsyncMock(return_value=tenant_doc)

                result = await service.get_tenant_statistics(tenant_id)

                assert "estimations_last_30_days" in result
                assert "total_feedback" in result
                assert "average_rating" in result
                assert "active_regions" in result
                assert "monthly_limit" in result
                assert result["estimations_last_30_days"] == 12
                assert result["total_feedback"] == 5
                assert result["average_rating"] == 4.2


# ---------------------------------------------------------------------------
# validate_tenant_limits
# ---------------------------------------------------------------------------

@pytest.mark.asyncio
async def test_validate_tenant_limits_uses_tenant_collection(tenant_id):
    """validate_tenant_limits calls get_tenant_collection for estimates."""
    estimates_mock = make_tenant_collection_mock()
    tenant_doc = make_tenant_doc(tenant_id, tier="trial")

    with patch("app.services.tenant_service.get_tenants_collection"):
        with patch("app.services.tenant_service.get_tenant_collection", return_value=estimates_mock) as mock_gtc:
            with patch("app.services.tenant_service.get_collection"):
                from app.services.tenant_service import TenantService
                service = TenantService()
                service.get_by_tenant_id = AsyncMock(return_value=tenant_doc)

                await service.validate_tenant_limits(tenant_id)

                mock_gtc.assert_called_once_with(DB_COLLECTIONS["ESTIMATES"], tenant_id)


@pytest.mark.asyncio
async def test_validate_tenant_limits_returns_correct_shape(tenant_id):
    """validate_tenant_limits returns dict with all required keys."""
    estimates_mock = make_tenant_collection_mock()
    estimates_mock.count_documents = AsyncMock(return_value=30)
    tenant_doc = make_tenant_doc(tenant_id, tier="paid")

    with patch("app.services.tenant_service.get_tenants_collection"):
        with patch("app.services.tenant_service.get_tenant_collection", return_value=estimates_mock):
            with patch("app.services.tenant_service.get_collection"):
                from app.services.tenant_service import TenantService
                service = TenantService()
                service.get_by_tenant_id = AsyncMock(return_value=tenant_doc)

                result = await service.validate_tenant_limits(tenant_id)

                assert "monthly_usage" in result
                assert "monthly_limit" in result
                assert "limit_exceeded" in result
                assert "remaining_estimations" in result
                assert "is_active" in result
                assert result["monthly_usage"] == 30
                # paid tier default limit is 1000
                assert result["monthly_limit"] == 1000
                assert result["limit_exceeded"] is False
                assert result["remaining_estimations"] == 970


# ---------------------------------------------------------------------------
# get_all_tenant_statistics
# ---------------------------------------------------------------------------

@pytest.mark.asyncio
async def test_get_all_tenant_statistics_uses_raw_collection():
    """get_all_tenant_statistics calls get_collection (NOT get_tenant_collection)."""
    estimates_raw_mock = AsyncMock()
    estimates_raw_mock.count_documents = AsyncMock(return_value=100)
    feedback_raw_mock = AsyncMock()
    feedback_raw_mock.count_documents = AsyncMock(return_value=20)

    tenants_col_mock = AsyncMock()
    tenants_col_mock.count_documents = AsyncMock(return_value=5)

    def raw_side_effect(collection_name):
        if collection_name == DB_COLLECTIONS["ESTIMATES"]:
            return estimates_raw_mock
        return feedback_raw_mock

    with patch("app.services.tenant_service.get_tenants_collection", return_value=tenants_col_mock):
        with patch("app.services.tenant_service.get_tenant_collection") as mock_gtc:
            with patch("app.services.tenant_service.get_collection", side_effect=raw_side_effect) as mock_gc:
                from app.services.tenant_service import TenantService
                service = TenantService()

                await service.get_all_tenant_statistics()

                # Raw get_collection must be called for ESTIMATES and FEEDBACK
                mock_gc.assert_any_call(DB_COLLECTIONS["ESTIMATES"])
                mock_gc.assert_any_call(DB_COLLECTIONS["FEEDBACK"])

                # Tenant-scoped get_tenant_collection must NOT be called
                mock_gtc.assert_not_called()


# ---------------------------------------------------------------------------
# get_tenant (UUID string lookup)
# ---------------------------------------------------------------------------

@pytest.mark.asyncio
async def test_get_tenant_uses_tenant_id_string(tenant_id):
    """get_tenant uses {"tenant_id": tenant_id} filter — no ObjectId."""
    tenants_col_mock = AsyncMock()
    tenant_doc = make_tenant_doc(tenant_id)
    tenants_col_mock.find_one = AsyncMock(return_value=tenant_doc)

    with patch("app.services.tenant_service.get_tenants_collection", return_value=tenants_col_mock):
        from app.services.tenant_service import TenantService
        service = TenantService()

        result = await service.get_tenant(tenant_id)

        # find_one must have been called with string tenant_id filter
        tenants_col_mock.find_one.assert_called_once_with({"tenant_id": tenant_id})
        # Result is the raw dict (not a Tenant model)
        assert result == tenant_doc
