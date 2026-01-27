use crate::auth::get_saved_token_with_profile;
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Internal response from GitHub user endpoint
#[derive(Debug, Deserialize)]
struct UserResponse {
    login: String,
}

/// Internal response from GitHub contents endpoint
#[derive(Debug, Deserialize)]
struct FileResponse {
    content: String,
    sha: String,
}

/// Request body for creating or updating a file on GitHub
#[derive(Serialize)]
struct UpdateFileRequest {
    message: String,
    content: String,
    sha: Option<String>,
}

/// Represents a specific version (commit) of a key
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct KeyVersion {
    /// Commit SHA
    pub sha: String,
    /// ISO 8601 date string
    pub date: String,
    /// Commit message
    pub message: String,
}

/// Internal struct to map GitHub commit list response
#[derive(Debug, Deserialize)]
struct GitHubCommit {
    sha: String,
    commit: GitHubCommitDetails,
}

/// Internal struct for GitHub commit details
#[derive(Debug, Deserialize)]
struct GitHubCommitDetails {
    author: GitHubAuthor,
    message: String,
}

/// Internal struct for GitHub commit author data
#[derive(Debug, Deserialize)]
struct GitHubAuthor {
    date: String,
}

/// Handles all interactions with the GitHub repository backend
pub struct Storage {
    client: Client,
    token: String,
    owner: String,
    repo: String,
    api_base: String,
}

