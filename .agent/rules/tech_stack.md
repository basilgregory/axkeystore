# Tech Stack & Coding Standards

## Core Technologies
- **Language**: Rust (Edition 2021 or later)
- **CLI Framework**: `clap` (derive feature)
- **Async Runtime**: `tokio`
- **HTTP/API**: `reqwest` or `octocrab` for GitHub API interactions
- **Serialization**: `serde` and `serde_json`
- **Encryption**: `age` or `ring` (Strong AES-GCM or ChaCha20-Poly1305)

## coding Standards
- **Error Handling**: Use `anyhow` for application-level error handling and `thiserror` for library-level errors.
- **Config Management**: Store local configuration (like auth tokens) in the system's standard configuration directory (e.g., using `directories` crate).
- **Formatting**: run `cargo fmt` before committing.
- **Linting**: run `cargo clippy` and ensure no warnings.

## Project Structure
- `src/main.rs`: Entry point, CLI parsing.
- `src/auth.rs`: GitHub OAuth authentication logic.
- `src/storage.rs`: Interaction with the GitHub repository (read/write files).
- `src/encryption.rs`: Encryption and decryption logic.
- `src/config.rs`: Local configuration management.
