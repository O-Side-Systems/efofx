"""
TenantAwareCollection — Hard tenant isolation for MongoDB operations.

Every MongoDB operation automatically injects the tenant_id into filters
and documents, making cross-tenant data leakage structurally impossible.

Usage:
    from app.db.tenant_collection import TenantAwareCollection

    col = TenantAwareCollection(raw_motor_collection, tenant_id="acme-corp")
    doc = await col.find_one({"status": "active"})  # auto-injects tenant_id

Platform data (tenant_id=None) is accessible via allow_platform_data=True:
    col = TenantAwareCollection(raw_collection, "acme-corp", allow_platform_data=True)
    docs = col.find({"category": "residential"})  # sees own + platform data
"""

from motor.motor_asyncio import AsyncIOMotorCollection
from typing import Any, Optional


class TenantAwareCollection:
    """
    Wraps Motor AsyncIOMotorCollection to enforce tenant isolation on every operation.

    All query filters and inserted documents are automatically scoped to the
    provided tenant_id. The wrapper cannot be created without a valid tenant_id,
    ensuring isolation is enforced at construction time.
    """

    def __init__(
        self,
        collection: AsyncIOMotorCollection,
        tenant_id: str,
        allow_platform_data: bool = False,
    ) -> None:
        """
        Create a tenant-scoped collection wrapper.

        Args:
            collection: Raw Motor collection to wrap.
            tenant_id: The tenant's ID — required and must be non-empty.
            allow_platform_data: If True, queries use $or to include platform
                data (tenant_id=None) alongside the tenant's own data.

        Raises:
            ValueError: If tenant_id is empty or None.
        """
        if not tenant_id:
            raise ValueError(
                "tenant_id is required for TenantAwareCollection. "
                "Provide a valid non-empty tenant_id."
            )
        self._col = collection
        self._tenant_id = tenant_id
        self._allow_platform = allow_platform_data

    @property
    def tenant_id(self) -> str:
        """The tenant_id this collection is scoped to."""
        return self._tenant_id

    def _scoped_filter(self, filter: dict[str, Any] | None = None) -> dict[str, Any]:
        """
        Build a tenant-scoped MongoDB filter.

        If allow_platform_data=True, uses $or to include both the tenant's data
        and platform data (tenant_id=None). Otherwise uses a simple equality match.

        If an additional filter is provided, wraps both in $and.
        """
        if self._allow_platform:
            tenant_filter: dict[str, Any] = {
                "$or": [
                    {"tenant_id": self._tenant_id},
                    {"tenant_id": None},
                ]
            }
        else:
            tenant_filter = {"tenant_id": self._tenant_id}

        if filter:
            return {"$and": [tenant_filter, filter]}
        return tenant_filter

    # ------------------------------------------------------------------
    # Read operations
    # ------------------------------------------------------------------

    async def find_one(
        self, filter: dict[str, Any] | None = None, **kwargs: Any
    ) -> Optional[dict[str, Any]]:
        """Find a single document scoped to this tenant."""
        return await self._col.find_one(self._scoped_filter(filter), **kwargs)

    def find(self, filter: dict[str, Any] | None = None, **kwargs: Any):
        """Find documents scoped to this tenant. Returns Motor cursor."""
        return self._col.find(self._scoped_filter(filter), **kwargs)

    async def count_documents(
        self, filter: dict[str, Any] | None = None, **kwargs: Any
    ) -> int:
        """Count documents scoped to this tenant."""
        return await self._col.count_documents(self._scoped_filter(filter), **kwargs)

    async def aggregate(self, pipeline: list[dict[str, Any]], **kwargs: Any):
        """
        Run an aggregation pipeline scoped to this tenant.

        Prepends a $match stage with the tenant filter so all subsequent
        pipeline stages operate only on this tenant's data.
        """
        tenant_match = {"$match": self._scoped_filter()}
        scoped_pipeline = [tenant_match] + pipeline
        return self._col.aggregate(scoped_pipeline, **kwargs)

    # ------------------------------------------------------------------
    # Write operations
    # ------------------------------------------------------------------

    async def insert_one(self, document: dict[str, Any], **kwargs: Any):
        """
        Insert a single document, always stamping it with this tenant's tenant_id.

        Even if the document already contains a tenant_id field, it is overwritten
        to prevent injection attacks where a caller sets a different tenant_id.
        """
        document["tenant_id"] = self._tenant_id
        return await self._col.insert_one(document, **kwargs)

    async def insert_many(self, documents: list[dict[str, Any]], **kwargs: Any):
        """Insert multiple documents, stamping all with this tenant's tenant_id."""
        for doc in documents:
            doc["tenant_id"] = self._tenant_id
        return await self._col.insert_many(documents, **kwargs)

    async def update_one(
        self,
        filter: dict[str, Any],
        update: dict[str, Any],
        **kwargs: Any,
    ):
        """Update a single document scoped to this tenant."""
        return await self._col.update_one(self._scoped_filter(filter), update, **kwargs)

    async def update_many(
        self,
        filter: dict[str, Any],
        update: dict[str, Any],
        **kwargs: Any,
    ):
        """Update multiple documents scoped to this tenant."""
        return await self._col.update_many(
            self._scoped_filter(filter), update, **kwargs
        )

    async def delete_one(self, filter: dict[str, Any], **kwargs: Any):
        """Delete a single document scoped to this tenant."""
        return await self._col.delete_one(self._scoped_filter(filter), **kwargs)

    async def delete_many(self, filter: dict[str, Any], **kwargs: Any):
        """Delete multiple documents scoped to this tenant."""
        return await self._col.delete_many(self._scoped_filter(filter), **kwargs)
