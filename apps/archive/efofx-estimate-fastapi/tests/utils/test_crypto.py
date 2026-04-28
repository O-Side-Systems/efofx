"""
Unit tests for BYOK crypto utilities.

Tests are pure unit tests — no DB or network calls required.
Covers HKDF key derivation, Fernet encrypt/decrypt round-trip,
per-tenant isolation, and key masking.
"""

import os

import pytest
from cryptography.fernet import InvalidToken

from app.utils.crypto import (
    decrypt_openai_key,
    derive_tenant_fernet_key,
    encrypt_openai_key,
    mask_openai_key,
)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture
def master_key_bytes() -> bytes:
    """32 bytes of random master key material for tests."""
    return os.urandom(32)


# ---------------------------------------------------------------------------
# HKDF key derivation tests
# ---------------------------------------------------------------------------


def test_derive_tenant_fernet_key_deterministic(master_key_bytes):
    """Same master_key + tenant_id must produce the same Fernet key each time."""
    fernet_a = derive_tenant_fernet_key(master_key_bytes, "tenant-123")
    fernet_b = derive_tenant_fernet_key(master_key_bytes, "tenant-123")

    # Verify determinism by encrypting with one and decrypting with the other
    # (only works if they share the same key material)
    plaintext = "sk-test-key-determinism"
    ciphertext = fernet_a.encrypt(plaintext.encode())
    result = fernet_b.decrypt(ciphertext).decode()
    assert result == plaintext


def test_derive_different_tenants_different_keys(master_key_bytes):
    """Different tenant_ids must produce different derived Fernet keys."""
    fernet_a = derive_tenant_fernet_key(master_key_bytes, "tenant-123")
    fernet_b = derive_tenant_fernet_key(master_key_bytes, "tenant-456")

    # Encrypt plaintext with tenant A's key, attempt to decrypt with tenant B's key
    plaintext = "sk-test-isolation-check"
    ciphertext = fernet_a.encrypt(plaintext.encode())

    with pytest.raises(InvalidToken):
        fernet_b.decrypt(ciphertext)


# ---------------------------------------------------------------------------
# Encrypt / decrypt round-trip tests
# ---------------------------------------------------------------------------


def test_encrypt_decrypt_roundtrip(master_key_bytes):
    """encrypt_openai_key followed by decrypt_openai_key returns the original plaintext."""
    tenant_id = "tenant-roundtrip"
    plaintext = "sk-proj-abcdef1234567890"

    ciphertext = encrypt_openai_key(master_key_bytes, tenant_id, plaintext)
    recovered = decrypt_openai_key(master_key_bytes, tenant_id, ciphertext)

    assert recovered == plaintext


def test_per_tenant_encryption_isolation(master_key_bytes):
    """Ciphertext from tenant A cannot be decrypted using tenant B's derived key."""
    plaintext = "sk-proj-supersecretkey"

    ciphertext_a = encrypt_openai_key(master_key_bytes, "tenant-A", plaintext)

    with pytest.raises(InvalidToken):
        decrypt_openai_key(master_key_bytes, "tenant-B", ciphertext_a)


def test_encrypt_produces_string(master_key_bytes):
    """encrypt_openai_key must return a string, not bytes."""
    result = encrypt_openai_key(master_key_bytes, "tenant-stringtest", "sk-test-key")
    assert isinstance(result, str)


def test_encrypt_different_ciphertexts_same_input(master_key_bytes):
    """Fernet uses timestamps, so two encryptions of the same plaintext differ."""
    tenant_id = "tenant-nonce"
    plaintext = "sk-same-plaintext"

    cipher1 = encrypt_openai_key(master_key_bytes, tenant_id, plaintext)
    cipher2 = encrypt_openai_key(master_key_bytes, tenant_id, plaintext)

    # Ciphertexts differ due to Fernet's built-in nonce/timestamp
    assert cipher1 != cipher2


# ---------------------------------------------------------------------------
# mask_openai_key tests
# ---------------------------------------------------------------------------


def test_mask_openai_key_normal():
    """Long key shows last 6 characters after 'sk-...' prefix."""
    result = mask_openai_key("sk-proj-abcdef123456")
    assert result == "sk-...123456"


def test_mask_openai_key_short():
    """Key shorter than 6 chars returns the fallback mask."""
    result = mask_openai_key("abc")
    assert result == "sk-...******"


def test_mask_openai_key_exact_six():
    """A key with exactly 6 characters shows all 6 after 'sk-...'."""
    result = mask_openai_key("123456")
    assert result == "sk-...123456"
