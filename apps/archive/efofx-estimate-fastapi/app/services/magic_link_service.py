"""
Magic link token service for feedback email flow.

Generates cryptographically secure tokens, stores SHA-256 hashes in MongoDB,
and manages the token lifecycle: create -> open (GET) -> consume (POST).

Token pattern mirrors auth_service.py verification tokens:
- secrets.token_urlsafe(32) for raw token
- hashlib.sha256 for storage hash
- MongoDB TTL index on expires_at for auto-cleanup
"""

import hashlib
import logging
import secrets
from datetime import datetime, timedelta, timezone
from typing import Optional, Tuple

from app.db.mongodb import get_database
from app.core.constants import DB_COLLECTIONS

logger = logging.getLogger(__name__)

MAGIC_LINK_TTL_HOURS = 72


class MagicLinkService:
    """Manages feedback magic link token lifecycle."""

    COLLECTION = DB_COLLECTIONS["FEEDBACK_TOKENS"]

    @staticmethod
    def generate_token() -> Tuple[str, str, datetime]:
        """Generate (raw_token, token_hash, expires_at).

        raw_token: emailed to customer in magic link URL.
        token_hash: stored in MongoDB (never store raw).
        expires_at: 72h from now, used by MongoDB TTL index.
        """
        raw = secrets.token_urlsafe(32)
        token_hash = hashlib.sha256(raw.encode()).hexdigest()
        expires_at = datetime.now(timezone.utc) + timedelta(hours=MAGIC_LINK_TTL_HOURS)
        return raw, token_hash, expires_at

    @staticmethod
    def hash_token(raw_token: str) -> str:
        """Hash a raw token for DB lookup."""
        return hashlib.sha256(raw_token.encode()).hexdigest()

    async def create_magic_link(
        self,
        tenant_id: str,
        estimation_session_id: str,
        customer_email: str,
        project_name: str = "Your Project",
    ) -> Tuple[str, str]:
        """Create a magic link token and store hash in DB.

        Returns (raw_token, token_hash).
        Raw token is returned so it can be embedded in the email URL.
        Token hash is stored in MongoDB — raw token is never persisted.
        """
        raw, token_hash, expires_at = self.generate_token()

        doc = {
            "token_hash": token_hash,
            "tenant_id": tenant_id,
            "estimation_session_id": estimation_session_id,
            "customer_email": customer_email,
            "project_name": project_name,
            "expires_at": expires_at,
            "opened_at": None,
            "used_at": None,
            "created_at": datetime.now(timezone.utc),
        }

        db = get_database()
        await db[self.COLLECTION].insert_one(doc)
        logger.info(
            "Magic link created for session %s (tenant %s)",
            estimation_session_id,
            tenant_id,
        )
        return raw, token_hash

    async def resolve_token_state(self, raw_token: str) -> Tuple[str, Optional[dict]]:
        """Resolve token to one of: 'valid', 'expired', 'used', 'not_found'.

        Returns (state, token_doc).
        """
        token_hash = self.hash_token(raw_token)
        db = get_database()
        token_doc = await db[self.COLLECTION].find_one({"token_hash": token_hash})

        if token_doc is None:
            return "not_found", None

        now = datetime.now(timezone.utc)
        expires_at = token_doc["expires_at"]
        # Handle timezone-naive datetimes from MongoDB
        if expires_at.tzinfo is None:
            expires_at = expires_at.replace(tzinfo=timezone.utc)

        if now > expires_at:
            return "expired", token_doc

        if token_doc.get("used_at") is not None:
            return "used", token_doc

        return "valid", token_doc

    async def mark_opened(self, raw_token: str) -> None:
        """Set opened_at on first GET — idempotent (does not overwrite)."""
        token_hash = self.hash_token(raw_token)
        db = get_database()
        await db[self.COLLECTION].update_one(
            {"token_hash": token_hash, "opened_at": None},
            {"$set": {"opened_at": datetime.now(timezone.utc)}},
        )

    async def consume(self, raw_token: str) -> bool:
        """Mark token as used (POST consumes). Returns False if already used."""
        token_hash = self.hash_token(raw_token)
        db = get_database()
        result = await db[self.COLLECTION].update_one(
            {"token_hash": token_hash, "used_at": None},
            {"$set": {"used_at": datetime.now(timezone.utc)}},
        )
        return result.modified_count > 0
