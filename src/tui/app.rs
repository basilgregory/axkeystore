use anyhow::{Context, Result};
use std::collections::BTreeMap;
use crate::storage::Storage;
use crate::crypto::{CryptoHandler, EncryptedBlob};

pub enum InputMode {
    Normal,
    AddingCategory,
    AddingName,
    AddingValue,
    Processing,
    Error(String),
    SelectingProfile,
    EnteringPasswordForProfile,
}

pub struct App {
    pub storage: Storage,
    pub master_key: String,
    pub entries: BTreeMap<Option<String>, Vec<(String, String)>>,
    pub flat_entries: Vec<(Option<String>, String, String)>, // Category, Key, Decrypted Value
    pub selected_index: usize,
    pub input_mode: InputMode,
    pub category_input: String,
    pub name_input: String,
    pub value_input: String,
    pub profiles: Vec<String>,
    pub selected_profile_index: usize,
    pub target_profile: Option<String>,
    pub password_input: String,
}

impl App {
    pub async fn new(storage: Storage, master_key: String) -> Result<App> {
        let mut app = App {
            storage,
            master_key,
            entries: BTreeMap::new(),
            flat_entries: Vec::new(),
            selected_index: 0,
            input_mode: InputMode::Normal,
            category_input: String::new(),
            name_input: String::new(),
            value_input: String::new(),
            profiles: Vec::new(),
            selected_profile_index: 0,
            target_profile: None,
            password_input: String::new(),
        };
        app.load_keys().await?;
        Ok(app)
    }

    pub async fn load_keys(&mut self) -> Result<()> {
        let entries = self.storage.list_all_keys().await?;
        
        self.entries.clear();
        for entry in &entries {
            let encrypted: EncryptedBlob = serde_json::from_slice(&entry.data)
                .context("Failed to parse encrypted blob")?;
            if let Ok(decrypted) = CryptoHandler::decrypt(&encrypted, &self.master_key) {
                if let Ok(value) = String::from_utf8(decrypted) {
                    self.entries
                        .entry(entry.category.clone())
                        .or_default()
                        .push((entry.name.clone(), value));
                }
            }
        }

        self.flat_entries.clear();
        for (category, pairs) in &self.entries {
            for (name, value) in pairs {
                self.flat_entries.push((category.clone(), name.clone(), value.clone()));
            }
        }

        self.selected_index = 0;
        Ok(())
    }

