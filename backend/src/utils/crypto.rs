use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use sha2::{Digest, Sha256};
use thiserror::Error;

const VERSION: &str = "v1";
const NONCE_LEN: usize = 12;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("invalid encryption key")]
    InvalidKey,
    #[error("encryption failed")]
    Encrypt,
    #[error("invalid encrypted secret format")]
    InvalidFormat,
    #[error("decryption failed")]
    Decrypt,
    #[error("random source unavailable")]
    Random,
}

pub fn encrypt(secret_key: &str, plaintext: &str) -> Result<String, CryptoError> {
    let cipher = cipher(secret_key)?;
    let nonce_bytes = random_nonce()?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_bytes())
        .map_err(|_| CryptoError::Encrypt)?;

    Ok(format!(
        "{VERSION}:{}:{}",
        STANDARD_NO_PAD.encode(nonce_bytes),
        STANDARD_NO_PAD.encode(ciphertext)
    ))
}

#[allow(dead_code)]
pub fn decrypt(secret_key: &str, stored_value: &str) -> Result<String, CryptoError> {
    let mut parts = stored_value.split(':');
    let version = parts.next().ok_or(CryptoError::InvalidFormat)?;
    let nonce = parts.next().ok_or(CryptoError::InvalidFormat)?;
    let ciphertext = parts.next().ok_or(CryptoError::InvalidFormat)?;

    if version != VERSION || parts.next().is_some() {
        return Err(CryptoError::InvalidFormat);
    }

    let nonce = STANDARD_NO_PAD
        .decode(nonce)
        .map_err(|_| CryptoError::InvalidFormat)?;
    if nonce.len() != NONCE_LEN {
        return Err(CryptoError::InvalidFormat);
    }

    let ciphertext = STANDARD_NO_PAD
        .decode(ciphertext)
        .map_err(|_| CryptoError::InvalidFormat)?;

    let cipher = cipher(secret_key)?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| CryptoError::Decrypt)?;

    String::from_utf8(plaintext).map_err(|_| CryptoError::Decrypt)
}

fn cipher(secret_key: &str) -> Result<Aes256Gcm, CryptoError> {
    let key = Sha256::digest(secret_key.as_bytes());
    Aes256Gcm::new_from_slice(&key).map_err(|_| CryptoError::InvalidKey)
}

pub fn generate_db_password() -> String {
    const CHARSET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("random source unavailable");
    bytes
        .iter()
        .map(|&b| CHARSET[(b as usize) % CHARSET.len()] as char)
        .collect()
}

fn random_nonce() -> Result<[u8; NONCE_LEN], CryptoError> {
    let mut nonce = [0_u8; NONCE_LEN];
    getrandom::getrandom(&mut nonce).map_err(|_| CryptoError::Random)?;
    Ok(nonce)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "test-secret-key-with-at-least-32c!";

    /// encrypt then decrypt must recover the original plaintext.
    #[test]
    fn round_trip() {
        let plaintext = "postgres://user:pass@localhost:5432/mydb";
        let ciphertext = encrypt(KEY, plaintext).expect("encrypt failed");
        let recovered = decrypt(KEY, &ciphertext).expect("decrypt failed");
        assert_eq!(recovered, plaintext);
    }

    /// Two encryptions of the same plaintext must produce different ciphertexts
    /// because each call uses a freshly generated random nonce.
    #[test]
    fn same_plaintext_produces_different_ciphertexts() {
        let plaintext = "secret-value";
        let first = encrypt(KEY, plaintext).expect("first encrypt failed");
        let second = encrypt(KEY, plaintext).expect("second encrypt failed");
        assert_ne!(first, second, "ciphertexts must differ due to random nonce");
        // Both must still decrypt correctly.
        assert_eq!(decrypt(KEY, &first).unwrap(), plaintext);
        assert_eq!(decrypt(KEY, &second).unwrap(), plaintext);
    }
}
