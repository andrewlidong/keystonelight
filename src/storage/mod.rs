use crate::storage::log::{LogEntry, LogFile};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::sync::{Arc, Mutex, RwLock};

const CACHE_PATH: &str = "cache.txt";
const DATA_PATH: &str = "data.txt";

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

    pub fn load_from_file(&self) -> io::Result<()> {
        if let Ok(file) = File::open(DATA_PATH) {
            let reader = BufReader::new(file);
            let mut data = self.cache.write().unwrap();

            for line in reader.lines().map_while(Result::ok) {
                if let Some((k, v)) = line.split_once('|') {
                    data.insert(k.to_string(), v.as_bytes().to_vec());
                }
            }
        }
        Ok(())
    }

    pub fn save_to_file(&self) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(DATA_PATH)?;

        let data = self.cache.read().unwrap();
        for (k, v) in data.iter() {
            writeln!(file, "{}|{}", k, String::from_utf8(v.clone()).unwrap())?;
        }
        Ok(())
    }

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