    pub fn next(&mut self) {
        if !self.flat_entries.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.flat_entries.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.flat_entries.is_empty() {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = self.flat_entries.len() - 1;
            }
        }
    }

    pub fn start_add_key(&mut self) {
        self.category_input.clear();
        self.name_input.clear();
        self.value_input.clear();
        self.input_mode = InputMode::AddingCategory;
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn start_switch_profile(&mut self) {
        let mut profiles = vec!["default".to_string()];
        if let Ok(loaded) = crate::config::GlobalConfig::list_profiles() {
            profiles.extend(loaded);
        }
        
        self.profiles = profiles;
        self.selected_profile_index = 0;
        self.input_mode = InputMode::SelectingProfile;
    }

    pub fn next_profile(&mut self) {
        if !self.profiles.is_empty() {
            self.selected_profile_index = (self.selected_profile_index + 1) % self.profiles.len();
        }
    }

    pub fn previous_profile(&mut self) {
        if !self.profiles.is_empty() {
            if self.selected_profile_index > 0 {
                self.selected_profile_index -= 1;
            } else {
                self.selected_profile_index = self.profiles.len() - 1;
            }
        }
    }

    pub fn select_profile(&mut self) {
        if let Some(profile) = self.profiles.get(self.selected_profile_index) {
            let target = if profile == "default" { None } else { Some(profile.clone()) };
            self.target_profile = target;
            self.password_input.clear();
            self.input_mode = InputMode::EnteringPasswordForProfile;
        }
    }

    pub fn handle_password_char(&mut self, c: char) {
        self.password_input.push(c);
    }

    pub fn handle_password_backspace(&mut self) {
        self.password_input.pop();
    }

    pub fn handle_char(&mut self, c: char) {
        match self.input_mode {
            InputMode::AddingCategory => self.category_input.push(c),
            InputMode::AddingName => self.name_input.push(c),
            InputMode::AddingValue => self.value_input.push(c),
            _ => {}
        }
    }

    pub fn handle_backspace(&mut self) {
        match self.input_mode {
            InputMode::AddingCategory => { self.category_input.pop(); },
            InputMode::AddingName => { self.name_input.pop(); },
            InputMode::AddingValue => { self.value_input.pop(); },
            _ => {}
        }
    }

    pub fn handle_enter(&mut self) -> bool {
        match self.input_mode {
            InputMode::AddingCategory => {
                self.input_mode = InputMode::AddingName;
                false
            }
            InputMode::AddingName => {
                if !self.name_input.trim().is_empty() {
                    self.input_mode = InputMode::AddingValue;
                }
                false
            }
            InputMode::AddingValue => {
                if !self.value_input.trim().is_empty() {
                    self.input_mode = InputMode::Processing;
                    return true;
                }
                false
            }
            _ => false
        }
    }

    pub async fn save_new_key(&mut self) -> Result<()> {
        let category = if self.category_input.trim().is_empty() {
            None
        } else {
            Some(self.category_input.trim())
        };
        let key = self.name_input.trim();
        let value = self.value_input.trim();

        let encrypted = CryptoHandler::encrypt(value.as_bytes(), &self.master_key)?;
        let json_blob = serde_json::to_vec(&encrypted)?;

        match self.storage.save_blob(key, &json_blob, category).await {
            Ok(_) => {
                self.load_keys().await?;
                self.input_mode = InputMode::Normal;
            }
            Err(e) => {
                self.input_mode = InputMode::Error(format!("Failed to save: {}", e));
            }
        }
        Ok(())
    }

    pub async fn submit_profile_switch(&mut self) -> Result<()> {
        let profile = self.target_profile.clone();
        let password = self.password_input.clone();
        self.password_input.clear();
        self.input_mode = InputMode::Processing;

        // Fetch repo name
        let repo_name = match crate::config::Config::get_repo_name_with_profile(profile.as_deref(), &password) {
            Ok(name) => name,
            Err(e) => {
                self.input_mode = InputMode::Error(format!("Incorrect password or configuration missing: {}", e));
                return Ok(());
            }
        };

        // Create storage
        let storage = match Storage::new_with_profile(profile.as_deref(), &repo_name, &password).await {
            Ok(s) => s,
            Err(e) => {
                self.input_mode = InputMode::Error(format!("Failed to initialize storage: {}", e));
                return Ok(());
            }
        };

        // Fetch master key
        let master_key = match storage.get_master_key_blob().await {
            Ok(Some(data)) => {
                let encrypted: crate::crypto::EncryptedBlob = match serde_json::from_slice(&data) {
                    Ok(e) => e,
                    Err(_) => {
                        self.input_mode = InputMode::Error("Failed to parse master key blob".to_string());
                        return Ok(());
                    }
                };

                match crate::crypto::CryptoHandler::decrypt(&encrypted, &password) {
                    Ok(decrypted) => {
                        match String::from_utf8(decrypted) {
                            Ok(s) => s,
                            Err(_) => {
                                self.input_mode = InputMode::Error("Master key is not valid UTF-8".to_string());
                                return Ok(());
                            }
                        }
                    }
                    Err(_) => {
                        self.input_mode = InputMode::Error("Incorrect master password.".to_string());
                        return Ok(());
                    }
                }
            }
            Ok(None) => {
                // Initialize master key
                let mk = crate::crypto::CryptoHandler::generate_master_key();
                let encrypted = match crate::crypto::CryptoHandler::encrypt(mk.as_bytes(), &password) {
                    Ok(e) => e,
                    Err(e) => {
                        self.input_mode = InputMode::Error(format!("Encryption failed: {}", e));
                        return Ok(());
                    }
                };
                let json_blob = match serde_json::to_vec(&encrypted) {
                    Ok(b) => b,
                    Err(_) => {
                        self.input_mode = InputMode::Error("Failed to serialize".to_string());
                        return Ok(());
                    }
                };

                if let Err(e) = storage.save_master_key_blob(&json_blob).await {
                    self.input_mode = InputMode::Error(format!("Failed to save master key: {}", e));
                    return Ok(());
                }
                mk
            }
            Err(e) => {
                self.input_mode = InputMode::Error(format!("Failed to fetch master key: {}", e));
                return Ok(());
            }
        };

        self.storage = storage;
        self.master_key = master_key;
        if let Err(e) = self.load_keys().await {
            self.input_mode = InputMode::Error(format!("Failed to load keys: {}", e));
            return Ok(());
        }

        if let Err(e) = crate::config::GlobalConfig::set_active_profile(profile.clone()) {
            self.input_mode = InputMode::Error(format!("Failed to save active profile: {}", e));
            return Ok(());
        }

        self.input_mode = InputMode::Normal;
        Ok(())
    }
}
