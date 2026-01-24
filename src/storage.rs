use crate::auth::get_saved_token;
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct UserResponse {
    login: String,
}

#[derive(Debug, Deserialize)]
struct FileResponse {
    content: String,
    sha: String,
}

#[derive(Serialize)]
struct UpdateFileRequest {
    message: String,
    content: String,
    sha: Option<String>,
}

pub struct Storage {
    client: Client,
    token: String,
    owner: String,
    repo: String,
}

impl Storage {
    pub async fn new(repo: &str) -> Result<Self> {
        let token = get_saved_token()?;
        let client = Client::builder().user_agent("axkeystore-cli").build()?;

        // Get current user to determine owner
        let user_res: UserResponse = client
            .get("https://api.github.com/user")
            .bearer_auth(&token)
            .send()
            .await?
            .json()
            .await
            .context("Failed to get user info. Check if token is valid.")?;

        Ok(Self {
            client,
            token,
            owner: user_res.login,
            repo: repo.to_string(),
        })
    }

    pub async fn init_repo(&self) -> Result<()> {
        println!(
            "Checking if repository {}/{} exists...",
            self.owner, self.repo
        );

        // precise logic to check repo existence could vary,
        // simple way: try to get it.
        let url = format!("https://api.github.com/repos/{}/{}", self.owner, self.repo);
        let res = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await?;

        if res.status() == reqwest::StatusCode::NOT_FOUND {
            println!("Repository not found. Creating private repository...");
            // Create repo
            // Endpoint: POST /user/repos
            let create_body = serde_json::json!({
                "name": self.repo,
                "private": true,
                "description": "Secure storage for AxKeyStore"
            });

            let create_res = self
                .client
                .post("https://api.github.com/user/repos")
                .bearer_auth(&self.token)
                .json(&create_body)
                .send()
                .await?;

            if !create_res.status().is_success() {
                return Err(anyhow::anyhow!(
                    "Failed to create repo: {}",
                    create_res.status()
                ));
            }
            println!("Repository created successfully.");
        } else if res.status().is_success() {
            println!("Repository exists.");
        } else {
            return Err(anyhow::anyhow!("Error checking repo: {}", res.status()));
        }

        Ok(())
    }

    pub async fn get_blob(&self, key: &str) -> Result<Option<(Vec<u8>, String)>> {
        // We store keys as files in the repo root or a subdirectory.
        // Let's store them in `keys/{key}.json`
        let path = format!("keys/{}.json", key);
        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/{}",
            self.owner, self.repo, path
        );

        let res = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await?;

        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !res.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch key: {}", res.status()));
        }

        let file_res: FileResponse = res.json().await?;
        // Github returns content as base64 with newlines
        let content_clean = file_res.content.replace('\n', "");
        let decoded = BASE64
            .decode(content_clean)
            .context("Failed to decode base64 content from GitHub")?;

        Ok(Some((decoded, file_res.sha)))
    }

    pub async fn save_blob(&self, key: &str, data: &[u8]) -> Result<()> {
        let path = format!("keys/{}.json", key);
        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/{}",
            self.owner, self.repo, path
        );

        // Check if file exists to get SHA (for update)
        let sha = if let Ok(Some((_, sha))) = self.get_blob(key).await {
            Some(sha)
        } else {
            None
        };

        let encoded_content = BASE64.encode(data);

        let body = UpdateFileRequest {
            message: format!("Update key: {}", key),
            content: encoded_content,
            sha,
        };

        let res = self
            .client
            .put(&url)
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Failed to save key: {} - {}", status, text));
        }

        Ok(())
    }
}
