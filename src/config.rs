use crate::crypto::{CryptoHandler, EncryptedBlob};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Local configuration for AxKeyStore
#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    /// Encrypted repository name where secrets are stored
    pub encrypted_repo_name: Option<EncryptedBlob>,
}

impl Config {
    /// Returns the absolute path to the configuration file
    fn get_config_path() -> Result<PathBuf> {
        if let Ok(test_dir) = std::env::var("AXKEYSTORE_TEST_CONFIG_DIR") {
            let path = PathBuf::from(test_dir);
            std::fs::create_dir_all(&path)?;
            return Ok(path.join("config.json"));
        }

        let project_dirs = directories::ProjectDirs::from("com", "ax", "axkeystore")
            .context("Could not determine user data directory")?;
        let config_dir = project_dirs.config_dir();
        std::fs::create_dir_all(config_dir)?;
        Ok(config_dir.join("config.json"))
    }

    /// Loads the configuration from the local filesystem
    pub fn load() -> Result<Self> {
        let path = Self::get_config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&content).unwrap_or_default();
        Ok(config)
    }

    /// Saves the current configuration to the local filesystem
    pub fn save(&self) -> Result<()> {
        let path = Self::get_config_path()?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Decrypts and retrieves the repository name from the local configuration
    pub fn get_repo_name(password: &str) -> Result<String> {
        let config = Self::load()?;
        match config.encrypted_repo_name {
            Some(blob) => {
                let decrypted = CryptoHandler::decrypt(&blob, password).map_err(|_| {
                    anyhow::anyhow!("Incorrect master password or corrupted local configuration.")
                })?;
                Ok(String::from_utf8(decrypted).context("Repo name is not valid UTF-8")?)
            }
            None => Err(anyhow::anyhow!(
                "Repository not configured. Please run 'axkeystore init' to set up your storage repository."
            )),
        }
    }

    /// Encrypts and saves the repository name to the local configuration
    pub fn set_repo_name(name: &str, password: &str) -> Result<()> {
        let mut config = Self::load()?;
        let encrypted = CryptoHandler::encrypt(name.as_bytes(), password)?;
        config.encrypted_repo_name = Some(encrypted);
        config.save()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_save_load() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        std::env::set_var("AXKEYSTORE_TEST_CONFIG_DIR", path);
        let password = "test-password";

        // 1. Load empty
        let config = Config::load().expect("Should load default");
        assert!(config.encrypted_repo_name.is_none());
        assert!(Config::get_repo_name(password).is_err());

        // 2. Set repo name
        Config::set_repo_name("my-new-repo", password).unwrap();

        // 3. Verify persistence and encryption
        let config2 = Config::load().unwrap();
        assert!(config2.encrypted_repo_name.is_some());
        assert_eq!(Config::get_repo_name(password).unwrap(), "my-new-repo");

        std::env::remove_var("AXKEYSTORE_TEST_CONFIG_DIR");
    }
}
