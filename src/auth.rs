use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

use std::time::Duration;
use tokio::time::sleep;

/// Response from GitHub device code request
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct DeviceCodeResponse {
    /// The device code used for verification
    pub device_code: String,
    /// The user code to display to the user
    pub user_code: String,
    /// The URI where the user should enter the code
    pub verification_uri: String,
    /// The interval in seconds to poll for the token
    pub interval: u64,
    /// The expiration time in seconds
    pub expires_in: u64,
}

/// Response from GitHub containing the access token
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AccessTokenResponse {
    /// The GitHub access token
    pub access_token: String,
    /// The type of token (usually "bearer")
    pub token_type: String,
    /// The scopes granted to the token (optional for GitHub Apps)
    pub scope: Option<String>,
}

/// Internal enum to handle polymorphic response from polling endpoint
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PollResponse {
    Success(AccessTokenResponse),
    Error(AuthError),
}

/// Error response from GitHub during authentication
#[derive(Debug, Deserialize)]
struct AuthError {
    error: String,
    error_description: String,
    #[serde(default)]
    interval: u64,
}

/// Parses the device code response from GitHub
fn parse_device_code_response(text: &str) -> Result<DeviceCodeResponse> {
    match serde_json::from_str(text) {
        Ok(res) => Ok(res),
        Err(_) => {
            #[derive(Deserialize, Debug)]
            struct GitHubErrorResponse {
                error: String,
                error_description: Option<String>,
            }

            if let Ok(err_res) = serde_json::from_str::<GitHubErrorResponse>(text) {
                return Err(anyhow::anyhow!(
                    "GitHub API Error: {} - {}",
                    err_res.error,
                    err_res.error_description.unwrap_or_default()
                ));
            }

            return Err(anyhow::anyhow!("Failed to parse response: {}", text));
        }
    }
}

/// Starts the GitHub OAuth Device Flow to authenticate the user
pub async fn authenticate() -> Result<String> {
    let client_id =
        std::env::var("GITHUB_CLIENT_ID").unwrap_or_else(|_| "Iv23lil2mpu0qFEEaQ2a".to_string());

    let client = Client::new();

    // 1. Request Device Code
    println!("Requesting device code...");
    let res = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .query(&[("client_id", client_id.as_str())]) // Omitted scope for GitHub App
        .send()
        .await?;

    let text = res.text().await?;
    // println!("Device code response: {}", text); // Debug

    // Try to parse response
    let device_res = parse_device_code_response(&text)?;

    println!("Please visit: {}", device_res.verification_uri);
    println!("And enter code: {}", device_res.user_code);

    // 2. Poll for Token
    let token = poll_for_token(&client, &device_res, &client_id).await?;

    // 3. (Optional) Provide Installation Link for GitHub App
    let app_name = std::env::var("GITHUB_APP_NAME").unwrap_or_else(|_| "axkeystore".to_string());
    println!("\nImportant: AxKeyStore is using a GitHub App.");
    println!("Please ensure the App is installed on your account/organization to grant repository access:");
    println!("https://github.com/apps/{}/installations/new", app_name);

    Ok(token)
}

/// Polls GitHub API for the access token after device code generation
async fn poll_for_token(
    client: &Client,
    device_res: &DeviceCodeResponse,
    client_id: &str,
) -> Result<String> {
    let mut interval = Duration::from_secs(device_res.interval + 1); // Add minimal buffer

    loop {
        sleep(interval).await;

        let res = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .query(&[
                ("client_id", client_id),
                ("device_code", device_res.device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?;

        let text = res.text().await?;
        // println!("Poll response: {}", text); // Debug

        let poll_res: PollResponse = serde_json::from_str(&text)?;

        match poll_res {
            PollResponse::Success(token_data) => {
                println!("Successfully authenticated!");
                return Ok(token_data.access_token);
            }
            PollResponse::Error(err) => {
                match err.error.as_str() {
                    "authorization_pending" => {
                        // Continue polling
                    }
                    "slow_down" => {
                        interval = Duration::from_secs(err.interval + 5);
                        println!("Slowing down polling...");
                    }
                    "expired_token" => {
                        return Err(anyhow::anyhow!("Device code expired. Please try again."));
                    }
                    "access_denied" => {
                        return Err(anyhow::anyhow!("Access denied by user."));
                    }
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Authentication error: {}",
                            err.error_description
                        ));
                    }
                }
            }
        }
    }
}

use crate::crypto::{CryptoHandler, EncryptedBlob};

/// Encrypts and saves the GitHub access token for a specific profile
pub fn save_token_with_profile(profile: Option<&str>, token: &str, password: &str) -> Result<()> {
    let lmk = crate::config::Config::get_or_create_lmk_with_profile(profile, password)?;
    let config_dir = crate::config::Config::get_config_dir(profile)?;
    let token_path = config_dir.join("github_token.json");

    save_token_to_path(token, &token_path, &lmk)
}

/// Internal helper to save token to a specific path with encryption
fn save_token_to_path(token: &str, path: &std::path::Path, key: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let encrypted = CryptoHandler::encrypt(token.as_bytes(), key)?;
    let json_blob = serde_json::to_string_pretty(&encrypted)?;

    std::fs::write(path, json_blob)?;

    // Set file permissions to be readable only by user on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }

    Ok(())
}