impl Storage {
    /// Creates a new Storage instance for a specific profile
    pub async fn new_with_profile(
        profile: Option<&str>,
        repo: &str,
        password: &str,
    ) -> Result<Self> {
        let token = if let Ok(t) = std::env::var("AXKEYSTORE_TEST_TOKEN") {
            t
        } else {
            get_saved_token_with_profile(profile, password)?
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

    /// Ensures the storage repository exists on GitHub, creating it if it doesn't
    pub async fn init_repo(&self) -> Result<()> {
        println!(
            "Checking if repository {}/{} exists...",
            self.owner, self.repo
        );

        let url = format!("{}/repos/{}/{}", self.api_base, self.owner, self.repo);
        let res = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await?;

        if res.status() == reqwest::StatusCode::NOT_FOUND {
            println!("Repository not found. Creating private repository...");
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

    /// Validates and sanitizes a category path string
    fn validate_category(category: Option<&str>) -> Result<Option<String>> {
        match category {
            None => Ok(None),
            Some(cat) => {
                let cat = cat.trim().trim_matches('/');
                if cat.is_empty() {
                    return Ok(None);
                }

                // Validate each segment of the category path
                for segment in cat.split('/') {
                    let segment = segment.trim();
                    if segment.is_empty() {
                        return Err(anyhow::anyhow!("Category path contains empty segments"));
                    }
                    // Check for invalid characters (only allow alphanumeric, dash, underscore)
                    if !segment
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                    {
                        return Err(anyhow::anyhow!(
                            "Category segment '{}' contains invalid characters. Only alphanumeric, dash, and underscore are allowed.",
                            segment
                        ));
                    }
                    // Prevent path traversal
                    if segment == ".." || segment == "." {
                        return Err(anyhow::anyhow!("Category path cannot contain '.' or '..'"));
                    }
                }

                // Normalize the path (remove extra slashes, trim segments)
                let normalized: Vec<&str> = cat
                    .split('/')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();

                Ok(Some(normalized.join("/")))
            }
        }
    }

    /// Generates the GitHub file path for a specific key and category
    fn build_key_path(key: &str, category: Option<&str>) -> Result<String> {
        let validated_category = Self::validate_category(category)?;

        // Validate the key name
        if key.contains('/') || key.contains('\\') {
            return Err(anyhow::anyhow!(
                "Key name cannot contain path separators. Use --category for organizing keys."
            ));
        }

        let path = match validated_category {
            Some(cat) => format!("keys/{}/{}.json", cat, key),
            None => format!("keys/{}.json", key),
        };

        Ok(path)
    }

    /// Fetches the encrypted master key blob from the hidden application directory
    pub async fn get_master_key_blob(&self) -> Result<Option<Vec<u8>>> {
        let url = format!(
            "{}/repos/{}/{}/contents/.axkeystore/master_key.json",
            self.api_base, self.owner, self.repo
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
            return Err(anyhow::anyhow!(
                "Failed to fetch master key: {}",
                res.status()
            ));
        }

        let file_res: FileResponse = res.json().await?;
        let content_clean = file_res.content.replace('\n', "");
        let decoded = BASE64
            .decode(content_clean)
            .context("Failed to decode base64 master key from GitHub")?;

        Ok(Some(decoded))
    }

    /// Saves the encrypted master key blob to the repository
    pub async fn save_master_key_blob(&self, data: &[u8]) -> Result<()> {
        let url = format!(
            "{}/repos/{}/{}/contents/.axkeystore/master_key.json",
            self.api_base, self.owner, self.repo
        );

        // Check if file exists to get SHA
        let res = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await?;

        let sha = if res.status().is_success() {
            let file_res: FileResponse = res.json().await?;
            Some(file_res.sha)
        } else {
            None
        };

        let encoded_content = BASE64.encode(data);

        let body = UpdateFileRequest {
            message: "Initialize master key".to_string(),
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
            return Err(anyhow::anyhow!(
                "Failed to save master key: {} - {}",
                status,
                text
            ));
        }

        Ok(())
    }

    /// Fetches the current encrypted data and SHA for a specific key
    pub async fn get_blob(
        &self,
        key: &str,
        category: Option<&str>,
    ) -> Result<Option<(Vec<u8>, String)>> {
        let path = Self::build_key_path(key, category)?;
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

    /// Fetches the encrypted data for a key at a specific commit version
    pub async fn get_blob_at_version(
        &self,
        key: &str,
        category: Option<&str>,
        sha: &str,
    ) -> Result<Option<Vec<u8>>> {
        let path = Self::build_key_path(key, category)?;
        let url = format!(
            "{}/repos/{}/{}/contents/{}?ref={}",
            self.api_base, self.owner, self.repo, path, sha
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
            return Err(anyhow::anyhow!(
                "Failed to fetch key at version {}: {}",
                sha,
                res.status()
            ));
        }

        let file_res: FileResponse = res.json().await?;
        let content_clean = file_res.content.replace('\n', "");
        let decoded = BASE64
            .decode(content_clean)
            .context("Failed to decode base64 content from GitHub")?;

        Ok(Some(decoded))
    }

    /// Retrieves the list of versions (commits) for a specific key
    pub async fn get_key_history(
        &self,
        key: &str,
        category: Option<&str>,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<KeyVersion>> {
        let path = Self::build_key_path(key, category)?;
        let url = format!(
            "{}/repos/{}/{}/commits",
            self.api_base, self.owner, self.repo
        );

        let res = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .query(&[
                ("path", path.as_str()),
                ("page", &page.to_string()),
                ("per_page", &per_page.to_string()),
            ])
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch key history: {}",
                res.status()
            ));
        }

        let commits: Vec<GitHubCommit> = res.json().await?;
        let versions = commits
            .into_iter()
            .map(|c| KeyVersion {
                sha: c.sha,
                date: c.commit.author.date,
                message: c.commit.message,
            })
            .collect();

        Ok(versions)
    }

    /// Uploads or updates an encrypted key blob to the repository
    pub async fn save_blob(&self, key: &str, data: &[u8], category: Option<&str>) -> Result<()> {
        let path = Self::build_key_path(key, category)?;
        let url = format!(
            "{}/repos/{}/{}/contents/{}",
            self.api_base, self.owner, self.repo, path
        );

        // Check if file exists to get SHA (for update)
        let sha = if let Ok(Some((_, sha))) = self.get_blob(key, category).await {
            Some(sha)
        } else {
            None
        };

        let encoded_content = BASE64.encode(data);

        let commit_message = match category {
            Some(cat) => format!("Update key: {}/{}", cat.trim_matches('/'), key),
            None => format!("Update key: {}", key),
        };

        let body = UpdateFileRequest {
            message: commit_message,
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

    /// Deletes a key from the repository
    pub async fn delete_blob(&self, key: &str, category: Option<&str>) -> Result<bool> {
        let path = Self::build_key_path(key, category)?;

        // First, get the file to retrieve its SHA (required for deletion)
        let sha = match self.get_blob(key, category).await? {
            Some((_, sha)) => sha,
            None => return Ok(false), // Key doesn't exist
        };

        let url = format!(
            "{}/repos/{}/{}/contents/{}",
            self.api_base, self.owner, self.repo, path
        );

        let commit_message = match category {
            Some(cat) => format!("Delete key: {}/{}", cat.trim_matches('/'), key),
            None => format!("Delete key: {}", key),
        };

        let body = serde_json::json!({
            "message": commit_message,
            "sha": sha
        });

        let res = self
            .client
            .delete(&url)
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to delete key: {} - {}",
                status,
                text
            ));
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_storage_init_repo_exists() {
        let _lock = crate::config::TEST_MUTEX.lock().unwrap();
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

        let storage = Storage::new_with_profile(None, "test-repo", "test-pass")
            .await
            .unwrap();
        storage.init_repo().await.unwrap();

        std::env::remove_var("AXKEYSTORE_TEST_TOKEN");
        std::env::remove_var("AXKEYSTORE_API_URL");
    }

    #[tokio::test]
    async fn test_storage_create_repo() {
        let _lock = crate::config::TEST_MUTEX.lock().unwrap();
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

        let storage = Storage::new_with_profile(None, "new-repo", "test-pass")
            .await
            .unwrap();
        storage.init_repo().await.unwrap();
    }

    #[test]
    fn test_storage_validate_category() {
        assert_eq!(
            Storage::validate_category(Some("prod/api")).unwrap(),
            Some("prod/api".to_string())
        );
        assert_eq!(
            Storage::validate_category(Some("  stage/backend  ")).unwrap(),
            Some("stage/backend".to_string())
        );
        assert_eq!(
            Storage::validate_category(Some("/leading/slash/")).unwrap(),
            Some("leading/slash".to_string())
        );
        assert_eq!(Storage::validate_category(None).unwrap(), None);
        assert_eq!(Storage::validate_category(Some("")).unwrap(), None);

        // Errors
        assert!(Storage::validate_category(Some("invalid@char")).is_err());
        assert!(Storage::validate_category(Some("path/../traversal")).is_err());
        assert!(Storage::validate_category(Some("path//empty-segment")).is_err());
    }

    #[test]
    fn test_storage_build_key_path() {
        assert_eq!(
            Storage::build_key_path("my-key", None).unwrap(),
            "keys/my-key.json"
        );
        assert_eq!(
            Storage::build_key_path("my-key", Some("db/prod")).unwrap(),
            "keys/db/prod/my-key.json"
        );

        // Errors
        assert!(Storage::build_key_path("invalid/key", None).is_err());
    }

    #[tokio::test]
    async fn test_storage_get_key_history() {
        let _lock = crate::config::TEST_MUTEX.lock().unwrap();
        let mock_server = MockServer::start().await;
        std::env::set_var("AXKEYSTORE_TEST_TOKEN", "mock_token");
        std::env::set_var("AXKEYSTORE_API_URL", mock_server.uri());

        // Mock User
        Mock::given(method("GET"))
            .and(path("/user"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "login": "testuser" })),
            )
            .mount(&mock_server)
            .await;

        // Mock Commits
        Mock::given(method("GET"))
            .and(path("/repos/testuser/test-repo/commits"))
            .and(wiremock::matchers::query_param("path", "keys/my-key.json"))
            .and(wiremock::matchers::query_param("page", "1"))
            .and(wiremock::matchers::query_param("per_page", "10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "sha": "sha1",
                    "commit": {
                        "author": { "date": "2024-01-01T10:00:00Z" },
                        "message": "msg1"
                    }
                },
                {
                    "sha": "sha2",
                    "commit": {
                        "author": { "date": "2024-01-01T11:00:00Z" },
                        "message": "msg2"
                    }
                }
            ])))
            .mount(&mock_server)
            .await;

        let storage = Storage::new_with_profile(None, "test-repo", "test-pass")
            .await
            .unwrap();
        let history = storage
            .get_key_history("my-key", None, 1, 10)
            .await
            .unwrap();

        assert_eq!(history.len(), 2);
        assert_eq!(history[0].sha, "sha1");
        assert_eq!(history[1].sha, "sha2");
    }
}
