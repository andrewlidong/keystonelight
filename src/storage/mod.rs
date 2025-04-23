//! Storage module for the key-value database.
//!
//! This module provides persistent storage with an in-memory cache and log-based persistence.
//!
//! # Examples
//!
//! Basic usage:
//!
//! ```
//! use keystonelight::storage::Database;
//! use std::fs;
//! use std::path::Path;
//!
//! // Create a new database with a unique log file
//! let log_path = "test_db.log";
//! let db = Database::with_log_path(log_path).unwrap();
//!
//! // Set a key-value pair
//! db.set("key1", b"value1").unwrap();
//!
//! // Get the value
//! assert_eq!(db.get("key1").unwrap(), b"value1");
//!
//! // Delete the key
//! db.delete("key1").unwrap();
//! assert!(db.get("key1").is_none());
//!
//! // Clean up
//! fs::remove_file(log_path).unwrap_or(());
//! ```
//!
//! Binary data handling:
//!
//! ```
//! use keystonelight::storage::Database;
//! use std::fs;
//! use std::path::Path;
//!
//! let log_path = "test_db_binary.log";
//! let db = Database::with_log_path(log_path).unwrap();
//!
//! // Store binary data
//! let binary_data = vec![0, 1, 2, 3];
//! db.set("binary_key", &binary_data).unwrap();
//!
//! // Retrieve binary data
//! assert_eq!(db.get("binary_key").unwrap(), binary_data);
//!
//! // Clean up
//! fs::remove_file(log_path).unwrap_or(());
//! ```

use crate::storage::log::{LogEntry, LogFile};
use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

// Currently unused file paths
// const CACHE_PATH: &str = "cache.txt";
// const DATA_PATH: &str = "data.txt";

mod log;

/// A persistent key-value database with in-memory cache and log-based storage.
///
/// The database maintains an in-memory cache for fast access and a log file for persistence.
/// All operations are thread-safe and can be used concurrently.
///
/// # Examples
///
/// ```
/// use keystonelight::storage::Database;
/// use std::fs;
///
/// // Create a new database with default log file
/// let db = Database::new().unwrap();
///
/// // Basic operations
/// db.set("key1", b"value1").unwrap();
/// assert_eq!(db.get("key1").unwrap(), b"value1");
///
/// // Clean up
/// fs::remove_file("keystonelight.log").unwrap_or(());
/// ```
///
/// Using a custom log file path:
///
/// ```
/// use keystonelight::storage::Database;
/// use std::fs;
///
/// // Create a database with custom log file
/// let db = Database::with_log_path("custom.log").unwrap();
///
/// // Operations are persisted to the custom log file
/// db.set("key1", b"value1").unwrap();
/// assert_eq!(db.get("key1").unwrap(), b"value1");
///
/// // Clean up
/// fs::remove_file("custom.log").unwrap_or(());
/// ```
pub struct Database {
    log: Arc<Mutex<LogFile>>,
    cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl Database {
    /// Creates a new database with the default log file path.
    ///
    /// # Examples
    ///
    /// ```
    /// use keystonelight::storage::Database;
    /// use std::fs;
    ///
    /// let db = Database::new().unwrap();
    /// db.set("test_key", b"test_value").unwrap();
    ///
    /// // Clean up
    /// fs::remove_file("keystonelight.log").unwrap_or(());
    /// ```
    pub fn new() -> io::Result<Self> {
        Self::with_log_path("keystonelight.log")
    }

    /// Creates a new database with a custom log file path.
    ///
    /// # Examples
    ///
    /// ```
    /// use keystonelight::storage::Database;
    /// use std::fs;
    ///
    /// let db = Database::with_log_path("custom.log").unwrap();
    /// db.set("test_key", b"test_value").unwrap();
    ///
    /// // Clean up
    /// fs::remove_file("custom.log").unwrap_or(());
    /// ```
    pub fn with_log_path<P: AsRef<Path>>(log_path: P) -> io::Result<Self> {
        let mut log = LogFile::with_path(log_path)?;
        let cache = Arc::new(RwLock::new(HashMap::new()));

        // Replay the log to build the cache
        let entries = log.replay()?;
        {
            let mut cache = cache.write().unwrap();
            for entry in entries {
                match entry {
                    LogEntry::Set(key, value) => {
                        cache.insert(key, value);
                    }
                    LogEntry::Delete(key) => {
                        cache.remove(&key);
                    }
                    LogEntry::Compact => {
                        // Skip compact entries when replaying
                        continue;
                    }
                }
            }
        }

        Ok(Self {
            log: Arc::new(Mutex::new(log)),
            cache,
        })
    }

    // Currently unused file operations
    /*
    pub fn load_from_file(&self) -> io::Result<()> {
        let mut storage = self.storage.write().unwrap();
        let file = File::open(DATA_PATH)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() == 2 {
                storage.insert(parts[0].to_string(), parts[1].as_bytes().to_vec());
            }
        }
        Ok(())
    }

    pub fn save_to_file(&self) -> io::Result<()> {
        let storage = self.storage.read().unwrap();
        let mut file = File::create(DATA_PATH)?;

        for (key, value) in storage.iter() {
            writeln!(file, "{} {}", key, String::from_utf8_lossy(value))?;
        }
        Ok(())
    }
    */

    /// Retrieves a value from the database.
    ///
    /// # Examples
    ///
    /// ```
    /// use keystonelight::storage::Database;
    /// use std::fs;
    ///
    /// let db = Database::new().unwrap();
    ///
    /// // Get non-existent key
    /// assert!(db.get("missing").is_none());
    ///
    /// // Get existing key
    /// db.set("key1", b"value1").unwrap();
    /// assert_eq!(db.get("key1").unwrap(), b"value1");
    ///
    /// // Clean up
    /// fs::remove_file("keystonelight.log").unwrap_or(());
    /// ```
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.cache.read().unwrap().get(key).cloned()
    }