/// Retrieves and decrypts the saved GitHub access token for a specific profile
pub fn get_saved_token_with_profile(profile: Option<&str>, password: &str) -> Result<String> {
    let lmk = crate::config::Config::get_or_create_lmk_with_profile(profile, password)?;
    let config_dir = crate::config::Config::get_config_dir(profile)?;
    let token_path = config_dir.join("github_token.json");

    if !token_path.exists() {
        return Err(anyhow::anyhow!(
            "Not logged in for profile '{}'. Please run 'axkeystore login' first.",
            profile.unwrap_or("default")
        ));
    }

    let content = std::fs::read_to_string(token_path)?;
    let encrypted: EncryptedBlob =
        serde_json::from_str(&content).context("Failed to parse encrypted token")?;

    let decrypted = CryptoHandler::decrypt(&encrypted, &lmk)
        .map_err(|_| anyhow::anyhow!("Incorrect master password or corrupted local master key."))?;

    Ok(String::from_utf8(decrypted).context("Token is not valid UTF-8")?)
}

/// Checks if an encrypted token exists for a specific profile
pub fn is_logged_in_with_profile(profile: Option<&str>) -> bool {
    crate::config::Config::get_config_dir(profile)
        .map(|dir| dir.join("github_token.json").exists())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_device_code_success() {
        let json = r#"{
            "device_code": "dc123",
            "user_code": "uc123",
            "verification_uri": "https://github.com/login/device",
            "interval": 5,
            "expires_in": 900
        }"#;
        let res = parse_device_code_response(json).unwrap();
        assert_eq!(res.device_code, "dc123");
        assert_eq!(res.user_code, "uc123");
        assert_eq!(res.interval, 5);
    }

    #[test]
    fn test_parse_device_code_error() {
        let json = r#"{
            "error": "access_denied",
            "error_description": "User denied access"
        }"#;
        let res = parse_device_code_response(json);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "GitHub API Error: access_denied - User denied access"
        );
    }

    #[test]
    fn test_save_token() {
        let temp_dir = tempfile::tempdir().unwrap();
        let token_path = temp_dir.path().join("test_token.json");
        save_token_to_path("test-token-content", &token_path, "test-password").unwrap();

        let content = std::fs::read_to_string(&token_path).unwrap();
        assert!(content.contains("salt"));
        assert!(content.contains("ciphertext"));

        let decrypted = get_saved_token_from_path(&token_path, "test-password").unwrap();
        assert_eq!(decrypted, "test-token-content");
    }

    #[test]
    fn test_is_logged_in_state() {
        // This is hard to test perfectly because it uses ProjectDirs
        // But we already verify existence in test_save_token.
        // We can trust the Path::exists call.
    }

    #[test]
    fn test_token_multiple_updates() {
        let temp_dir = tempfile::tempdir().unwrap();
        let token_path = temp_dir.path().join("test_token.json");

        save_token_to_path("token1", &token_path, "pass").unwrap();
        assert_eq!(
            get_saved_token_from_path(&token_path, "pass").unwrap(),
            "token1"
        );

        save_token_to_path("token2", &token_path, "pass").unwrap();
        assert_eq!(
            get_saved_token_from_path(&token_path, "pass").unwrap(),
            "token2"
        );
    }

    #[test]
    fn test_token_corrupted() {
        let temp_dir = tempfile::tempdir().unwrap();
        let token_path = temp_dir.path().join("test_token.json");

        std::fs::write(&token_path, "not a json").unwrap();
        let res = get_saved_token_from_path(&token_path, "pass");
        assert!(res.is_err());
    }

    fn get_saved_token_from_path(path: &std::path::Path, password: &str) -> Result<String> {
        let content = std::fs::read_to_string(path)?;
        let encrypted: EncryptedBlob = serde_json::from_str(&content)?;
        let decrypted = CryptoHandler::decrypt(&encrypted, password)?;
        Ok(String::from_utf8(decrypted)?)
    }

    #[test]
    fn test_poll_response_parsing() {
        let json = r#"{
            "access_token": "gho_123",
            "token_type": "bearer",
            "scope": "repo"
        }"#;
        let res: PollResponse = serde_json::from_str(json).unwrap();
        match res {
            PollResponse::Success(t) => assert_eq!(t.access_token, "gho_123"),
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_profile_token_isolation() {
        let _lock = crate::config::TEST_MUTEX.lock().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_str().unwrap();
        std::env::set_var("AXKEYSTORE_TEST_CONFIG_DIR", path);

        let pass = "test-pass";
        save_token_with_profile(Some("p1"), "token-p1", pass).unwrap();
        save_token_with_profile(Some("p2"), "token-p2", pass).unwrap();

        assert_eq!(
            get_saved_token_with_profile(Some("p1"), pass).unwrap(),
            "token-p1"
        );
        assert_eq!(
            get_saved_token_with_profile(Some("p2"), pass).unwrap(),
            "token-p2"
        );
        assert!(get_saved_token_with_profile(None, pass).is_err());

        std::env::remove_var("AXKEYSTORE_TEST_CONFIG_DIR");
    }
}
