"""
Tenant service for efOfX Estimation Service.

This module provides functionality for managing tenants and their
configuration in the multitenant system.
"""

import logging
from typing import List, Dict, Any, Optional

from app.models.tenant import Tenant, TenantCreate, TenantUpdate
from app.db.mongodb import get_tenants_collection

logger = logging.getLogger(__name__)


class TenantService:
    """Service for handling tenant management."""

    def __init__(self):
        self.collection = get_tenants_collection()

    async def get_tenant(self, tenant_id: str) -> Optional[Tenant]:
        """Get tenant by ObjectId (legacy). Prefer get_by_tenant_id for UUID lookups."""
        try:
            from bson import ObjectId
            tenant_data = await self.collection.find_one({"_id": ObjectId(tenant_id)})

            if tenant_data:
                return Tenant(**tenant_data)
            return None

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
            from bson import ObjectId
            from datetime import datetime
            
            # Remove None values
            update_data = {k: v for k, v in updates.dict().items() if v is not None}
            update_data["updated_at"] = datetime.utcnow()
            
            result = await self.collection.update_one(
                {"_id": ObjectId(tenant_id)},
                {"$set": update_data}
            )
            
            return result.modified_count > 0
            
        except Exception as e:
            logger.error(f"Error updating tenant: {e}")
            raise
    
    async def deactivate_tenant(self, tenant_id: str) -> bool:
        """Deactivate a tenant."""
        try:
            from bson import ObjectId
            from datetime import datetime
            
            result = await self.collection.update_one(
                {"_id": ObjectId(tenant_id)},
                {"$set": {"is_active": False, "updated_at": datetime.utcnow()}}
            )
            
            return result.modified_count > 0
            
        except Exception as e:
            logger.error(f"Error deactivating tenant: {e}")
            raise
    
    async def get_tenant_statistics(self, tenant_id: str) -> Dict[str, Any]:
        """Get statistics for a specific tenant."""
        try:
            from bson import ObjectId
            from datetime import datetime, timedelta
            
            # Get estimation count for last 30 days
            thirty_days_ago = datetime.utcnow() - timedelta(days=30)
            
            estimates_collection = get_estimates_collection()
            estimation_count = await estimates_collection.count_documents({
                "tenant_id": ObjectId(tenant_id),
                "created_at": {"$gte": thirty_days_ago}
            })
            
            # Get feedback count
            feedback_collection = get_feedback_collection()
            feedback_count = await feedback_collection.count_documents({
                "tenant_id": ObjectId(tenant_id)
            })
            
            # Get average rating
            pipeline = [
                {"$match": {"tenant_id": ObjectId(tenant_id)}},
                {"$group": {"_id": None, "avg_rating": {"$avg": "$rating"}}}
            ]
            
            rating_result = await feedback_collection.aggregate(pipeline).to_list(1)
            average_rating = rating_result[0]["avg_rating"] if rating_result else 0.0
            
            return {
                "estimations_last_30_days": estimation_count,
                "total_feedback": feedback_count,
                "average_rating": average_rating,
                "active_regions": [],  # Would be populated from actual data
                "monthly_limit": 1000  # Would come from tenant settings
            }
            
        except Exception as e:
            logger.error(f"Error getting tenant statistics: {e}")
            raise
    
    async def validate_tenant_limits(self, tenant_id: str) -> Dict[str, Any]:
        """Validate tenant usage against limits."""
        try:
            from bson import ObjectId
            from datetime import datetime, timedelta
            
            # Get tenant
            tenant = await self.get_tenant(tenant_id)
            if not tenant:
                raise ValueError("Tenant not found")
            
            # Get current month usage
            start_of_month = datetime.utcnow().replace(day=1, hour=0, minute=0, second=0, microsecond=0)
            
            estimates_collection = get_estimates_collection()
            monthly_usage = await estimates_collection.count_documents({
                "tenant_id": ObjectId(tenant_id),
                "created_at": {"$gte": start_of_month}
            })
            
            # Check limits
            limit_exceeded = monthly_usage >= tenant.max_estimations_per_month
            remaining_estimations = max(0, tenant.max_estimations_per_month - monthly_usage)
            
            return {
                "monthly_usage": monthly_usage,
                "monthly_limit": tenant.max_estimations_per_month,
                "limit_exceeded": limit_exceeded,
                "remaining_estimations": remaining_estimations,
                "is_active": tenant.is_active
            }
            
        except Exception as e:
            logger.error(f"Error validating tenant limits: {e}")
            raise
    
    async def get_all_tenant_statistics(self) -> Dict[str, Any]:
        """Get statistics for all tenants."""
        try:
            # Count total tenants
            total_tenants = await self.collection.count_documents({})
            active_tenants = await self.collection.count_documents({"is_active": True})
            
            # Get total estimations across all tenants
            estimates_collection = get_estimates_collection()
            total_estimations = await estimates_collection.count_documents({})
            
            # Get total feedback
            feedback_collection = get_feedback_collection()
            total_feedback = await feedback_collection.count_documents({})
            
            return {
                "total_tenants": total_tenants,
                "active_tenants": active_tenants,
                "total_estimations": total_estimations,
                "total_feedback": total_feedback,
                "average_estimations_per_tenant": total_estimations / active_tenants if active_tenants > 0 else 0
            }
            
        except Exception as e:
            logger.error(f"Error getting all tenant statistics: {e}")
            raise 