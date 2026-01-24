use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub repo_name: Option<String>,
}

impl Config {
    fn get_config_path() -> Result<PathBuf> {
        if let Ok(test_dir) = std::env::var("AXKEYSTORE_TEST_CONFIG_DIR") {
            let path = PathBuf::from(test_dir);
            std::fs::create_dir_all(&path)?;
            return Ok(path.join("config.json"));
        }

        let project_dirs = directories::ProjectDirs::from("com", "appxiom", "axkeystore")
            .context("Could not determine user data directory")?;
        let config_dir = project_dirs.config_dir();
        std::fs::create_dir_all(config_dir)?;
        Ok(config_dir.join("config.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::get_config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&content).unwrap_or_default();
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::get_config_path()?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn get_repo_name() -> Result<String> {
        let config = Self::load()?;
        // Default to 'axkeystore-storage' if not set
        Ok(config
            .repo_name
            .unwrap_or_else(|| "axkeystore-storage".to_string()))
    }

    pub fn set_repo_name(name: &str) -> Result<()> {
        let mut config = Self::load()?;
        config.repo_name = Some(name.to_string());
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

        // Isolate environment for this test (though note: env vars are global)
        // We assume tests run sequentially or we use a unique var if possible,
        // but for now this is the simplest way without major refactor.
        // Rust's default test harness runs tests in threads, so this is risky for parallel tests.
        // But we only have one test file modifying this right now.
        std::env::set_var("AXKEYSTORE_TEST_CONFIG_DIR", path);

        // 1. Load empty
        let config = Config::load().expect("Should load default");
        assert!(config.repo_name.is_none());
        assert_eq!(Config::get_repo_name().unwrap(), "axkeystore-storage");

        // 2. Set repo name
        Config::set_repo_name("my-new-repo").unwrap();

        // 3. Verify persistence
        let config2 = Config::load().unwrap();
        assert_eq!(config2.repo_name.as_deref(), Some("my-new-repo"));
        assert_eq!(Config::get_repo_name().unwrap(), "my-new-repo");

        std::env::remove_var("AXKEYSTORE_TEST_CONFIG_DIR");
    }
}
