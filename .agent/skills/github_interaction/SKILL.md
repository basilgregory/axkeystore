---
name: GitHub Interaction Skill
description: Instructions and best practices for interacting with the GitHub API for AxKeyStore.
---

# GitHub Interaction Skill

This skill outlines how to handle interactions with GitHub, including authentication and repository manipulation.

## Authentication (Device Flow)
Since AxKeyStore is a CLI tool, use the GitHub **Device Authorization Flow**.
1. **Request Device Code**: POST `https://github.com/login/device/code` with `client_id`.
2. **User Prompt**: Display `user_code` and the `verification_uri` to the user.
3. **Poll for Token**: Poll `https://github.com/login/oauth/access_token` until the user authorizes.
4. **Storage**: Save the resulting `access_token` locally.

## Repository Operations
Use the GitHub REST API (v3) to read and write files.

### Reading a File
- Endpoint: `GET /repos/{owner}/{repo}/contents/{path}`
- Headers: `Authorization: token OAUTH_TOKEN`, `Accept: application/vnd.github.v3+json`
- Response: contains `content` (base64 encoded) and `sha`.

### Writing/Updating a File
- Endpoint: `PUT /repos/{owner}/{repo}/contents/{path}`
- Body:
  ```json
  {
    "message": "Update keys",
    "content": "BASE64_ENCODED_CONTENT",
    "sha": "FILE_SHA" // Required if updating, omit if creating new
  }
  ```
- **Note**: Always fetch the latest `sha` before updating to avoid conflicts.

## Crates
- Recommend using `reqwest` for raw control or `octocrab` if interaction is complex.
- Ensure `serde_json` is used for parsing responses.
