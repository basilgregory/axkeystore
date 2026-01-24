---
description: Implement GitHub OAuth Authentication Flow
---

# Implement Authentication Workflow

This workflow guides the implementation of the GitHub Device Flow.

## Prerequisites
- `src/auth.rs` exists.
- `reqwest`, `serde`, `serde_json` dependencies are added.
- A GitHub OAuth App Client ID is available (This might need to be hardcoded or provided by the user).

## Steps

1. **Define Structures**:
   - Create structs in `src/auth.rs` for:
     - `DeviceCodeResponse`: `device_code`, `user_code`, `verification_uri`, `interval`.
     - `AccessTokenResponse`: `access_token`, `token_type`, `scope`.

2. **Implement `request_device_code`**:
   - Create a function that POSTs to `https://github.com/login/device/code` with the Client ID.
   - Return the `DeviceCodeResponse`.

3. **Implement `poll_for_token`**:
   - Create a loop that POSTs to `https://github.com/login/oauth/access_token`.
   - Respect the `interval` from the previous step.
   - Handle errors: if response is "authorization_pending", continue looping. If "slow_down", increase interval.
   - Break loop and return token on success.

4. **Implement `save_token`**:
   - Use the `directories` crate to find the local data directory.
   - Save the token to a secure file (consider platform-specific credentials store if possible, otherwise a permission-locked file).

5. **Expose `login` command**:
   - In `src/main.rs`, wire the `login` subcommand to call these functions.