    /// Sets a key-value pair in the database.
    ///
    /// # Examples
    ///
    /// ```
    /// use keystonelight::storage::Database;
    /// use std::fs;
    ///
    /// let db = Database::new().unwrap();
    ///
    /// // Set and verify a value
    /// db.set("key1", b"value1").unwrap();
    /// assert_eq!(db.get("key1").unwrap(), b"value1");
    ///
    /// // Update existing value
    /// db.set("key1", b"new_value").unwrap();
    /// assert_eq!(db.get("key1").unwrap(), b"new_value");
    ///
    /// // Clean up
    /// fs::remove_file("keystonelight.log").unwrap_or(());
    /// ```
    pub fn set(&self, key: &str, value: &[u8]) -> io::Result<()> {
        let mut cache = self.cache.write().unwrap();
        let value = value.to_vec();
        cache.insert(key.to_string(), value.clone());
        let mut log = self.log.lock().unwrap();
        log.append(&LogEntry::Set(key.to_string(), value))?;
        Ok(())
    }

    /// Deletes a key-value pair from the database.
    ///
    /// # Examples
    ///
    /// ```
    /// use keystonelight::storage::Database;
    /// use std::fs;
    ///
    /// // Create a new database with a unique log file
    /// let log_path = "test_delete.log";
    /// let db = Database::with_log_path(log_path).unwrap();
    ///
    /// // Delete non-existent key
    /// db.delete("missing").unwrap();
    ///
    /// // Delete existing key
    /// db.set("key1", b"value1").unwrap();
    /// db.delete("key1").unwrap();
    /// assert!(db.get("key1").is_none());
    ///
    /// // Clean up
    /// fs::remove_file(log_path).unwrap_or(());
    /// ```
    pub fn delete(&self, key: &str) -> io::Result<()> {
        let mut cache = self.cache.write().unwrap();
        if cache.remove(key).is_some() {
            let mut log = self.log.lock().unwrap();
            log.append(&LogEntry::Delete(key.to_string()))?;
        }
        Ok(())
    }

    /// Compacts the log file by removing redundant entries.
    ///
    /// # Examples
    ///
    /// ```
    /// use keystonelight::storage::Database;
    /// use std::fs;
    ///
    /// let db = Database::new().unwrap();
    ///
    /// // Create some data
    /// db.set("key1", b"value1").unwrap();
    /// db.set("key2", b"value2").unwrap();
    /// db.delete("key1").unwrap();
    ///
    /// // Compact the log
    /// db.compact().unwrap();
    ///
    /// // Verify data is still intact
    /// assert!(db.get("key1").is_none());
    /// assert_eq!(db.get("key2").unwrap(), b"value2");
    ///
    /// // Clean up
    /// fs::remove_file("keystonelight.log").unwrap_or(());
    /// ```
    pub fn compact(&self) -> io::Result<()> {
        let mut log = self.log.lock().unwrap();
        log.compact()?;
        Ok(())
    }
}
