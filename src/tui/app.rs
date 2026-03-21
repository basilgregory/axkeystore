use anyhow::{Context, Result};
use std::collections::BTreeMap;
use crate::storage::Storage;
use crate::crypto::{CryptoHandler, EncryptedBlob};

pub struct App {
    pub storage: Storage,
    pub master_key: String,
    pub entries: BTreeMap<Option<String>, Vec<(String, String)>>,
    pub flat_entries: Vec<(Option<String>, String, String)>, // Category, Key, Decrypted Value
    pub selected_index: usize,
}

impl App {
    pub async fn new(storage: Storage, master_key: String) -> Result<App> {
        let mut app = App {
            storage,
            master_key,
            entries: BTreeMap::new(),
            flat_entries: Vec::new(),
            selected_index: 0,
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
}
