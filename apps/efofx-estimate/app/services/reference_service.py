"""
Reference service for efOfX Estimation Service.

This module provides functionality for managing reference classes
and reference projects used in RCF estimation.
"""

import logging
from typing import List, Dict, Any, Optional
from datetime import datetime

from app.models.reference import ReferenceClass, ReferenceProject
from app.db.mongodb import get_tenant_collection, get_collection
from app.core.constants import DB_COLLECTIONS

logger = logging.getLogger(__name__)


class ReferenceService:
    """
    Service for handling reference classes and projects.

    Reference data can be either:
    - Platform-provided (tenant_id=None): visible to all tenants
    - Tenant-specific (tenant_id=<id>): visible only to that tenant

    Read methods accept an optional tenant_id and use allow_platform_data=True
    so tenants see both their own data and platform data.
    Write/admin methods require a tenant_id for proper scoping.
    """

    def _classes_collection(self, tenant_id: Optional[str] = None, allow_platform: bool = True):
        """
        Get reference classes collection.

        If tenant_id is provided, returns TenantAwareCollection with platform data.
        If no tenant_id, returns raw collection (for platform admin operations only).
        """
        if tenant_id:
            return get_tenant_collection(
                DB_COLLECTIONS["REFERENCE_CLASSES"],
                tenant_id,
                allow_platform_data=allow_platform,
            )
        # No tenant context — use raw collection (caller must apply own filters)
        return get_collection(DB_COLLECTIONS["REFERENCE_CLASSES"])

    def _projects_collection(self, tenant_id: Optional[str] = None, allow_platform: bool = True):
        """
        Get reference projects collection.

        If tenant_id is provided, returns TenantAwareCollection with platform data.
        If no tenant_id, returns raw collection (for platform admin operations only).
        """
        if tenant_id:
            return get_tenant_collection(
                DB_COLLECTIONS["REFERENCE_PROJECTS"],
                tenant_id,
                allow_platform_data=allow_platform,
            )
        return get_collection(DB_COLLECTIONS["REFERENCE_PROJECTS"])

    async def get_reference_classes(
        self,
        category: Optional[str] = None,
        tenant_id: Optional[str] = None,
    ) -> List[ReferenceClass]:
        """
        Get reference classes, optionally filtered by category.

        When tenant_id is provided, returns both tenant-specific and platform classes.
        When no tenant_id, returns platform classes only (tenant_id=None).
        """
        try:
            collection = self._classes_collection(tenant_id)
            query: Dict[str, Any] = {"is_active": True}
            if category:
                query["category"] = category
            if not tenant_id:
                # Platform-only query (no TenantAwareCollection wrapping)
                query["tenant_id"] = None

            cursor = collection.find(query)
            classes_list = await cursor.to_list(length=None)

            return [ReferenceClass(**cls) for cls in classes_list]

        except Exception as e:
            logger.error(f"Error getting reference classes: {e}")
            raise

    async def list_reference_classes(
        self,
        category: Optional[str] = None,
        tenant_id: Optional[str] = None,
    ) -> List[ReferenceClass]:
        """Alias for get_reference_classes() — used by API routes."""
        return await self.get_reference_classes(category=category, tenant_id=tenant_id)

    async def get_reference_class(
        self,
        name: str,
        tenant_id: Optional[str] = None,
    ) -> Optional[ReferenceClass]:
        """Get specific reference class by name."""
        try:
            collection = self._classes_collection(tenant_id)
            query: Dict[str, Any] = {"name": name, "is_active": True}
            if not tenant_id:
                query["tenant_id"] = None
            class_data = await collection.find_one(query)

            if class_data:
                return ReferenceClass(**class_data)
            return None

        except Exception as e:
            logger.error(f"Error getting reference class: {e}")
            raise

    async def get_reference_projects(
        self,
        reference_class: str,
        region: str,
        limit: int = 10,
        tenant_id: Optional[str] = None,
    ) -> List[Dict[str, Any]]:
        """Get reference projects for a specific class and region."""
        try:
            collection = self._projects_collection(tenant_id)
            query: Dict[str, Any] = {
                "reference_class": reference_class,
                "region": region,
                "is_active": True,
            }
            if not tenant_id:
                query["tenant_id"] = None

            cursor = collection.find(query).sort("quality_score", -1).limit(limit)
            projects_list = await cursor.to_list(length=None)

            return [
                {
                    "project_id": p.get("project_id"),
                    "total_cost": p.get("total_cost"),
                    "timeline_weeks": p.get("timeline_weeks"),
                    "team_size": p.get("team_size"),
                    "cost_breakdown": p.get("cost_breakdown"),
                    "quality_score": p.get("quality_score"),
                    "description": p.get("description"),
                    "metadata": p.get("metadata", {}),
                }
                for p in projects_list
            ]

        except Exception as e:
            logger.error(f"Error getting reference projects: {e}")
            return []

    async def get_reference_project(
        self,
        project_id: str,
        tenant_id: Optional[str] = None,
    ) -> Optional[ReferenceProject]:
        """Get specific reference project by ID."""
        try:
            collection = self._projects_collection(tenant_id)
            query: Dict[str, Any] = {"project_id": project_id, "is_active": True}
            if not tenant_id:
                query["tenant_id"] = None
            project_data = await collection.find_one(query)

            if project_data:
                return ReferenceProject(**project_data)
            return None

        except Exception as e:
            logger.error(f"Error getting reference project: {e}")
            raise

    async def create_reference_class(
        self,
        class_data: Dict[str, Any],
        tenant_id: Optional[str] = None,
    ) -> str:
        """
        Create a new reference class.

        If tenant_id is provided, the class is scoped to that tenant.
        If no tenant_id, the class is platform-provided (admin use only).
        """
        try:
            if tenant_id:
                collection = get_tenant_collection(
                    DB_COLLECTIONS["REFERENCE_CLASSES"], tenant_id, allow_platform_data=False
                )
            else:
                collection = get_collection(DB_COLLECTIONS["REFERENCE_CLASSES"])

            existing = await collection.find_one({"name": class_data["name"]})
            if existing:
                raise ValueError("Reference class already exists")

            reference_class = ReferenceClass(**class_data)
            result = await collection.insert_one(reference_class.dict(by_alias=True))

            logger.info(f"Reference class created: {result.inserted_id}")
            return str(result.inserted_id)

        except Exception as e:
            logger.error(f"Error creating reference class: {e}")
            raise

    async def create_reference_project(
        self,
        project_data: Dict[str, Any],
        tenant_id: Optional[str] = None,
    ) -> str:
        """
        Create a new reference project.

        If tenant_id is provided, the project is scoped to that tenant.
        If no tenant_id, the project is platform-provided (admin use only).
        """
        try:
            if tenant_id:
                collection = get_tenant_collection(
                    DB_COLLECTIONS["REFERENCE_PROJECTS"], tenant_id, allow_platform_data=False
                )
            else:
                collection = get_collection(DB_COLLECTIONS["REFERENCE_PROJECTS"])

            existing = await collection.find_one({"project_id": project_data["project_id"]})
            if existing:
                raise ValueError("Reference project already exists")

            reference_project = ReferenceProject(**project_data)
            result = await collection.insert_one(reference_project.dict(by_alias=True))

            logger.info(f"Reference project created: {result.inserted_id}")
            return str(result.inserted_id)

        except Exception as e:
            logger.error(f"Error creating reference project: {e}")
            raise

    async def update_reference_class(
        self,
        name: str,
        updates: Dict[str, Any],
        tenant_id: Optional[str] = None,
    ) -> bool:
        """Update an existing reference class."""
        try:
            if tenant_id:
                collection = get_tenant_collection(
                    DB_COLLECTIONS["REFERENCE_CLASSES"], tenant_id, allow_platform_data=False
                )
            else:
                collection = get_collection(DB_COLLECTIONS["REFERENCE_CLASSES"])

            result = await collection.update_one(
                {"name": name},
                {"$set": {**updates, "updated_at": datetime.utcnow()}},
            )

            return result.modified_count > 0

        except Exception as e:
            logger.error(f"Error updating reference class: {e}")
            raise

    async def update_reference_project(
        self,
        project_id: str,
        updates: Dict[str, Any],
        tenant_id: Optional[str] = None,
    ) -> bool:
        """Update an existing reference project."""
        try:
            if tenant_id:
                collection = get_tenant_collection(
                    DB_COLLECTIONS["REFERENCE_PROJECTS"], tenant_id, allow_platform_data=False
                )
            else:
                collection = get_collection(DB_COLLECTIONS["REFERENCE_PROJECTS"])

            result = await collection.update_one(
                {"project_id": project_id},
                {"$set": {**updates, "updated_at": datetime.utcnow()}},
            )

            return result.modified_count > 0

        except Exception as e:
            logger.error(f"Error updating reference project: {e}")
            raise

    async def deactivate_reference_class(
        self,
        name: str,
        tenant_id: Optional[str] = None,
    ) -> bool:
        """Deactivate a reference class."""
        try:
            if tenant_id:
                collection = get_tenant_collection(
                    DB_COLLECTIONS["REFERENCE_CLASSES"], tenant_id, allow_platform_data=False
                )
            else:
                collection = get_collection(DB_COLLECTIONS["REFERENCE_CLASSES"])

            result = await collection.update_one(
                {"name": name},
                {"$set": {"is_active": False, "updated_at": datetime.utcnow()}},
            )

            return result.modified_count > 0

        except Exception as e:
            logger.error(f"Error deactivating reference class: {e}")
            raise

    async def deactivate_reference_project(
        self,
        project_id: str,
        tenant_id: Optional[str] = None,
    ) -> bool:
        """Deactivate a reference project."""
        try:
            if tenant_id:
                collection = get_tenant_collection(
                    DB_COLLECTIONS["REFERENCE_PROJECTS"], tenant_id, allow_platform_data=False
                )
            else:
                collection = get_collection(DB_COLLECTIONS["REFERENCE_PROJECTS"])

            result = await collection.update_one(
                {"project_id": project_id},
                {"$set": {"is_active": False, "updated_at": datetime.utcnow()}},
            )

            return result.modified_count > 0

        except Exception as e:
            logger.error(f"Error deactivating reference project: {e}")
            raise

    async def get_reference_statistics(
        self,
        tenant_id: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Get statistics about reference data."""
        try:
            classes_col = self._classes_collection(tenant_id)
            projects_col = self._projects_collection(tenant_id)

            active_filter: Dict[str, Any] = {"is_active": True}
            if not tenant_id:
                # Platform admin view: count platform data only
                active_filter["tenant_id"] = None

            class_count = await classes_col.count_documents(active_filter)
            project_count = await projects_col.count_documents(active_filter)

            # Aggregate pipelines — TenantAwareCollection prepends $match with tenant filter
            region_pipeline = [
                {"$match": {"is_active": True}},
                {"$group": {"_id": "$region", "count": {"$sum": 1}}},
            ]
            region_stats = await (
                await projects_col.aggregate(region_pipeline)
            ).to_list(None)

            class_pipeline = [
                {"$match": {"is_active": True}},
                {"$group": {"_id": "$reference_class", "count": {"$sum": 1}}},
            ]
            class_stats = await (
                await projects_col.aggregate(class_pipeline)
            ).to_list(None)

            return {
                "total_classes": class_count,
                "total_projects": project_count,
                "projects_by_region": {r["_id"]: r["count"] for r in region_stats},
                "projects_by_class": {c["_id"]: c["count"] for c in class_stats},
            }

        except Exception as e:
            logger.error(f"Error getting reference statistics: {e}")
            raise 