---
name: Cryptography Handling Skill
description: Instructions for securely encrypting and decrypting data.
---

# Cryptography Handling Skill

This skill defines how to handle encryption for the `AxKeyStore` before data is sent to GitHub.

## Philosophy
The cloud (GitHub) is untrusted. All data must be encrypted at rest (locally before upload) and in transit (via HTTPS).

## Implementation Details

### Master Password
- The user must provide a master password or passphrase to unlock the keystore.
- **Key Derivation**: Use `Argon2id` or `PBKDF2` to derive a symmetric encryption key from the user's password + salt.

### Encryption Schema
- **Algorithm**: XChaCha20-Poly1305 or AES-256-GCM.
- **Library**: Use the `age` crate (for file encryption) or `ring`/`sodiumoxide` for low-level primitives.
- **Structure**:
  ```json
  {
    "salt": "random_salt_for_kdf",
    "nonce": "random_nonce",
    "ciphertext": "base64_encoded_encrypted_data"
  }
  ```

### Decryption Flow
1. Read the JSON blob from GitHub.
2. Extract `salt`, `nonce`, and `ciphertext`.
3. Prompt user for Master Password.
4. Derive Key using Salt + Password.
5. Decrypt `ciphertext` using Key + Nonce.

## Dependencies
- `argon2` (for KDF)
- `chacha20poly1305` or `aes-gcm` (for encryption)
- `rand` (for generating salt/nonce)
