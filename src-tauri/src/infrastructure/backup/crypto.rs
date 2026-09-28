//! Password encryption for backup archives (17-backup.md §3.1), byte-compatible with
//! `src/modules/settings/helpers/backupCrypto.ts`: PBKDF2-HMAC-SHA256 key derivation, AES-256-GCM
//! with a 12-byte IV, ciphertext‖tag (the WebCrypto layout — `aes-gcm`'s `encrypt`/`decrypt` already
//! append/verify the 16-byte tag at the end of the buffer, matching `crypto.subtle.encrypt`'s output
//! exactly). `manifest.json` is never encrypted; salt/IV are random per archive.

use aes_gcm::aead::{Aead, KeyInit, Nonce};
use aes_gcm::{Aes256Gcm, Key};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

use crate::core::error::AppError;

/// `backupCrypto.ts:8`.
pub const PBKDF2_ITERATIONS: u32 = 150_000;
const SALT_LEN: usize = 16;
const IV_LEN: usize = 12;
const KEY_LEN: usize = 32; // 256 bits

pub struct EncryptedPayload {
    pub ciphertext: Vec<u8>,
    pub salt_b64: String,
    pub iv_b64: String,
    pub iterations: u32,
}

fn derive_key(password: &str, salt: &[u8], iterations: u32) -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iterations, &mut key);
    key
}

/// Encrypts `plain` with a freshly generated random salt/IV — the write path (`build_backup_archive`).
/// Random bytes come from `getrandom` (the OS CSPRNG), the same source
/// `infrastructure::database::credentials` already uses (G-46-adjacent precedent) rather than
/// pulling in the `rand` crate as a new direct dependency.
pub fn encrypt_bytes(plain: &[u8], password: &str) -> Result<EncryptedPayload, AppError> {
    let mut salt = [0u8; SALT_LEN];
    let mut iv = [0u8; IV_LEN];
    getrandom::fill(&mut salt).map_err(|e| AppError::internal("تعذر توليد بيانات عشوائية آمنة", Some(e.to_string())))?;
    getrandom::fill(&mut iv).map_err(|e| AppError::internal("تعذر توليد بيانات عشوائية آمنة", Some(e.to_string())))?;

    let key_bytes = derive_key(password, &salt, PBKDF2_ITERATIONS);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::<Aes256Gcm>::from_slice(&iv);
    let ciphertext = cipher
        .encrypt(nonce, plain)
        .map_err(|_| AppError::internal("تعذر تشفير النسخة الاحتياطية", None))?;

    Ok(EncryptedPayload {
        ciphertext,
        salt_b64: BASE64.encode(salt),
        iv_b64: BASE64.encode(iv),
        iterations: PBKDF2_ITERATIONS,
    })
}

/// Decrypts `ciphertext` given the salt/IV/iterations recorded in `crypto.json`. Wrong password or
/// a corrupted/tampered ciphertext both fail the AES-GCM auth tag check — one user-facing message
/// for both, exactly `backupCrypto.ts:65`'s text.
pub fn decrypt_bytes(ciphertext: &[u8], password: &str, salt_b64: &str, iv_b64: &str, iterations: u32) -> Result<Vec<u8>, AppError> {
    let wrong_password_or_corrupt = || AppError::validation("كلمة المرور غير صحيحة، أو الملف تالف");

    let salt = BASE64.decode(salt_b64).map_err(|_| wrong_password_or_corrupt())?;
    let iv = BASE64.decode(iv_b64).map_err(|_| wrong_password_or_corrupt())?;
    if iv.len() != IV_LEN {
        return Err(wrong_password_or_corrupt());
    }

    let key_bytes = derive_key(password, &salt, iterations);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::<Aes256Gcm>::from_slice(&iv);
    cipher.decrypt(nonce, ciphertext).map_err(|_| wrong_password_or_corrupt())
}

/// SHA-256 checksum (hex) — the manifest's `checksum` field (`backupCrypto.ts:70`).
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_encrypts_and_decrypts() {
        let plain = b"hello world, this is a backup payload";
        let enc = encrypt_bytes(plain, "correct horse").unwrap();
        let dec = decrypt_bytes(&enc.ciphertext, "correct horse", &enc.salt_b64, &enc.iv_b64, enc.iterations).unwrap();
        assert_eq!(dec, plain);
    }

    #[test]
    fn wrong_password_fails_with_the_exact_arabic_message() {
        let plain = b"secret data";
        let enc = encrypt_bytes(plain, "right password").unwrap();
        let err = decrypt_bytes(&enc.ciphertext, "wrong password", &enc.salt_b64, &enc.iv_b64, enc.iterations).unwrap_err();
        assert_eq!(err.to_string(), "كلمة المرور غير صحيحة، أو الملف تالف");
    }

    #[test]
    fn flipped_byte_fails_the_auth_tag() {
        let plain = b"some payload bytes";
        let enc = encrypt_bytes(plain, "pw").unwrap();
        let mut tampered = enc.ciphertext.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 0x01;
        let err = decrypt_bytes(&tampered, "pw", &enc.salt_b64, &enc.iv_b64, enc.iterations).unwrap_err();
        assert_eq!(err.to_string(), "كلمة المرور غير صحيحة، أو الملف تالف");
    }

    #[test]
    fn sha256_hex_matches_known_vector() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855.
        assert_eq!(sha256_hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }
}
