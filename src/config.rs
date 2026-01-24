use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub repo_name: Option<String>,
}

impl Config {
    fn get_config_path() -> Result<PathBuf> {
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
