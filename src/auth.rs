use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

use std::time::Duration;
use tokio::time::sleep;

const GITHUB_CLIENT_ID: &str = "Ov23limHTNOfaODLB0Jg"; // AxKeyStore Test Client ID

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AccessTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub scope: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PollResponse {
    Success(AccessTokenResponse),
    Error(AuthError),
}

#[derive(Debug, Deserialize)]
struct AuthError {
    error: String,
    error_description: String,
    #[serde(default)]
    interval: u64,
}

pub async fn authenticate() -> Result<String> {
    let client = Client::new();

    // 1. Request Device Code
    println!("Requesting device code...");
    let res = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .query(&[("client_id", GITHUB_CLIENT_ID), ("scope", "repo")])
        .send()
        .await?;

    let text = res.text().await?;
    // println!("Device code response: {}", text); // Debug

    // Try to parse success response
    let device_res: DeviceCodeResponse = match serde_json::from_str(&text) {
        Ok(res) => res,
        Err(_) => {
            // Try to parse as error Response
            #[derive(Deserialize, Debug)]
            struct GitHubErrorResponse {
                error: String,
                error_description: Option<String>,
            }

            if let Ok(err_res) = serde_json::from_str::<GitHubErrorResponse>(&text) {
                return Err(anyhow::anyhow!(
                    "GitHub API Error: {} - {}",
                    err_res.error,
                    err_res.error_description.unwrap_or_default()
                ));
            }

            // If neither, return the raw text for debugging
            return Err(anyhow::anyhow!("Failed to parse response: {}", text));
        }
    };

    println!("Please visit: {}", device_res.verification_uri);
    println!("And enter code: {}", device_res.user_code);

    // 2. Poll for Token
    let token = poll_for_token(&client, &device_res).await?;

    // 3. Save Token
    save_token(&token)?;

    Ok(token)
}

async fn poll_for_token(client: &Client, device_res: &DeviceCodeResponse) -> Result<String> {
    let mut interval = Duration::from_secs(device_res.interval + 1); // Add minimal buffer

    loop {
        sleep(interval).await;

        let res = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .query(&[
                ("client_id", GITHUB_CLIENT_ID),
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

fn save_token(token: &str) -> Result<()> {
    let project_dirs = directories::ProjectDirs::from("com", "appxiom", "axkeystore")
        .context("Could not determine user data directory")?;
    let config_dir = project_dirs.config_dir();

    std::fs::create_dir_all(config_dir)?;

    let token_path = config_dir.join("github_token");
    std::fs::write(&token_path, token)?;

    // Set file permissions to be readable only by user on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&token_path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&token_path, perms)?;
    }

    println!("Token saved to {:?}", token_path);
    Ok(())
}

pub fn get_saved_token() -> Result<String> {
    let project_dirs = directories::ProjectDirs::from("com", "appxiom", "axkeystore")
        .context("Could not determine user data directory")?;
    let token_path = project_dirs.config_dir().join("github_token");

    if !token_path.exists() {
        return Err(anyhow::anyhow!(
            "Not logged in. Please run 'axkeystore login' first."
        ));
    }

    let token = std::fs::read_to_string(token_path)?;
    Ok(token.trim().to_string())
}
