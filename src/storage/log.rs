use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use fs2::FileExt;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Seek, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

const MAX_LOG_SIZE: usize = 1024 * 1024; // 1MB

#[derive(Debug, Clone)]
pub enum LogEntry {
    Set(String, Vec<u8>),
    Delete(String),
    Compact,
}

impl LogEntry {
    pub fn to_string(&self) -> String {
        match self {
            LogEntry::Set(key, value) => {
                match String::from_utf8(value.clone()) {
                    Ok(text) => format!("SET {} {}", key, text),
                    Err(_) => {
                        // Only use base64 for binary data
                        let encoded_value = BASE64.encode(value);
                        format!("SET {} base64:{}", key, encoded_value)
                    }
                }
            }
            LogEntry::Delete(key) => format!("DELETE {}", key),
            LogEntry::Compact => "COMPACT".to_string(),
        }
    }

    pub fn from_string(line: &str) -> Option<LogEntry> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        let mut parts = line.splitn(3, ' ');
        match parts.next() {
            Some("SET") => {
                let key = parts.next()?;
                let value = parts.next()?;
                if value.starts_with("base64:") {
                    // Handle base64-encoded binary data
                    let encoded = &value[7..]; // Skip "base64:" prefix
                    let decoded_value = BASE64.decode(encoded).ok()?;
                    Some(LogEntry::Set(key.to_string(), decoded_value))
                } else {
                    // Handle plain text
                    Some(LogEntry::Set(key.to_string(), value.as_bytes().to_vec()))
                }
            }
            Some("DELETE") => {
                let key = parts.next()?;
                Some(LogEntry::Delete(key.to_string()))
            }
            Some("COMPACT") => Some(LogEntry::Compact),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct LogFile {
    file: File,
    current_size: usize,
    path: PathBuf,
}

impl LogFile {
    pub fn with_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        println!("Creating new log file at {}", path.display());
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .mode(0o600)
            .open(&path)?;

        // Try to acquire an exclusive lock on the file
        if let Err(e) = file.try_lock_exclusive() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("Another server instance is already running: {}", e),
            ));
        }

        // Get current file size
        let current_size = file.metadata()?.len() as usize;

        file.sync_all()?;
        println!("Log file opened and locked successfully");
        Ok(Self {
            file,
            current_size,
            path,
        })
    }

    pub fn append(&mut self, entry: &LogEntry) -> io::Result<()> {
        let entry_str = entry.to_string();
        println!("Appending log entry: {}", entry_str.trim());
        self.file.write_all(entry_str.as_bytes())?;
        self.file.write_all(b"\n")?;
        self.current_size += entry_str.len() + 1;
        self.file.sync_all()?; // Ensure data is written to disk
        println!("Log entry appended and synced");

        // Check if we need to compact
        if self.current_size > MAX_LOG_SIZE {
            println!(
                "Log size ({}) exceeds maximum size ({}), triggering compaction",
                self.current_size, MAX_LOG_SIZE
            );
            self.compact()?;
            // Update current size after compaction
            self.current_size = self.file.metadata()?.len() as usize;
            println!("Log compaction completed. New size: {}", self.current_size);
        }

        Ok(())
    }

    pub fn replay(&mut self) -> io::Result<Vec<LogEntry>> {
        println!("Replaying log file");
        let mut entries = Vec::new();
        // Seek to the beginning of the file
        self.file.seek(std::io::SeekFrom::Start(0))?;

        let reader = BufReader::new(&self.file);
        for line in reader.lines() {
            let line = line?;
            println!("Reading log line: {}", line);
            if let Some(entry) = LogEntry::from_string(&line) {
                println!("Parsed log entry: {:?}", entry);
                entries.push(entry);
            } else if !line.trim().is_empty() {
                println!("Failed to parse log line: {}", line);
            }
        }

        println!("Replay complete, found {} entries", entries.len());
        Ok(entries)
    }

    pub fn compact(&mut self) -> io::Result<()> {
        println!("Starting log compaction");

        // First, replay the log to get the current state
        let entries = self.replay()?;
        let mut current_state = HashMap::new();

        // Build the current state, keeping only the latest entry for each key
        for entry in entries {
            match entry {
                LogEntry::Set(key, value) => {
                    current_state.insert(key, Some(value));
                }
                LogEntry::Delete(key) => {
                    current_state.insert(key, None);
                }
                LogEntry::Compact => continue,
            }
        }

        // Create a temporary file for the compacted log
        let temp_path = self.path.with_extension("tmp");
        let mut temp_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&temp_path)?;

        // Write only the current state to the temporary file
        for (key, value_opt) in current_state {
            if let Some(value) = value_opt {
                let entry = LogEntry::Set(key, value);
                writeln!(temp_file, "{}", entry.to_string())?;
            }
        }
        temp_file.sync_all()?;

        // Release the lock on the old file
        fs2::FileExt::unlock(&self.file)?;

        // Close both files
        drop(temp_file);
        drop(std::mem::replace(
            &mut self.file,
            OpenOptions::new()
                .create(true)
                .append(true)
                .read(true)
                .mode(0o600)
                .open(&temp_path)?,
        ));

        // Rename the temporary file to the main log file
        fs::rename(&temp_path, &self.path)?;

        // Open and lock the new file
        self.file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .mode(0o600)
            .open(&self.path)?;
        self.file.try_lock_exclusive()?;

        Ok(())
    }

    pub fn unlock(&self) -> io::Result<()> {
        // Use fully qualified syntax to avoid naming conflicts
        fs2::FileExt::unlock(&self.file)
    }
}

impl Drop for LogFile {
    fn drop(&mut self) {
        // The lock will be automatically released when the file is closed
        println!("Log file closed and lock released");
    }
}
