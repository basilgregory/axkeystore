use anyhow::{Context, Result};
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
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
    /// Derives a 32-byte key from a password and salt using Argon2id
    fn derive_key(password: &str, salt: &str) -> Result<[u8; 32]> {
        let salt = SaltString::from_b64(salt).context("Invalid salt")?;

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
