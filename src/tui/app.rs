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
}
