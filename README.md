# AxKeyStore

> ⭐ **AxKeyStore** is an **Open Source Project** built by **Appxiom Team**

> AxKeyStore is a secure, open-source command-line interface (CLI) tool designed to manage your secrets, keys, and passwords. It leverages your own private GitHub repository as the secure storage backend, ensuring your data is accessible, versioned, and under your control. Data travels encrypted over the wire and is stored encrypted in the remote repository. No secrets are ever stored in plain text in the remote repository. Also, no secrets are ever stored in the local filesystem or on any other remote server. 
>
> Visit [https://www.appxiom.com](https://www.appxiom.com) to know more about us.
> You will love our product if you are into software engineering!

> MIT License

## 🔒 Security First (Zero Trust)

AxKeyStore is built on a **Zero Trust** architecture:
- **Two-Layer Encryption**: 
    1.  **Master Key**: A 36-character random alphanumeric string is generated uniquely for your vault. This key is used to encrypt all your secrets.
    2.  **Master Password**: Your master password encrypts the Master Key using `Argon2id` and `XChaCha20-Poly1305`. This encrypted blob is stored in your private GitHub repo.
- **Client-Side Encryption**: All secrets are encrypted locally on your machine *before* they are ever sent to the network. The Master Key is decrypted into memory only when needed and never touches the disk in plain text.
- **Untrusted Storage**: The remote GitHub repository is treated as untrusted storage. It only ever sees encrypted binary blobs for both your secrets and your Master Key.
- **Secure Algorithms**: Uses modern, authenticated encryption standards (XChaCha20-Poly1305) and robust key derivation (Argon2id).

## 🚀 Features

- **GitHub Storage**: Utilizes a private repository on your GitHub account for free, reliable, and versioned cloud storage.
- **Device Authentication**: Authenticates securely using GitHub's OAuth Device Flow.
- **Cross-Platform**: Built with Rust for performance and portability across macOS, Linux, and Windows.
- **Simple CLI**: Easy-to-use commands to store and retrieve your credentials.
- **Category Organization**: Organize your secrets in hierarchical categories (e.g., `api/production/internal`).

## 🛠 Tech Stack

- **Language**: Rust
- **CLI Framework**: `clap`
- **Async Runtime**: `tokio`
- **Crypto**: `argon2`, `chacha20poly1305`, `rand`

## 🔄 How it Works

The following flowchart illustrates how AxKeyStore interacts with the User, GitHub, and Local Storage during different operations:

```mermaid
graph TD
    User((User))
    CLI[AxKeyStore CLI]
    GitHub[GitHub API]
    LocalConfig[Local Config]
    Crypto[Crypto Engine]

    User --> CLI

    subgraph "Commands"
        direction TB
        Login[login]
        Init[init]
        Store[store]
        Get[get]
    end

    CLI --> Login
    CLI --> Init
    CLI --> Store
    CLI --> Get

    %% Login Flow
    Login -- "1. Req Device Code" --> GitHub
    Login -- "2. Show Code" --> User
    User -. "3. Authorize" .-> GitHub
    Login -- "4. Poll Token" --> GitHub
    Login -- "5. Save Token" --> LocalConfig

    %% Init Flow
    Init -- "1. Check/Create Repo" --> GitHub
    Init -- "2. Save Repo Name" --> LocalConfig

    %% Store Flow
    Store -- "1. Get Repo Name" --> LocalConfig
    Store -- "2. Get/Init Master Key" --> MK_Flow
    subgraph MK_Flow [Master Key Flow]
        direction TB
        MK_Exists{Exists?}
        MK_Fetch[Fetch from GitHub]
        MK_Prompt[Prompt Master Password]
        MK_Decrypt[Decrypt with Password]
        MK_Gen[Generate 36-char Key]
        MK_Encrypt[Encrypt with Password]
        MK_Upload[Upload to GitHub]

        MK_Exists -- Yes --> MK_Fetch --> MK_Prompt --> MK_Decrypt
        MK_Exists -- No --> MK_Gen --> MK_Prompt --> MK_Encrypt --> MK_Upload
    end
    MK_Flow -- "Returns Decrypted MK" --> Store
    Store -- "3. Encrypt(Data, MK)" --> Crypto
    Crypto --> Store
    Store -- "4. Upload Encrypted Blob" --> GitHub

    %% Get Flow
    Get -- "1. Get Repo Name" --> LocalConfig
    Get -- "2. Fetch Blob" --> GitHub
    Get -- "3. Unlock Master Key" --> MK_Flow
    MK_Flow -- "Returns Decrypted MK" --> Get
    Get -- "4. Decrypt(Blob, MK)" --> Crypto
    Crypto --> Get
    Get -- "5. Display Secret" --> User
```

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
   > **Note**: On first use, you will be prompted to set a **Master Password**. This password is used to encrypt your vault's Master Key. You must enter this password for every `store`, `get`, or `delete` operation.

4. **Auto-Generate a Secret**: If you don't provide a value, AxKeyStore will generate a secure random alphanumeric value (6-36 characters) for you.
   ```bash
   axkeystore store --key "my-api-key"
   ```
   You'll see the generated value and be asked to confirm before storing:
   ```
   🔑 Generated value: qOmH8qHQ3pnuASPrho662Mqd
      (Length: 24 characters)

   Do you want to use this generated value? (y/n):
   ```

5. **Retrieve a Secret**: Download and decrypt a key.
   ```bash
   axkeystore get "my-api-key"
   ```

6. **Store with Category**: Organize secrets in hierarchical categories.
   ```bash
   axkeystore store --key "aws-key" --value "AKIAIOSFODNN7EXAMPLE" --category "cloud/aws/production"
   ```
   > **Tip**: You can also auto-generate values with categories:
   > ```bash
   > axkeystore store --key "aws-key" --category "cloud/aws/production"
   > ```

7. **Retrieve from Category**: Retrieve a secret from a specific category.
   ```bash
   axkeystore get "aws-key" --category "cloud/aws/production"
   ```

8. **Delete a Secret**: Delete a stored key (with confirmation prompt).
   ```bash
   axkeystore delete "my-api-key"
   ```

9. **Delete from Category**: Delete a secret from a specific category.
   ```bash
   axkeystore delete "aws-key" --category "cloud/aws/production"
   ```

### Category Path Rules

- Categories can be nested using `/` separator (e.g., `api/production/internal`)
- Category segments can only contain alphanumeric characters, dashes (`-`), and underscores (`_`)
- Key names cannot contain path separators
- Categories are optional; keys can be stored without any category

## 📦 Installation

*(Instructions coming soon)*

## ⚙️ Setup
To use AxKeyStore, you need to register a GitHub OAuth application to get a Client ID:

1. Go to [GitHub Developer Settings > OAuth Apps](https://github.com/settings/developers).
2. Click **New OAuth App**.
3. Fill in the details:
   - **Application Name**: AxKeyStore (or your choice)
   - **Homepage URL**: `http://localhost`
   - **Callback URL**: `http://localhost`
4. Click **Register application**.
5. Copy the **Client ID** (e.g., `Iv1...`).
6. Update the `GITHUB_CLIENT_ID` constant in `src/auth.rs` with your new Client ID.

## 📄 License

[MIT License](LICENSE)
