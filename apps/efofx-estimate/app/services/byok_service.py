"""
BYOK (Bring Your Own Key) service for managing tenant OpenAI API keys.

Contractors supply their own OpenAI API keys. This service handles:
  - Validation: key is tested against openai.models.list() before storage
  - Encryption: key is encrypted with a per-tenant Fernet key (HKDF-derived)
  - Storage: ciphertext stored in the tenant document alongside the last 6 chars
  - Rotation: new key overwrites old immediately (no version history per locked decision)  # noqa: E501
  - Decryption: plaintext returned on demand within request scope only

Locked decisions:
  - No platform key fallback — LLM endpoints return 402 when no key stored
  - Simple overwrite for rotation (no version history)
  - Validation uses models.list() — lightweight, no tokens burned
"""

import logging
from datetime import datetime, timezone

from fastapi import HTTPException
from openai import AsyncOpenAI, AuthenticationError

from app.core.config import settings
from app.core.constants import DB_COLLECTIONS
from app.db.mongodb import get_database
from app.utils.crypto import decrypt_openai_key, encrypt_openai_key, mask_openai_key

logger = logging.getLogger(__name__)


async def _validate_openai_key(plaintext_key: str) -> None:
    """Call OpenAI's models.list() to verify the key is valid.

    Raises:
        HTTPException 400: AuthenticationError — key is invalid or expired.
        HTTPException 503: Any other exception — OpenAI service unreachable.
    """
    try:
        client = AsyncOpenAI(api_key=plaintext_key)
        await client.models.list()
    except AuthenticationError:
        raise HTTPException(status_code=400, detail="Invalid OpenAI API key")
    except Exception as exc:
        logger.warning("OpenAI key validation failed with non-auth error: %s", exc)
        raise HTTPException(
            status_code=503,
            detail="Could not validate OpenAI key. Try again later.",
        )


async def validate_and_store_openai_key(tenant_id: str, plaintext_key: str) -> str:
    """Validate, encrypt, and store a tenant's OpenAI API key.

    Flow:
    1. Validate key against OpenAI's models.list() endpoint.
    2. Encrypt with the per-tenant HKDF-derived Fernet key.
    3. Store ciphertext and last 6 chars in the tenant document.
    4. Return the masked key for display.

    Args:
        tenant_id:     The tenant's UUID string.
        plaintext_key: The raw OpenAI API key (never persisted).

    Returns:
        Masked key string, e.g. "sk-...abc123".

    Raises:
        HTTPException 400: Invalid OpenAI API key.
        HTTPException 503: OpenAI service unavailable during validation.
    """
    await _validate_openai_key(plaintext_key)

    master_key = settings.MASTER_ENCRYPTION_KEY.encode()
    encrypted = encrypt_openai_key(master_key, tenant_id, plaintext_key)
    last6 = plaintext_key[-6:] if len(plaintext_key) >= 6 else ""

    db = get_database()
    now = datetime.now(timezone.utc)
    await db[DB_COLLECTIONS["TENANTS"]].update_one(
        {"tenant_id": tenant_id},
        {
            "$set": {
                "encrypted_openai_key": encrypted,
                "openai_key_last6": last6,
                "updated_at": now,
            }
        },
    )

    logger.info("OpenAI key stored for tenant_id=%s", tenant_id)
    return mask_openai_key(plaintext_key)


async def decrypt_tenant_openai_key(tenant_id: str) -> str:
    """Decrypt and return a tenant's OpenAI API key for use in request scope.

    IMPORTANT: Never persist the returned plaintext key — use it within the
    request scope only and discard it after use.

    Args:
        tenant_id: The tenant's UUID string.

    Returns:
        The plaintext OpenAI API key.

    Raises:
        HTTPException 402: Tenant has no stored OpenAI key.
    """
    db = get_database()
    tenant_doc = await db[DB_COLLECTIONS["TENANTS"]].find_one({"tenant_id": tenant_id})

    if not tenant_doc or not tenant_doc.get("encrypted_openai_key"):
        raise HTTPException(
            status_code=402,
            detail="OpenAI API key required. Add your key in Settings.",
        )

    master_key = settings.MASTER_ENCRYPTION_KEY.encode()
    return decrypt_openai_key(master_key, tenant_id, tenant_doc["encrypted_openai_key"])


def get_openai_key_status(tenant_doc: dict) -> dict:
    """Return BYOK key status for a tenant document without decrypting.

    Uses the stored openai_key_last6 field to build the masked key,
    avoiding any decryption operation.

    Args:
        tenant_doc: Raw tenant document dict from MongoDB.

    Returns:
        Dict with keys:
          - has_key (bool): Whether an OpenAI key is stored.
          - masked_key (str | None): "sk-...{last6}" if stored, else None.
    """
    has_key = bool(tenant_doc.get("encrypted_openai_key"))
    if has_key:
        last6 = tenant_doc.get("openai_key_last6", "")
        masked_key = f"sk-...{last6}" if last6 else "sk-...******"
    else:
        masked_key = None

    return {"has_key": has_key, "masked_key": masked_key}
