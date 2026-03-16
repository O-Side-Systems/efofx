"""
MongoDB connection and database management for efOfX Estimation Service.

This module provides MongoDB connection management, database access,
and collection utilities for the estimation service.

Tenant data access pattern:
    Use get_tenant_collection() for all tenant-scoped data. It returns a
    TenantAwareCollection that auto-injects tenant_id on every operation.

    col = get_tenant_collection("estimates", tenant_id)
    doc = await col.find_one({"session_id": sid})  # tenant_id auto-injected
"""

from motor.motor_asyncio import AsyncIOMotorClient, AsyncIOMotorDatabase
from typing import Optional
import logging
from contextlib import asynccontextmanager

from app.core.config import settings
from app.core.constants import DB_COLLECTIONS
from app.db.tenant_collection import TenantAwareCollection

logger = logging.getLogger(__name__)

# Global database client
_client: Optional[AsyncIOMotorClient] = None
_database: Optional[AsyncIOMotorDatabase] = None


async def connect_to_mongo():
    """Create database connection."""
    global _client, _database

    try:
        _client = AsyncIOMotorClient(settings.MONGO_URI)
        _database = _client[settings.MONGO_DB_NAME]

        # Test the connection
        await _client.admin.command("ping")
        logger.info("Successfully connected to MongoDB")

    except Exception as e:
        logger.error(f"Failed to connect to MongoDB: {e}")
        raise


async def close_mongo_connection():
    """Close database connection."""
    global _client

    if _client:
        _client.close()
        logger.info("MongoDB connection closed")


def get_database() -> AsyncIOMotorDatabase:
    """Get database instance."""
    if _database is None:
        raise RuntimeError("Database not initialized. Call connect_to_mongo() first.")
    return _database


def get_collection(collection_name: str):
    """Get collection instance."""
    db = get_database()
    return db[collection_name]


def get_tenant_collection(
    collection_name: str,
    tenant_id: str,
    allow_platform_data: bool = False,
) -> TenantAwareCollection:
    """
    Get a tenant-scoped collection. All tenant data access MUST use this.

    Returns a TenantAwareCollection that automatically injects tenant_id into
    every MongoDB operation, making cross-tenant data leakage structurally
    impossible.

    Args:
        collection_name: MongoDB collection name (use DB_COLLECTIONS constants).
        tenant_id: The tenant's ID — required and must be non-empty.
        allow_platform_data: If True, queries include platform data (tenant_id=None)
            alongside the tenant's own data. Use for reference class lookups.

    Returns:
        TenantAwareCollection scoped to the given tenant_id.

    Raises:
        ValueError: If tenant_id is empty or None.

    Example:
        col = get_tenant_collection("estimates", tenant_id)
        doc = await col.find_one({"session_id": session_id})
    """
    db = get_database()
    raw_col = db[collection_name]
    return TenantAwareCollection(raw_col, tenant_id, allow_platform_data)


@asynccontextmanager
async def get_db_session():
    """Database session context manager."""
    if not _database:
        await connect_to_mongo()

    try:
        yield _database
    except Exception as e:
        logger.error(f"Database session error: {e}")
        raise


# Collection access functions
def get_tenants_collection():
    """
    Get tenants collection (raw, not tenant-scoped).

    Tenants are top-level entities — they are NOT scoped by tenant_id.
    This is still the correct accessor for tenant management operations
    (registration, login, lookup by email/api_key).
    """
    return get_collection(DB_COLLECTIONS["TENANTS"])


