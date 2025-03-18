use crate::error::StoreError;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

/// Core storage engine for KeystoneLight
#[derive(Serialize, Deserialize, Debug)]
pub struct Store {
    data: HashMap<String, Value>,
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

impl Store {
    /// Creates a new empty store
    pub fn new() -> Self {
        Store {
            data: HashMap::new(),
        }
    }

    /// Loads a store from a file
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

    /// Saves the store to a file
    pub fn save(&self, path: &Path) -> Result<(), StoreError> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        let json = serde_json::to_string_pretty(self)?;
        write!(&file, "{}", json)?;
        Ok(())
    }

    /// Sets a value in the store
    pub fn set(&mut self, key: String, value: Value) {
        self.data.insert(key, value);
    }

    /// Gets a value from the store
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    /// Deletes a value from the store
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
