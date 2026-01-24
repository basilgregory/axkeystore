---
description: Initialize the AxKeyStore Rust Project
---

# Initialize AxKeyStore Project

This workflow sets up the basic structure and dependencies for the AxKeyStore CLI.

## Steps

1. **Verify Cargo**: Ensure `cargo` is installed and the directory is a Rust project.
   - Run `cargo --version`.
   - If `Cargo.toml` exists, skip `cargo init`.

2. **Add Dependencies**:
   - Run the following commands to add necessary crates:
     ```bash
     cargo add clap --features derive
     cargo add tokio --features full
     cargo add reqwest --features json
     cargo add serde --features derive
     cargo add serde_json
     cargo add anyhow
     cargo add directories
     cargo add chacha20poly1305
     cargo add rand
     cargo add argon2
     cargo add base64
     ```
// turbo-all
3. **Create Module Structure**:
   - Create `src/auth.rs`.
   - Create `src/storage.rs`.
   - Create `src/crypto.rs`.
   - Create `src/config.rs`.

4. **Update `src/main.rs`**:
   - Set up the basic `clap` command structure with subcommands:
     - `login`: for authentication.
     - `store`: for adding a key.
     - `get`: for retrieving a key.
     - `init`: for setting up the repo.

5. **Verify Build**:
   - Run `cargo build` to ensure all dependencies resolve correctly.
