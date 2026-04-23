//! BYOK crypto primitives.
//!
//! ## Design
//!
//! Each tenant gets its own Fernet key derived from the master key via
//! HKDF-SHA256 with `info = "efofx-byok-{tenant_id}"`. Consequences:
//!
//! - No two tenants share a Fernet key, so a leaked ciphertext is useless
//!   without the master key AND the tenant id.
//! - The master key never directly encrypts user data.
//! - Derivation is deterministic — the same `(master, tenant_id)` pair
//!   always yields the same Fernet key, so `decrypt` on any pod works.
//!
//! This is a byte-compatible port of `packages/efofx-shared/efofx_shared/
//! utils/crypto.py`. Ciphertexts produced on one side decrypt cleanly on
//! the other — important during cutover.

use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;
use efofx_domain::TenantId;
use hkdf::Hkdf;
use sha2::Sha256;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("hkdf expansion failed: {0}")]
    Hkdf(&'static str),
    #[error("fernet operation failed")]
    Fernet,
    #[error("invalid fernet ciphertext")]
    InvalidCiphertext,
}

/// Derive the per-tenant Fernet instance.
fn derive_fernet(master_key: &[u8], tenant_id: TenantId) -> Result<fernet::Fernet, CryptoError> {
    let hk = Hkdf::<Sha256>::new(None, master_key);
    let mut okm = [0u8; 32];
    let info = format!("efofx-byok-{tenant_id}");
    hk.expand(info.as_bytes(), &mut okm)
        .map_err(|_| CryptoError::Hkdf("okm length"))?;

    // Python's `base64.urlsafe_b64encode` of 32 bytes produces 44 chars
    // including one `=` pad; the `fernet` crate's `Fernet::new` accepts
    // exactly that representation.
    let key_b64 = URL_SAFE.encode(okm);
    fernet::Fernet::new(&key_b64).ok_or(CryptoError::Fernet)
}

/// Encrypt a tenant's OpenAI API key. Returns Fernet ciphertext suitable
/// for MongoDB storage.
pub fn encrypt_openai_key(
    master_key: &[u8],
    tenant_id: TenantId,
    plaintext: &str,
) -> Result<String, CryptoError> {
    let fernet = derive_fernet(master_key, tenant_id)?;
    Ok(fernet.encrypt(plaintext.as_bytes()))
}

/// Decrypt a tenant's stored OpenAI API key. Call only within request
/// scope — never persist the returned plaintext.
pub fn decrypt_openai_key(
    master_key: &[u8],
    tenant_id: TenantId,
    ciphertext: &str,
) -> Result<String, CryptoError> {
    let fernet = derive_fernet(master_key, tenant_id)?;
    let bytes = fernet
        .decrypt(ciphertext)
        .map_err(|_| CryptoError::InvalidCiphertext)?;
    String::from_utf8(bytes).map_err(|_| CryptoError::InvalidCiphertext)
}

/// Mask an OpenAI API key for display. Returns `"sk-...{last6}"` if the
/// key is at least 6 chars, else `"sk-...******"`.
pub fn mask_openai_key(plaintext: &str) -> String {
    if plaintext.len() < 6 {
        return "sk-...******".to_string();
    }
    let last6: String = plaintext.chars().rev().take(6).collect::<String>();
    // rev().take(6) reverses — flip back.
    let last6: String = last6.chars().rev().collect();
    format!("sk-...{last6}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn master() -> Vec<u8> {
        vec![0x42; 32]
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let tid = TenantId::new();
        let master = master();
        let plain = "sk-testkey-123456";
        let ct = encrypt_openai_key(&master, tid, plain).unwrap();
        let back = decrypt_openai_key(&master, tid, &ct).unwrap();
        assert_eq!(back, plain);
    }

    #[test]
    fn different_tenants_cannot_decrypt_each_others_ciphertext() {
        let a = TenantId::new();
        let b = TenantId::new();
        let master = master();
        let ct = encrypt_openai_key(&master, a, "secret").unwrap();
        assert!(decrypt_openai_key(&master, b, &ct).is_err());
    }

    #[test]
    fn mask_handles_short_and_normal_keys() {
        assert_eq!(mask_openai_key("sk-short"), "sk-...-short");
        assert_eq!(mask_openai_key("abc"), "sk-...******");
        assert_eq!(mask_openai_key("sk-proj-abcdef123456"), "sk-...123456");
    }

    #[test]
    fn derivation_is_deterministic_same_tenant() {
        let tid = TenantId::new();
        let master = master();
        let ct1 = encrypt_openai_key(&master, tid, "x").unwrap();
        let ct2 = encrypt_openai_key(&master, tid, "x").unwrap();
        // Ciphertexts differ each call (Fernet includes a fresh IV) but
        // both decrypt with the same derived key.
        assert_ne!(ct1, ct2);
        assert_eq!(decrypt_openai_key(&master, tid, &ct1).unwrap(), "x");
        assert_eq!(decrypt_openai_key(&master, tid, &ct2).unwrap(), "x");
    }
}
