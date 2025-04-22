use crate::storage::log::{LogEntry, LogFile};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::sync::{Arc, Mutex, RwLock};

// Currently unused file paths
// const CACHE_PATH: &str = "cache.txt";
// const DATA_PATH: &str = "data.txt";

mod log;

pub struct Database {
    log: Arc<Mutex<LogFile>>,
    cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl Database {
    pub fn new() -> io::Result<Self> {
        let mut log = LogFile::new()?;
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

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.cache.read().unwrap().get(key).cloned()
    }

    pub fn set(&self, key: &str, value: &[u8]) -> io::Result<()> {
        let mut cache = self.cache.write().unwrap();
        let value = value.to_vec();
        cache.insert(key.to_string(), value.clone());
        let mut log = self.log.lock().unwrap();
        log.append(&LogEntry::Set(key.to_string(), value))?;
        Ok(())
    }

    pub fn delete(&self, key: &str) -> io::Result<()> {
        let mut cache = self.cache.write().unwrap();
        if cache.remove(key).is_some() {
            let mut log = self.log.lock().unwrap();
            log.append(&LogEntry::Delete(key.to_string()))?;
        }
        Ok(())
    }

    pub fn compact(&self) -> io::Result<()> {
        let mut log = self.log.lock().unwrap();
        log.compact()?;
        Ok(())
    }
}
