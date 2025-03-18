use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use serde::{Serialize, Deserialize};
use crate::error::StoreError;
use crate::value::Value;

/// Core storage engine for KeystoneLight
#[derive(Serialize, Deserialize, Debug)]
pub struct Store {
    data: HashMap<String, Value>,
}

impl Store {
    /// Creates a new empty KeystoneLight store
    pub fn new() -> Self {
        Store {
            data: HashMap::new(),
        }
    }

    /// Loads a KeystoneLight store from the specified path
    pub fn load(path: &Path) -> Result<Self, StoreError> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        if contents.is_empty() {
            return Ok(Store::new());
        }
        
        let store = serde_json::from_str(&contents)?;
        Ok(store)
    }

    /// Saves the store to the specified path
    pub fn save(&self, path: &Path) -> Result<(), StoreError> {
        let encoded = serde_json::to_string_pretty(self)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        file.write_all(encoded.as_bytes())?;
        Ok(())
    }

    /// Sets a value for the given key
    pub fn set(&mut self, key: String, value: Value) {
        self.data.insert(key, value);
    }

    /// Gets a value for the given key, supporting nested access with dot notation
    pub fn get(&self, key: &str) -> Option<&Value> {
        let parts: Vec<&str> = key.split('.').collect();
        let base_key = parts[0];
        let base_value = self.data.get(base_key)?;

        if parts.len() == 1 {
            Some(base_value)
        } else {
            base_value.get(&parts[1..].join("."))
        }
    }

    /// Deletes a value for the given key
    pub fn delete(&mut self, key: &str) -> Option<Value> {
        self.data.remove(key)
    }

    /// Returns true if the store is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns an iterator over the store's key-value pairs
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.data.iter()
    }
} 