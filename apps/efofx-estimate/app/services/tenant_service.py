"""
Tenant service for efOfX Estimation Service.

This module provides functionality for managing tenants and their
configuration in the multitenant system.
"""

import logging
from typing import List, Dict, Any, Optional
from datetime import datetime

from app.models.tenant import Tenant, TenantCreate, TenantUpdate
from app.db.mongodb import get_tenants_collection, get_tenant_collection, get_collection
from app.core.constants import DB_COLLECTIONS

logger = logging.getLogger(__name__)

# Tier-based estimation limits (used when tenant settings don't specify a limit)
TIER_LIMITS = {
    "trial": 100,
    "paid": 1000,
}
DEFAULT_MONTHLY_LIMIT = 100


class TenantService:
    """Service for handling tenant management."""

    def __init__(self):
        self.collection = get_tenants_collection()

    async def get_tenant(self, tenant_id: str) -> Optional[dict]:
        """Get tenant document by UUID tenant_id field."""
        try:
            return await self.collection.find_one({"tenant_id": tenant_id})

        except Exception as e:
            logger.error(f"Error getting tenant: {e}")
            raise

    async def get_by_tenant_id(self, tenant_id: str) -> Optional[dict]:
        """Get tenant document by UUID tenant_id field."""
        try:
            return await self.collection.find_one({"tenant_id": tenant_id})
        except Exception as e:
            logger.error(f"Error getting tenant by tenant_id: {e}")
            raise

    async def get_by_email(self, email: str) -> Optional[dict]:
        """Get tenant document by email address."""
        try:
            return await self.collection.find_one({"email": email})
        except Exception as e:
            logger.error(f"Error getting tenant by email: {e}")
            raise

    async def get_tenant_by_api_key(self, api_key: str) -> Optional[Tenant]:
        """Get tenant by API key (legacy plaintext lookup — superseded by hashed key auth)."""
        try:
            tenant_data = await self.collection.find_one({"api_key": api_key})

            if tenant_data:
                return Tenant(**tenant_data)
            return None

        except Exception as e:
            logger.error(f"Error getting tenant by API key: {e}")
            raise

    async def list_tenants(self, limit: int = 10, offset: int = 0) -> List[Tenant]:
        """List tenants with pagination."""
        try:
            cursor = self.collection.find({"is_active": True}).skip(offset).limit(limit)
            tenants_list = await cursor.to_list(length=None)

            return [Tenant(**tenant) for tenant in tenants_list]

        except Exception as e:
            logger.error(f"Error listing tenants: {e}")
            raise

    async def create_tenant(self, tenant_data: TenantCreate) -> str:
        """Create a new tenant."""
        try:
            # Check if API key already exists
            existing = await self.collection.find_one({"api_key": tenant_data.api_key})
            if existing:
                raise ValueError("API key already exists")

            # Create new tenant
            tenant = Tenant(**tenant_data.dict())
            result = await self.collection.insert_one(tenant.dict(by_alias=True))

            logger.info(f"Tenant created: {result.inserted_id}")
            return str(result.inserted_id)

        except Exception as e:
            logger.error(f"Error creating tenant: {e}")
            raise

    async def update_tenant(self, tenant_id: str, updates: TenantUpdate) -> bool:
        """Update an existing tenant."""
        try:
            from datetime import datetime

            # Remove None values
            update_data = {k: v for k, v in updates.dict().items() if v is not None}
            update_data["updated_at"] = datetime.utcnow()

            result = await self.collection.update_one(
                {"tenant_id": tenant_id},
                {"$set": update_data}
            )

            return result.modified_count > 0

        except Exception as e:
            logger.error(f"Error updating tenant: {e}")
            raise

    async def deactivate_tenant(self, tenant_id: str) -> bool:
        """Deactivate a tenant."""
        try:
            from datetime import datetime

            result = await self.collection.update_one(
                {"tenant_id": tenant_id},
                {"$set": {"is_active": False, "updated_at": datetime.utcnow()}}
            )

            return result.modified_count > 0

        except Exception as e:
            logger.error(f"Error deactivating tenant: {e}")
            raise

    async def get_tenant_statistics(self, tenant_id: str) -> Dict[str, Any]:
        """Get statistics for a specific tenant."""
        try:
            from datetime import datetime, timedelta

            # Get estimation count for last 30 days
            thirty_days_ago = datetime.utcnow() - timedelta(days=30)

            estimates_col = get_tenant_collection(DB_COLLECTIONS["ESTIMATES"], tenant_id)
            estimation_count = await estimates_col.count_documents({
                "created_at": {"$gte": thirty_days_ago}
            })

            # Get feedback count
            feedback_col = get_tenant_collection(DB_COLLECTIONS["FEEDBACK"], tenant_id)
            feedback_count = await feedback_col.count_documents({})

            # Get average rating — TenantAwareCollection.aggregate() prepends $match for tenant_id
            pipeline = [
                {"$group": {"_id": None, "avg_rating": {"$avg": "$rating"}}}
            ]

            rating_result = await feedback_col.aggregate(pipeline).to_list(1)
            average_rating = rating_result[0]["avg_rating"] if rating_result else 0.0

            # Determine monthly limit from tenant settings or tier defaults
            tenant_doc = await self.get_by_tenant_id(tenant_id)
            if tenant_doc:
                tier = tenant_doc.get("tier", "trial")
                monthly_limit = tenant_doc.get("settings", {}).get(
                    "max_estimations_per_month",
                    TIER_LIMITS.get(tier, DEFAULT_MONTHLY_LIMIT)
                )
            else:
                monthly_limit = DEFAULT_MONTHLY_LIMIT

            return {
                "estimations_last_30_days": estimation_count,
                "total_feedback": feedback_count,
                "average_rating": average_rating,
                "active_regions": [],  # Would be populated from actual data
                "monthly_limit": monthly_limit,
            }

        except Exception as e:
            logger.error(f"Error getting tenant statistics: {e}")
            raise

    async def validate_tenant_limits(self, tenant_id: str) -> Dict[str, Any]:
        """Validate tenant usage against limits."""
        try:
            from datetime import datetime

            # Get tenant by UUID string (not legacy ObjectId)
            tenant_doc = await self.get_by_tenant_id(tenant_id)
            if not tenant_doc:
                raise ValueError("Tenant not found")

            # Determine monthly limit from tier defaults or settings override
            tier = tenant_doc.get("tier", "trial")
            monthly_limit = tenant_doc.get("settings", {}).get(
                "max_estimations_per_month",
                TIER_LIMITS.get(tier, DEFAULT_MONTHLY_LIMIT)
            )

            # Get current month usage via TenantAwareCollection
            start_of_month = datetime.utcnow().replace(
                day=1, hour=0, minute=0, second=0, microsecond=0
            )

            estimates_col = get_tenant_collection(DB_COLLECTIONS["ESTIMATES"], tenant_id)
            monthly_usage = await estimates_col.count_documents({
                "created_at": {"$gte": start_of_month}
            })

            limit_exceeded = monthly_usage >= monthly_limit
            remaining_estimations = max(0, monthly_limit - monthly_usage)

            return {
                "monthly_usage": monthly_usage,
                "monthly_limit": monthly_limit,
                "limit_exceeded": limit_exceeded,
                "remaining_estimations": remaining_estimations,
                "is_active": tenant_doc.get("is_active", True),
            }

        except Exception as e:
            logger.error(f"Error validating tenant limits: {e}")
            raise

    async def get_all_tenant_statistics(self) -> Dict[str, Any]:
        """Get statistics for all tenants (cross-tenant admin aggregation)."""
        try:
            # Count total tenants
            total_tenants = await self.collection.count_documents({})
            active_tenants = await self.collection.count_documents({"is_active": True})

            # Cross-tenant raw access — intentionally unscoped (platform-level admin stats)
            estimates_col = get_collection(DB_COLLECTIONS["ESTIMATES"])
            total_estimations = await estimates_col.count_documents({})

            feedback_col = get_collection(DB_COLLECTIONS["FEEDBACK"])
            total_feedback = await feedback_col.count_documents({})

            return {
                "total_tenants": total_tenants,
                "active_tenants": active_tenants,
                "total_estimations": total_estimations,
                "total_feedback": total_feedback,
                "average_estimations_per_tenant": (
                    total_estimations / active_tenants if active_tenants > 0 else 0
                ),
            }

        except Exception as e:
            logger.error(f"Error getting all tenant statistics: {e}")
            raise
