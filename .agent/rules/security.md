# Security Rules

1. **Zero Trust Storage**:
   - ALL credentials (keys, passwords) must be encrypted *before* they leave the local machine.
   - The remote GitHub repository is treated as untrusted storage; it only holds encrypted blobs.

2. **Credential Handling**:
   - GitHub OAuth tokens must be stored securely on the local machine (e.g., using the OS keychain if possible, or a permissions-restricted file).
   - Never log raw secrets or tokens to stdout/stderr.

3. **Encryption Standards**:
   - Use authenticated encryption (AEAD).
   - Ensure proper random nonce generation.
   - User master password/key retrieval should be handled securely (e.g., prompted securely, not stored in plain text history).
