# AxKeyStore

AxKeyStore is a secure, open-source command-line interface (CLI) tool designed to manage your secrets, keys, and passwords. It leverages your own private GitHub repository as the secure storage backend, ensuring your data is accessible, versioned, and under your control. Data travels encrypted over the wire and is stored encrypted in the remote repository. No secrets are ever stored in plain text in the remote repository. Also, no secrets are ever stored in the local filesystem or on any other remote server.

> **AxKeyStore** is an **Open Source Project** built by **Appxiom Team**
> Visit [https://www.appxiom.com](https://www.appxiom.com) to know more about us.
> You will love our product if you are into software engineering!

> MIT License

## Security First (Zero Trust)

AxKeyStore is built on a **Zero Trust** architecture with a robust multi-layered encryption scheme:

- **Local Master Key (LMK)**: A 36-character random alphanumeric string generated uniquely for each profile and stored on your local machine.
  - **Purpose**: Encrypts your sensitive local configuration, including your GitHub access token and the name of your private repository.
  - **Security**: The LMK itself is encrypted with the user's **Master Password** using `Argon2id` and `XChaCha20-Poly1305`.

- **Remote Master Key (RMK)**: A 36-character random alphanumeric string generated uniquely for your vault and stored on GitHub.
  - **Purpose**: Encrypts the actual secrets (keys/passwords) stored in your repository.
  - **Security**: The RMK is encrypted with the user's **Master Password** (via client-side encryption) before being uploaded to GitHub.

- **Three-Layer Encryption**:
  1.  **Secrets** are encrypted using the **RMK**.
  2.  **RMK** is encrypted using your **Master Password** and stored on GitHub.
  3.  **Local Credentials** (Token/Repo Name) are encrypted using the **LMK**, which is also secured by your **Master Password**.

- **Client-Side Encryption**: All encryption happens purely on your machine. No plain-text secrets, master keys, or passwords ever touch the network or are stored unencrypted on disk.
- **Untrusted Storage**: GitHub is treated as untrusted cloud storage. It only ever sees encrypted binary blobs.
- **Secure Algorithms**: Uses modern, authenticated encryption standards (`XChaCha20-Poly1305`) and robust key derivation (`Argon2id`).

## Features

- **GitHub Storage**: Utilizes a private repository on your GitHub account or an organization for free, reliable, and versioned cloud storage.
- **Unified Auth**: Authenticates securely using GitHub Apps and the Device Flow.
- **Installation Management**: Simple one-time installation to grant access to specific repositories.
- **Simple CLI**: Easy-to-use commands to store and retrieve your credentials.
- **Category Organization**: Organize your secrets in hierarchical categories (e.g., `api/production/internal`).
- **Multi-Profile Support**: Manage multiple vaults with different logins, master passwords, and GitHub repositories.

## Installation

### macOS / Linux

To install on macOS or Linux, run:

```bash
curl -sSL https://raw.githubusercontent.com/basilgregory/axkeystore/main/install.sh | bash
```

To install a specific version on macOS or Linux, run:

```bash
curl -sSL https://raw.githubusercontent.com/basilgregory/axkeystore/main/install.sh | bash -s v0.1.6
```

### Windows

To install on Windows, run the following command in **PowerShell**:

```powershell
powershell -c "irm https://raw.githubusercontent.com/basilgregory/axkeystore/main/install.ps1 | iex"
```

To install a specific version on Windows, run:

```powershell
powershell -c "irm https://raw.githubusercontent.com/basilgregory/axkeystore/main/install.ps1 | iex -s v0.1.6"
```

The scripts will download the appropriate binary, move it to `$HOME/.axkeystore/bin`, and automatically configure your `PATH`.

## Usage

1. **Login**: Authenticate with your GitHub account.

   ```bash
   axkeystore login
   ```

   > **Note**: During your first login, you will be prompted to set a **Master Password**. This password is used to encrypt your sensitive GitHub access token locally on your machine.
   >
   > **GitHub App Installation**: After logging in, the CLI will provide a link to install the app on your GitHub account or organization: `https://github.com/apps/<app-name>/installations/new`. You **must** install the app to grant it access to your repositories.

2. **Initialize**: Set up a repository for storage (if not already done).

   ```bash
   # Use a repo in your account
   axkeystore init --repo my-secret-store

   # Or specify an organization/owner
   axkeystore init --repo my-org/my-secret-store
   ```

   > **Note**: If the repository already exists and has been initialized previously (e.g., on another machine), AxKeyStore will prompt for your **Master Password** to verify access. You must provide the correct password associated with that repository to proceed.

3. **Store a Secret**: Encrypt and upload a key/password.

   ```bash
   axkeystore store --key "my-api-key" --value "super_secret_value"
   ```

   > **Note**: You **must** run `axkeystore init` before storing or retrieving any keys. If the repository is not configured, you will be prompted to do so. You must enter your **Master Password** for every operation to unlock your local session and vault.

4. **Auto-Generate a Secret**: If you don't provide a value, AxKeyStore will generate a secure random alphanumeric value (6-36 characters) for you.

   ```bash
   axkeystore store --key "my-api-key"
   ```

   You'll see the generated value and be asked to confirm before storing:

   ```
   Generated value: qOmH8qHQ3pnuASPrho662Mqd
      (Length: 24 characters)

   Do you want to use this generated value? (y/n):
   ```

5. **Retrieve a Secret**: Download and decrypt a key.

   ```bash
   axkeystore get "my-api-key"
   ```

6. **View Version History**: List previous versions of a key (10 at a time).

   ```bash
   axkeystore history "my-api-key"
   ```

   This will show a table with the SHA, date, and commit message for each version.

7. **Retrieve a Specific Version**: Use the SHA from history to retrieve a previous value.

   ```bash
   axkeystore get "my-api-key" --version <SHA>
   ```

8. **Store with Category**: Organize secrets in hierarchical categories.

   ```bash
   axkeystore store --key "aws-key" --value "AKIAIOSFODNN7EXAMPLE" --category "cloud/aws/production"
   ```

   > **Tip**: You can also auto-generate values with categories:
   >
   > ```bash
   > axkeystore store --key "aws-key" --category "cloud/aws/production"
   > ```

9. **Retrieve from Category**: Retrieve a secret from a specific category.

   ```bash
   axkeystore get "aws-key" --category "cloud/aws/production"
   ```

10. **Delete a Secret**: Delete a stored key (with confirmation prompt).

    ```bash
    axkeystore delete "my-api-key"
    ```

11. **Delete from Category**: Delete a secret from a specific category.

    ```bash
    axkeystore delete "aws-key" --category "cloud/aws/production"
    ```

12. **Reset Master Password**: Update your master password safely.

    ```bash
    axkeystore reset-password
    ```

    > **Note**: This command safely re-encrypts both your **Local Master Key** and your **Remote Master Key** with the new password. The process is transactional: it updates the remote key on GitHub _first_, and only on success does it update the local configuration.

13. **Manage Profiles**: AxKeyStore supports multiple profiles, each with its own master password, GitHub repository, and token.

    ```bash
    # List all profiles
    axkeystore profile list

    # Create a new profile
    axkeystore profile create "work"

    # Switch to a profile (makes it the default for all commands)
    axkeystore profile switch "work"

    # Switch back to the default root profile
    axkeystore profile switch

    # Show the currently active profile
    axkeystore profile current

    # Delete a profile (and all its local configuration)
    axkeystore profile delete "work"

    # Use a specific profile for a single command without switching
    axkeystore --profile "personal" get "my-key"
    ```

### Profile Rules

- Profile names can only contain alphanumeric characters, dashes (`-`), and underscores (`_`).
- If no profile is specified and no profile has been set as active, the CLI uses the default "root" configuration directory.
- Each profile has its own isolated master password and local configuration.

### Category Path Rules

- Categories can be nested using `/` separator (e.g., `api/production/internal`)
- Category segments can only contain alphanumeric characters, dashes (`-`), and underscores (`_`)
- Key names cannot contain path separators
- Categories are optional; keys can be stored without any category

## Developer Guide

### Tech Stack

- **Language**: Rust
- **CLI Framework**: `clap`
- **Async Runtime**: `tokio`
- **Crypto**: `argon2`, `chacha20poly1305`, `rand`
- **Path Resolution**: `directories`

### Internal Configuration Structure

AxKeyStore stores its configuration in the user's standard config directory (e.g., `~/Library/Application Support/com.ax.axkeystore` on macOS).

```text
com.ax.axkeystore/
├── global.json                  # Stores the active profile name
├── github_token.json            # Profile-specific encrypted token (via LMK)
├── config.json                  # Profile-specific LMK and Repo config
└── <profile_name>/              # Subdirectory for each named profile
    ├── github_token.json        # Sub-profile specific encrypted token
    └── config.json              # Sub-profile specific LMK and Repo config
```

> **Note**: The `config.json` file contains your **Local Master Key**, which is encrypted with your **Master Password**. All other sensitive local files (like `github_token.json`) are encrypted using that LMK.

### Running Locally

During development, you can run AxKeyStore directly using `cargo`. Use `--` to separate cargo arguments from the CLI arguments:

```bash
# Authenticate (and follow the installation link provided)
cargo run -- login

# Initialize your vault (supports owner/repo format)
cargo run -- init --repo axkeystore-storage
cargo run -- init --repo my-org/my-keystore

# Store a secret
cargo run -- store --key "api-token" --value "secret123"

# Get a secret
cargo run -- get "api-token"

# List version history
cargo run -- history "api-token"

# Delete a secret
cargo run -- delete "api-token"

# Working with categories
cargo run -- store --key "db-pass" --category "prod/database" --value "top_secret"
cargo run -- get "db-pass" --category "prod/database"
cargo run -- history "db-pass" --category "prod/database"
cargo run -- delete "db-pass" --category "prod/database"

# Managing Profiles
cargo run -- profile list
cargo run -- profile create work
cargo run -- profile switch work
cargo run -- profile current
cargo run -- profile switch
cargo run -- profile delete work

# Using a specific profile flag
cargo run -- --profile personal get "my-key"

# Reset Master Password
cargo run -- reset-password
```

### Testing

AxKeyStore includes a robust suite of unit and integration tests. You can run them using:

```bash
cargo test
```

#### Test Coverage:

- **`crypto`**: Verified authenticated encryption (XChaCha20-Poly1305), tamper detection, and Argon2id key derivation.
- **`auth`**: Tests for GitHub Device Flow response parsing and secure local token persistence.
- **`config`**: Validates global and profile-specific configuration encryption, isolation, profile name rules, and **Local Master Key (LMK)** generation and persistence.
- **`storage`**: Uses **`wiremock`** to simulate the GitHub API, testing repository initialization, version history retrieval, and hierarchical category validation in a profile-aware context.

> **Note**: Tests that modify process-wide environment variables (like API URLs) are synchronized using an internal `Mutex` to ensure stability when running in parallel.

### How it Works

The following diagrams illustrate the internal logic and interactions for each command.

#### 1. Login Flow

Authenticates the user via GitHub Device OAuth and secures the resulting token locally.

```mermaid
sequenceDiagram
    participant U as User
    participant C as CLI
    participant G as GitHub API
    participant GC as Global Config
    participant PC as Profile Config
    participant CR as Crypto Engine

    U->>C: axkeystore login [--profile P]
    C->>GC: Resolve Effective Profile
    GC-->>C: Profile P / Active Profile
    C->>G: Request Device Code (POST /login/device/code)
    G-->>C: user_code, verification_uri, device_code
    C->>U: Display "Visit URI and enter code XXXX"
    loop Polling
        C->>G: Request Access Token (POST /login/oauth/access_token)
        G-->>C: Access Token / pending
    end
    C->>PC: Check for existing LMK
    alt LMK exists
        C->>U: Enter Master Password
        U-->>C: (Password input)
        C->>CR: Decrypt(LMK, Password)
        note right of C: Verifies password against existing LMK
    else New Profile
        C->>U: Set Master Password (for profile)
        U-->>C: (Password input)
        C->>CR: Generate 36-char Local Master Key (LMK)
        C->>CR: Encrypt(LMK, Password)
        C->>PC: Save P/config.json (Encrypted LMK)
    end
    C->>CR: Encrypt(Token, LMK)
    C->>PC: Save P/github_token.json (Encrypted Token)
    C-->>U: Logged in successfully for profile
```

#### 2. Initialization Flow

Sets up the remote repository and the encrypted master key used for all secrets.

```mermaid
sequenceDiagram
    participant U as User
    participant C as CLI
    participant G as GitHub API
    participant GC as Global Config
    participant PC as Profile Config
    participant CR as Crypto Engine

    U->>C: axkeystore init --repo MY_REPO [--profile P]
    C->>GC: Resolve Effective Profile
    C->>U: Enter Master Password
    U-->>C: (Password input)
    C->>PC: Load & Decrypt LMK (using Password)
    C->>G: Check if MY_REPO exists
    alt Repo does not exist
        C->>G: Create Private Repo (POST /user/repos)
    end
    C->>G: Get .axkeystore/master_key.json
    alt Remote Master Key (RMK) exists
        C->>CR: Decrypt(RMK, Password)
        note right of C: Verifies password against Remote Key.<br/>Aborts if incorrect.
    else RMK not found
        C->>CR: Generate 36-char RMK
        C->>CR: Encrypt(RMK, Password)
        C->>G: Upload Encrypted RMK
    end
    C->>CR: Encrypt("MY_REPO", LMK)
    C->>PC: Save P/config.json (Update Encrypted Repo Name)
    C-->>U: Initialized successfully
```

#### 3. Store Flow

Encrypts and uploads a secret. Supports auto-generation of secure values.

```mermaid
sequenceDiagram
    participant U as User
    participant C as CLI
    participant G as GitHub API
    participant GC as Global Config
    participant PC as Profile Config
    participant CR as Crypto Engine

    U->>C: axkeystore store --key KEY [--profile P]
    C->>GC: Resolve Effective Profile
    C->>U: Enter Master Password
    U-->>C: (Password input)
    C->>PC: Load & Decrypt LMK (using Password)
    C->>PC: Decrypt Repo Name (using LMK)
    C->>G: Fetch Encrypted RMK
    C->>CR: Decrypt(RMK Blob, Password)
    alt Value not provided
        C->>CR: Generate Random Secret
        C->>U: Ask "Confirm secret XXXXX?"
        U-->>C: Yes
    end
    C->>CR: Encrypt(Secret, RMK)
    CR-->>C: EncryptedBlob
    C->>G: Upload keys/KEY.json (Encrypted)
    C-->>U: Secret stored successfully
```

#### 4. Get Flow

Retrieves and decrypts a secret. Supports fetching specific versions via commit SHA.

```mermaid
sequenceDiagram
    participant U as User
    participant C as CLI
    participant G as GitHub API
    participant GC as Global Config
    participant PC as Profile Config
    participant CR as Crypto Engine

    U->>C: axkeystore get KEY [--profile P]
    C->>GC: Resolve Effective Profile
    C->>U: Enter Master Password
    U-->>C: (Password input)
    C->>PC: Load & Decrypt LMK (using Password)
    C->>PC: Decrypt Repo Name (using LMK)
    C->>G: Fetch Encrypted RMK
    C->>CR: Decrypt(RMK Blob, Password)
    alt Version provided
        C->>G: Fetch keys/KEY.json?ref=SHA
    else Current
        C->>G: Fetch keys/KEY.json
    end
    C->>CR: Decrypt(Blob, RMK)
    CR-->>C: Plaintext secret
    C-->>U: secret_value
```

#### 5. History Flow

Lists the version history (commits) of a specific key path on GitHub.

```mermaid
sequenceDiagram
    participant U as User
    participant C as CLI
    participant G as GitHub API
    participant GC as Global Config
    participant PC as Profile Config

    U->>C: axkeystore history KEY [--profile P]
    C->>GC: Resolve Effective Profile
    C->>U: Enter Master Password
    U-->>C: (Password input)
    C->>PC: Load & Decrypt Repo Name (for P)
    loop Pagination
        C->>G: Fetch Commits for path 'keys/KEY.json' (page X)
        G-->>C: List of SHAs, dates, messages
        C->>U: Display Table (SHA | Date | Message)
        C->>U: Ask "Show more versions?"
        U-->>C: Yes/No
    end
```

#### 6. Delete Flow

Removes a secret from the repository.

```mermaid
sequenceDiagram
    participant U as User
    participant C as CLI
    participant G as GitHub API
    participant GC as Global Config
    participant PC as Profile Config

    U->>C: axkeystore delete KEY [--profile P]
    C->>GC: Resolve Effective Profile
    C->>U: Enter Master Password
    U-->>C: (Password input)
    C->>PC: Load & Decrypt Repo Name (for P)
    C->>G: Fetch keys/KEY.json (to get SHA)
    C->>U: Ask "Confirm delete KEY?"
    U-->>C: Yes
    C->>G: Delete file (DELETE /contents/path) with SHA
    C-->>U: Secret deleted
```

#### 7. Profile Management Flow

Manages the active profile and profile-specific data directories.

```mermaid
sequenceDiagram
    participant U as User
    participant C as CLI
    participant GC as Global Config
    participant FS as File System

    alt profile switch [NAME]
        U->>C: axkeystore profile switch [NAME]
        C->>GC: Set active_profile = NAME (or None)
        C->>GC: Save global.json
        C-->>U: Switched to NAME / default root
    else profile list
        U->>C: axkeystore profile list
        C->>FS: Scan config root for directories
        FS-->>C: List of profile names
        C->>GC: Get active_profile
        C-->>U: Display Profile List (* active)
    else profile delete NAME
        U->>C: axkeystore profile delete NAME
        C->>FS: Remove directory config/NAME/
        alt NAME was active
            C->>GC: Clear active_profile
        end
        C-->>U: Profile deleted
    end
```

#### 8. Password Reset Flow

Transactional update of both Remote and Local Master Keys.

```mermaid
sequenceDiagram
    participant U as User
    participant C as CLI
    participant G as GitHub API
    participant PC as Profile Config
    participant CR as Crypto Engine

    U->>C: axkeystore reset-password
    C->>U: Enter current Master Password
    U-->>C: (Password input)
    C->>PC: Load & Decrypt LMK (verify old password)
    alt Repo configured
        C->>PC: Decrypt Repo Name
        C->>G: Fetch Encrypted RMK
        C->>CR: Decrypt(RMK, old password)
    end
    C->>U: Enter New Master Password (twice)
    U-->>C: (New Password)
    alt RMK exists
        C->>CR: Encrypt(RMK, new password)
        C->>G: Upload Updated RMK
        note right of G: If this fails, the process aborts.
    end
    C->>CR: Encrypt(LMK, new password)
    C->>PC: Save P/config.json (Updated LMK)
    C-->>U: Master password reset successfully
```

### Setup

To use AxKeyStore as YOUR OWN application, you need to register a GitHub App to get a Client ID:

1. Go to [GitHub Developer Settings > GitHub Apps](https://github.com/settings/developers).
2. Click **New GitHub App**.
3. Fill in the details:
   - **GitHub App name**: A UNIQUE NAME OF YOUR CHOICE
   - **Homepage URL**: `https://www.appxiom.com` or your site.
   - **Permissions**:
     - **Repository permissions > Contents**: Read & write (required for storing/retriving keys)
     - **Repository permissions > Metadata**: Read-only (required)
   - **Where can this GitHub App be installed?**: "Any account" or "Only on this account"
   - **Enable Device Flow** (scroll down to find this checkbox).
4. Click **Create GitHub App**.
5. Note your **App Name** (slug) and **Client ID**.
6. Update your `.env` file:
   ```env
   GITHUB_CLIENT_ID=your_client_id
   GITHUB_APP_NAME=your_app_slug
   ```

## License

[MIT License](LICENSE)