# Database utilities
async def create_indexes():
    """
    Create database indexes for optimal performance and tenant isolation (ISOL-03).

    All tenant-scoped collections use compound indexes with tenant_id as the
    FIRST field, ensuring the MongoDB query planner uses the index for any
    tenant-scoped query (prefix rule). This prevents full-collection scans
    when filtering by tenant_id.

    Index design principles:
    - tenant_id is always the leftmost key for tenant-scoped collections
    - Compound indexes cover the most common query patterns
    - Unique constraints enforce data integrity within tenant scope
    """
    try:
        db = get_database()

        # ------------------------------------------------------------------
        # Tenants — NOT tenant-scoped (top-level entity, identified by email)
        # ------------------------------------------------------------------
        await db["tenants"].create_index("email", unique=True)
        logger.info("Index confirmed: tenants.email_1")
        await db["tenants"].create_index("tenant_id", unique=True)
        logger.info("Index confirmed: tenants.tenant_id_1")

        # ------------------------------------------------------------------
        # Estimates — compound tenant_id FIRST (ISOL-03)
        # ------------------------------------------------------------------
        # Primary sort: most recent sessions per tenant
        await db["estimates"].create_index([("tenant_id", 1), ("created_at", -1)])
        logger.info("Index confirmed: estimates.tenant_id_1_created_at_-1")
        # Unique per (tenant, session) — also covers session_id lookup within tenant
        await db["estimates"].create_index(
            [("tenant_id", 1), ("session_id", 1)],
            unique=True,
        )
        logger.info("Index confirmed: estimates.tenant_id_1_session_id_1")

        # ------------------------------------------------------------------
        # Reference classes — tenant_id first; platform data (None) queryable
        # ------------------------------------------------------------------
        await db["reference_classes"].create_index([("tenant_id", 1), ("category", 1)])
        logger.info("Index confirmed: reference_classes.tenant_id_1_category_1")
        await db["reference_classes"].create_index([("tenant_id", 1), ("name", 1)])
        logger.info("Index confirmed: reference_classes.tenant_id_1_name_1")

        # ------------------------------------------------------------------
        # Reference projects — tenant_id first
        # ------------------------------------------------------------------
        await db["reference_projects"].create_index(
            [("tenant_id", 1), ("reference_class", 1)]
        )
        logger.info("Index confirmed: reference_projects.tenant_id_1_reference_class_1")
        await db["reference_projects"].create_index([("tenant_id", 1), ("region", 1)])
        logger.info("Index confirmed: reference_projects.tenant_id_1_region_1")

        # ------------------------------------------------------------------
        # Feedback — tenant_id first
        # ------------------------------------------------------------------
        await db["feedback"].create_index(
            [("tenant_id", 1), ("estimation_session_id", 1)]
        )
        logger.info("Index confirmed: feedback.tenant_id_1_estimation_session_id_1")
        await db["feedback"].create_index([("tenant_id", 1), ("created_at", -1)])
        logger.info("Index confirmed: feedback.tenant_id_1_created_at_-1")

        # ------------------------------------------------------------------
        # Chat sessions — tenant_id first; unique per (tenant, session)
        # ------------------------------------------------------------------
        await db["chat_sessions"].create_index(
            [("tenant_id", 1), ("session_id", 1)],
            unique=True,
        )
        logger.info("Index confirmed: chat_sessions.tenant_id_1_session_id_1")
        # Chat sessions — TTL auto-expiry (expires_at is set to utcnow() + 24h on creation)  # noqa: E501
        await db["chat_sessions"].create_index("expires_at", expireAfterSeconds=0)
        logger.info("Index confirmed: chat_sessions.expires_at_1")

        # ------------------------------------------------------------------
        # Auth collections — TTL indexes (added by 02-01 and 02-02)
        # ------------------------------------------------------------------
        await db["verification_tokens"].create_index("expires_at", expireAfterSeconds=0)
        logger.info("Index confirmed: verification_tokens.expires_at_1")
        await db["refresh_tokens"].create_index("expires_at", expireAfterSeconds=0)
        logger.info("Index confirmed: refresh_tokens.expires_at_1")
        await db["refresh_tokens"].create_index("token_hash", unique=True)
        logger.info("Index confirmed: refresh_tokens.token_hash_1")

        # ------------------------------------------------------------------
        # Widget analytics — tenant_id + date unique (daily bucketing upsert)
        # ------------------------------------------------------------------
        await db["widget_analytics"].create_index(
            [("tenant_id", 1), ("date", 1)],
            unique=True,
        )
        logger.info("Index confirmed: widget_analytics.tenant_id_1_date_1")

        # ------------------------------------------------------------------
        # Widget leads — tenant_id + session lookup
        # ------------------------------------------------------------------
        await db["widget_leads"].create_index([("tenant_id", 1), ("session_id", 1)])
        logger.info("Index confirmed: widget_leads.tenant_id_1_session_id_1")

        # ------------------------------------------------------------------
        # Widget leads — tenant_id + time sort (most recent leads per tenant)
        # ------------------------------------------------------------------
        await db["widget_leads"].create_index([("tenant_id", 1), ("captured_at", -1)])
        logger.info("Index confirmed: widget_leads.tenant_id_1_captured_at_-1")

        # ------------------------------------------------------------------
        # Feedback tokens — TTL auto-expiry + unique hash lookup (Phase 7)
        # ------------------------------------------------------------------
        await db["feedback_tokens"].create_index("expires_at", expireAfterSeconds=0)
        logger.info("Index confirmed: feedback_tokens.expires_at_1")
        await db["feedback_tokens"].create_index("token_hash", unique=True)
        logger.info("Index confirmed: feedback_tokens.token_hash_1")
        await db["feedback_tokens"].create_index(
            [("tenant_id", 1), ("estimation_session_id", 1)]
        )
        logger.info(
            "Index confirmed: feedback_tokens.tenant_id_1_estimation_session_id_1"
        )

        logger.info("Database indexes created successfully")

    except Exception as e:
        logger.error(f"Failed to create database indexes: {e}")
        raise


