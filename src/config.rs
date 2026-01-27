use crate::crypto::{CryptoHandler, EncryptedBlob};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Local configuration for AxKeyStore (profile-specific)
#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    /// Encrypted repository name where secrets are stored
    pub encrypted_repo_name: Option<EncryptedBlob>,
}

/// Global settings across all profiles
#[derive(Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    /// The currently active profile name
    pub active_profile: Option<String>,
}

impl Config {
    /// Returns the absolute path to the base configuration directory
    fn get_base_dir() -> Result<PathBuf> {
        if let Ok(test_dir) = std::env::var("AXKEYSTORE_TEST_CONFIG_DIR") {
            let path = PathBuf::from(test_dir);
            std::fs::create_dir_all(&path)?;
            return Ok(path);
        }

        let project_dirs = directories::ProjectDirs::from("com", "ax", "axkeystore")
            .context("Could not determine user data directory")?;
        let config_dir = project_dirs.config_dir().to_path_buf();
        std::fs::create_dir_all(&config_dir)?;
        Ok(config_dir)
    }

    /// Returns the configuration directory for a specific profile (or default)
    pub fn get_config_dir(profile: Option<&str>) -> Result<PathBuf> {
        let base_dir = Self::get_base_dir()?;
        let dir = match profile {
            Some(p) => {
                Self::validate_profile_name(p)?;
                base_dir.join(p)
            }
            None => base_dir,
        };
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// Returns the path to the config.json for a specific profile
    fn get_config_path(profile: Option<&str>) -> Result<PathBuf> {
        Ok(Self::get_config_dir(profile)?.join("config.json"))
    }

    /// Validates that a profile name contains only alphabets, numbers, underscores, and dashes
    pub fn validate_profile_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(anyhow::anyhow!("Profile name cannot be empty"));
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(anyhow::anyhow!(
                "Profile name '{}' contains invalid characters. Only alphabets, numbers, '_' and '-' are allowed.",
                name
            ));
        }
        Ok(())
    }

    /// Loads the configuration for a specific profile
    pub fn load_with_profile(profile: Option<&str>) -> Result<Self> {
        let path = Self::get_config_path(profile)?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&content).unwrap_or_default();
        Ok(config)
    }

    /// Saves the current configuration to a specific profile
    pub fn save_with_profile(&self, profile: Option<&str>) -> Result<()> {
        let path = Self::get_config_path(profile)?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Decrypts and retrieves the repository name for a specific profile
    pub fn get_repo_name_with_profile(profile: Option<&str>, password: &str) -> Result<String> {
        let config = Self::load_with_profile(profile)?;
        match config.encrypted_repo_name {
            Some(blob) => {
                let decrypted = CryptoHandler::decrypt(&blob, password).map_err(|_| {
                    anyhow::anyhow!("Incorrect master password or corrupted local configuration.")
                })?;
                Ok(String::from_utf8(decrypted).context("Repo name is not valid UTF-8")?)
            }
            None => Err(anyhow::anyhow!(
                "Repository not configured for profile '{}'. Please run 'axkeystore init' to set up your storage repository.",
                profile.unwrap_or("default")
            )),
        }
    }

    /// Encrypts and saves the repository name for a specific profile
    pub fn set_repo_name_with_profile(
        profile: Option<&str>,
        name: &str,
        password: &str,
    ) -> Result<()> {
        let mut config = Self::load_with_profile(profile)?;
        let encrypted = CryptoHandler::encrypt(name.as_bytes(), password)?;
        config.encrypted_repo_name = Some(encrypted);
        config.save_with_profile(profile)?;
        Ok(())
    }
}

impl GlobalConfig {
    fn get_global_config_path() -> Result<PathBuf> {
        Ok(Config::get_base_dir()?.join("global.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::get_global_config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&content).unwrap_or_default();
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::get_global_config_path()?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn get_active_profile() -> Result<Option<String>> {
        let config = Self::load()?;
        Ok(config.active_profile)
    }

    pub fn set_active_profile(profile: Option<String>) -> Result<()> {
        if let Some(ref p) = profile {
            Config::validate_profile_name(p)?;
        }
        let mut config = Self::load()?;
        config.active_profile = profile;
        config.save()?;
        Ok(())
    }

    pub fn list_profiles() -> Result<Vec<String>> {
        let base_dir = Config::get_base_dir()?;
        let mut profiles = Vec::new();

        for entry in std::fs::read_dir(base_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if Config::validate_profile_name(name).is_ok() {
                        profiles.push(name.to_string());
                    }
                }
            }
        }
        profiles.sort();
        Ok(profiles)
    }

    pub fn delete_profile(name: &str) -> Result<()> {
        Config::validate_profile_name(name)?;
        let profile_dir = Config::get_base_dir()?.join(name);
        if profile_dir.exists() {
            std::fs::remove_dir_all(profile_dir)?;
        }

        // If we deleted the active profile, clear it
        if let Some(active) = Self::get_active_profile()? {
            if active == name {
                Self::set_active_profile(None)?;
            }
        }
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
        let config = Config::load_with_profile(None).expect("Should load default");
        assert!(config.encrypted_repo_name.is_none());
        assert!(Config::get_repo_name_with_profile(None, password).is_err());

        // 2. Set repo name
        Config::set_repo_name_with_profile(None, "my-new-repo", password).unwrap();

        // 3. Verify persistence and encryption
        let config2 = Config::load_with_profile(None).unwrap();
        assert!(config2.encrypted_repo_name.is_some());
        assert_eq!(
            Config::get_repo_name_with_profile(None, password).unwrap(),
            "my-new-repo"
        );

        std::env::remove_var("AXKEYSTORE_TEST_CONFIG_DIR");
    }

    #[test]
    fn test_config_update_repo_name() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_str().unwrap();
        std::env::set_var("AXKEYSTORE_TEST_CONFIG_DIR", path);
        let password = "test-password";

        Config::set_repo_name_with_profile(None, "repo-v1", password).unwrap();
        assert_eq!(
            Config::get_repo_name_with_profile(None, password).unwrap(),
            "repo-v1"
        );

        Config::set_repo_name_with_profile(None, "repo-v2", password).unwrap();
        assert_eq!(
            Config::get_repo_name_with_profile(None, password).unwrap(),
            "repo-v2"
        );

        std::env::remove_var("AXKEYSTORE_TEST_CONFIG_DIR");
    }

    #[test]
    fn test_config_wrong_password() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_str().unwrap();
        std::env::set_var("AXKEYSTORE_TEST_CONFIG_DIR", path);

        Config::set_repo_name_with_profile(None, "secret-repo", "password-a").unwrap();
        assert!(Config::get_repo_name_with_profile(None, "password-b").is_err());

        std::env::remove_var("AXKEYSTORE_TEST_CONFIG_DIR");
    }
}
