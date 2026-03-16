"""
BYOK Fernet encryption utilities using per-tenant HKDF key derivation.

Provides deterministic per-tenant Fernet key derivation via HKDF-SHA256,
plus encrypt/decrypt helpers and a key masking utility for safe display.

Design decisions:
- HKDF info string scoped to "efofx-byok-{tenant_id}" — limits blast radius
  if master key is ever compromised (each tenant has an independent derived key)
- Fernet provides authenticated encryption with timestamp — safe for at-rest storage
- decrypt_openai_key should only be called in request scope — never persist result
"""

import base64

from cryptography.fernet import Fernet
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.kdf.hkdf import HKDF


def derive_tenant_fernet_key(master_key: bytes, tenant_id: str) -> Fernet:
    """Derive a unique Fernet instance for a tenant using HKDF-SHA256.

    Args:
        master_key: High-entropy master key bytes (32+ bytes recommended).
        tenant_id:  Tenant UUID string — used as HKDF info to create isolation.

    Returns:
        A Fernet instance keyed with the derived key. Deterministic for the
        same (master_key, tenant_id) pair.
    """
    hkdf = HKDF(
        algorithm=hashes.SHA256(),
        length=32,
        salt=None,  # master_key is high-entropy — salt omitted per design
        info=f"efofx-byok-{tenant_id}".encode(),
    )
    derived_key = hkdf.derive(master_key)
    fernet_key = base64.urlsafe_b64encode(derived_key)
    return Fernet(fernet_key)


def encrypt_openai_key(master_key: bytes, tenant_id: str, plaintext_key: str) -> str:
    """Encrypt a tenant's OpenAI API key using their derived Fernet key.

    Args:
        master_key:    Master encryption key bytes.
        tenant_id:     Tenant identifier used for per-tenant key derivation.
        plaintext_key: The raw OpenAI API key to encrypt.

    Returns:
        Fernet ciphertext as a string, suitable for MongoDB storage.
        Note: Different each call due to Fernet's built-in timestamp/nonce.
    """
    fernet = derive_tenant_fernet_key(master_key, tenant_id)
    encrypted = fernet.encrypt(plaintext_key.encode())
    return encrypted.decode()


def decrypt_openai_key(master_key: bytes, tenant_id: str, encrypted_key: str) -> str:
    """Decrypt a tenant's stored OpenAI API key.

    Call this only within request scope — never persist the returned plaintext.

    Args:
        master_key:    Master encryption key bytes.
        tenant_id:     Tenant identifier used to re-derive the correct Fernet key.
        encrypted_key: Fernet ciphertext string from MongoDB.

    Returns:
        Decrypted OpenAI API key as a string.

    Raises:
        cryptography.fernet.InvalidToken: If the ciphertext was not produced by
            this tenant's derived key (e.g., cross-tenant decryption attempt).
    """
    fernet = derive_tenant_fernet_key(master_key, tenant_id)
    return fernet.decrypt(encrypted_key.encode()).decode()


def mask_openai_key(plaintext_key: str) -> str:
    """Return a masked representation of an OpenAI key showing the last 6 chars.

    Args:
        plaintext_key: The full OpenAI API key string.

    Returns:
        "sk-...{last6}" if key is 6+ chars long, else "sk-...******".
    """
    if len(plaintext_key) < 6:
        return "sk-...******"
    return f"sk-...{plaintext_key[-6:]}"
