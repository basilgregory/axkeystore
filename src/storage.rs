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
    api_base: String,
}

impl Storage {
    pub async fn new(repo: &str) -> Result<Self> {
        let token = if let Ok(t) = std::env::var("AXKEYSTORE_TEST_TOKEN") {
            t
        } else {
            get_saved_token()?
        };

        let api_base = std::env::var("AXKEYSTORE_API_URL")
            .unwrap_or_else(|_| "https://api.github.com".to_string());

        let client = Client::builder().user_agent("axkeystore-cli").build()?;

        // Get current user to determine owner
        let user_res: UserResponse = client
            .get(format!("{}/user", api_base))
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
            api_base,
        })
    }

    pub async fn init_repo(&self) -> Result<()> {
        println!(
            "Checking if repository {}/{} exists...",
            self.owner, self.repo
        );

        // precise logic to check repo existence could vary,
        // simple way: try to get it.
        let url = format!("{}/repos/{}/{}", self.api_base, self.owner, self.repo);
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
                .post(format!("{}/user/repos", self.api_base))
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
            "{}/repos/{}/{}/contents/{}",
            self.api_base, self.owner, self.repo, path
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
            "{}/repos/{}/{}/contents/{}",
            self.api_base, self.owner, self.repo, path
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

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_storage_init_repo_exists() {
        let mock_server = MockServer::start().await;

        std::env::set_var("AXKEYSTORE_TEST_TOKEN", "mock_token");
        std::env::set_var("AXKEYSTORE_API_URL", mock_server.uri());

        // 1. Mock User endpoint
        Mock::given(method("GET"))
            .and(path("/user"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "login": "testuser"
            })))
            .mount(&mock_server)
            .await;

        // 2. Mock Repo Check (Existing)
        Mock::given(method("GET"))
            .and(path("/repos/testuser/test-repo"))
            .respond_with(ResponseTemplate::new(200)) // 200 OK means exists
            .mount(&mock_server)
            .await;

        let storage = Storage::new("test-repo").await.unwrap();
        storage.init_repo().await.unwrap();

        std::env::remove_var("AXKEYSTORE_TEST_TOKEN");
        std::env::remove_var("AXKEYSTORE_API_URL");
    }

    #[tokio::test]
    async fn test_storage_create_repo() {
        let mock_server = MockServer::start().await;

        std::env::set_var("AXKEYSTORE_TEST_TOKEN", "mock_token");
        std::env::set_var("AXKEYSTORE_API_URL", mock_server.uri());

        // User
        Mock::given(method("GET"))
            .and(path("/user"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "login": "testuser" })),
            )
            .mount(&mock_server)
            .await;

        // Check (Not Found)
        Mock::given(method("GET"))
            .and(path("/repos/testuser/new-repo"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        // Create (Success)
        Mock::given(method("POST"))
            .and(path("/user/repos"))
            .respond_with(ResponseTemplate::new(201))
            .mount(&mock_server)
            .await;

        let storage = Storage::new("new-repo").await.unwrap();
        storage.init_repo().await.unwrap();
    }
}
