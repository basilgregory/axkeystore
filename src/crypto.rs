use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct EncryptedBlob {
    pub salt: String,
    pub nonce: String,
    pub ciphertext: String,
}

pub struct CryptoHandler;

impl CryptoHandler {
    /// Generates a 36-character random alphanumeric string for the master key
    pub fn generate_master_key() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        (0..36)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Derives a 32-byte key from a password and salt using Argon2id
    fn derive_key(password: &str, salt: &str) -> Result<[u8; 32]> {
        let salt =
            SaltString::from_b64(salt).map_err(|e| anyhow::anyhow!("Invalid salt: {}", e))?;

        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Key derivation failed: {}", e))?;

        let hash = password_hash.hash.context("No hash found")?;

        // Ensure we have enough bytes, XChaCha20Poly1305 key is 32 bytes
        let mut key = [0u8; 32];
        let output_bytes = hash.as_bytes();
        if output_bytes.len() < 32 {
            return Err(anyhow::anyhow!("Derived key too short"));
        }
        key.copy_from_slice(&output_bytes[0..32]);

        Ok(key)
    }

    pub fn encrypt(data: &[u8], password: &str) -> Result<EncryptedBlob> {
        let salt = SaltString::generate(&mut OsRng);
        let key = Self::derive_key(password, salt.as_str())?;

        let cipher = XChaCha20Poly1305::new(&key.into());
        let mut nonce_bytes = [0u8; 24]; // XChaCha20 uses 24-byte nonce
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(
                nonce,
                Payload {
                    msg: data,
                    aad: &[],
                },
            )
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        Ok(EncryptedBlob {
            salt: salt.as_str().to_string(),
            nonce: BASE64.encode(nonce_bytes),
            ciphertext: BASE64.encode(ciphertext),
        })
    }

    pub fn decrypt(blob: &EncryptedBlob, password: &str) -> Result<Vec<u8>> {
        let key = Self::derive_key(password, &blob.salt)?;

        let cipher = XChaCha20Poly1305::new(&key.into());

        let nonce_bytes = BASE64.decode(&blob.nonce).context("Invalid nonce base64")?;
        if nonce_bytes.len() != 24 {
            return Err(anyhow::anyhow!("Invalid nonce length"));
        }
        let nonce = XNonce::from_slice(&nonce_bytes);

        let ciphertext = BASE64
            .decode(&blob.ciphertext)
            .context("Invalid ciphertext base64")?;

        let plaintext = cipher
            .decrypt(
                nonce,
                Payload {
                    msg: &ciphertext,
                    aad: &[],
                },
            )
            .map_err(|_| anyhow::anyhow!("Decryption failed - wrong password?"))?;

        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_success() {
        let password = "complex_password_123";
        let data = b"secret data content";

        let encrypted = CryptoHandler::encrypt(data, password).unwrap();

        // Sanity check structure
        assert!(!encrypted.salt.is_empty());
        assert!(!encrypted.nonce.is_empty());
        assert!(!encrypted.ciphertext.is_empty());

        let decrypted = CryptoHandler::decrypt(&encrypted, password).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_decrypt_wrong_password() {
        let password = "correct_password";
        let data = b"secret data";

        let encrypted = CryptoHandler::encrypt(data, password).unwrap();

        // Try decrypting with wrong password
        let result = CryptoHandler::decrypt(&encrypted, "wrong_password");
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_is_random() {
        let password = "password";
        let data = b"data";

        let enc1 = CryptoHandler::encrypt(data, password).unwrap();
        let enc2 = CryptoHandler::encrypt(data, password).unwrap();

        // Salt and nonce should be random, so ciphertexts should differ
        assert_ne!(enc1.salt, enc2.salt);
        assert_ne!(enc1.nonce, enc2.nonce);
        assert_ne!(enc1.ciphertext, enc2.ciphertext);
    }
}