async def migrate_synthetic_reference_classes():
    """
    CALB-01: Tag synthetic reference class documents with data_source="synthetic".

    Synthetic documents have is_synthetic=True and tenant_id=None (they are
    platform-wide data, not scoped to any tenant). This migration adds the
    data_source field so the calibration pipeline can explicitly exclude them
    from accuracy calculations.

    Filter: {"is_synthetic": True, "data_source": {"$exists": False}}
    Update: {"$set": {"data_source": "synthetic"}}

    Idempotent: the $exists: False guard ensures re-runs are no-ops (already-
    tagged documents won't match the filter again).

    Uses get_database() directly (NOT get_tenant_collection) because synthetic
    documents have tenant_id=None and are not scoped to any tenant.
    """
    db = get_database()
    result = await db[DB_COLLECTIONS["REFERENCE_CLASSES"]].update_many(
        {"is_synthetic": True, "data_source": {"$exists": False}},
        {"$set": {"data_source": "synthetic"}},
    )
    logger.info(
        "CALB-01 migration: tagged %d synthetic reference class documents with data_source='synthetic'",  # noqa: E501
        result.modified_count,
    )


async def migrate_estimation_session_tenant_id():
    """
    DEBT-01: Safety migration — confirm no EstimationSession documents have
    BSON ObjectId-type tenant_id values. TenantAwareCollection.insert_one()
    always overwrites tenant_id to the correct string before saving, so in
    practice no ObjectId-typed values should exist. This confirms that and
    marks any orphaned documents if found.

    Idempotent: safe to run on every deploy.
    """
    db = get_database()
    result = await db["estimates"].count_documents({"tenant_id": {"$type": "objectId"}})
    if result > 0:
        logger.warning(
            "DEBT-01 migration: found %d estimates with ObjectId tenant_id — "
            "marking as orphaned (cannot attribute to a real tenant).",
            result,
        )
        await db["estimates"].update_many(
            {"tenant_id": {"$type": "objectId"}},
            {
                "$set": {
                    "tenant_id": "__orphaned__",
                    "migration_note": "DEBT-01: was random PyObjectId",
                }
            },
        )
    else:
        logger.info(
            "DEBT-01 migration: no estimates with ObjectId tenant_id found — nothing to migrate"  # noqa: E501
        )


async def health_check():
    """Check database health."""
    try:
        if _database is None:
            return False
        await _database.command("ping")
        return True
    except Exception as e:
        logger.error(f"Database health check failed: {e}")
        return False


async def get_database_stats():
    """Get database statistics."""
    try:
        if _database is None:
            return None
        stats = await _database.command("dbStats")
        return {
            "collections": stats.get("collections", 0),
            "data_size": stats.get("dataSize", 0),
            "storage_size": stats.get("storageSize", 0),
            "indexes": stats.get("indexes", 0),
            "index_size": stats.get("indexSize", 0),
        }
    except Exception as e:
        logger.error(f"Failed to get database stats: {e}")
        return None
