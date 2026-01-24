# AxKeyStore

AxKeyStore is a secure, open-source command-line interface (CLI) tool designed to manage your secrets, keys, and passwords. It leverages your own private GitHub repository as the secure storage backend, ensuring your data is accessible, versioned, and under your control.

## 🔒 Security First (Zero Trust)

AxKeyStore is built on a **Zero Trust** architecture:
- **Client-Side Encryption**: All secrets are encrypted locally on your machine *before* they are ever sent to the network. Authentication keys and passwords serve as the encryption key source.
- **Untrusted Storage**: The remote GitHub repository is treated as untrusted storage. It only ever sees encrypted binary blobs.
- **Secure Algorithms**: Uses modern, authenticated encryption standards (XChaCha20-Poly1305 or AES-GCM) and robust key derivation (Argon2id).

## 🚀 Features

- **GitHub Storage**: Utilizes a private repository on your GitHub account for free, reliable, and versioned cloud storage.
- **Device Authentication**: Authenticates securely using GitHub's OAuth Device Flow.
- **Cross-Platform**: Built with Rust for performance and portability across macOS, Linux, and Windows.
- **Simple CLI**: Easy-to-use commands to store and retrieve your credentials.

## 🛠 Tech Stack

- **Language**: Rust
- **CLI Framework**: `clap`
- **Async Runtime**: `tokio`
- **Crypto**: `argon2`, `chacha20poly1305`, `rand`

## ✨ Usage

1. **Login**: Authenticate with your GitHub account.
   ```bash
   axkeystore login
   ```

2. **Initialize**: Set up a repository for storage (if not already done).
   ```bash
   axkeystore init --repo my-secret-store
   ```

3. **Store a Secret**: Encrypt and upload a key/password.
   ```bash
   axkeystore store --key "my-api-key" --value "super_secret_value"
   ```

4. **Retrieve a Secret**: Download and decrypt a key.
   ```bash
   axkeystore get "my-api-key"
   ```

## 📦 Installation

*(Instructions coming soon)*

## 📄 License

[MIT License](LICENSE)
