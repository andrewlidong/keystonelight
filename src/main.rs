// Import standard library modules for command-line argument handling
use std::env;

// Import file system modules for file operations
use std::fs::{File, OpenOptions};

// Import I/O modules for reading, writing, and seeking in files
use std::io::{BufRead, BufReader, Write, Seek, SeekFrom, Read};

// Import Unix-specific file operations for setting file permissions
use std::os::unix::fs::OpenOptionsExt;

// Import path manipulation utilities for working with file paths
use std::path::Path;

// Import process management modules for handling child processes
use std::process::Command;

// Import thread and timing utilities for concurrent operations
use std::{thread, time};

// Import atomic operations for thread-safe flags
use std::sync::atomic::{AtomicBool, Ordering};

// Import thread synchronization primitives for safe concurrent access
use std::sync::{Arc, Mutex, RwLock};

// Import thread-safe hash map for storing key-value pairs
use std::collections::HashMap;

// Import TCP networking types for client-server communication
use std::net::{TcpListener, TcpStream};

// Import I/O error types and buffered I/O for efficient network operations
use std::io::{BufWriter, Error, ErrorKind};

// Import file locking capabilities
use fs2::FileExt;

// Import signal handling utilities
use signal_hook::consts::signal::SIGUSR1 as SIGUSR1_HOOK;
use signal_hook::flag;

// Define the path to the main database file where all key-value pairs are stored
const DB_PATH: &str = "db.txt";

// Define the path to the cache file for faster access to frequently used data
const CACHE_PATH: &str = "cache.txt";

// Define the number of worker threads to handle concurrent client connections
const NUM_WORKERS: usize = 4;

// Define the server's listening address and port
const SERVER_ADDR: &str = "127.0.0.1:7878";

// Define the path to store the server's process ID
const PID_FILE: &str = "keystonelight.pid";

// Define a thread-safe database structure that manages concurrent access to data
struct Database {
    // Main storage: HashMap protected by a read-write lock for concurrent access
    data: RwLock<HashMap<String, String>>,
    // Cache storage: HashMap protected by a read-write lock for faster access
    cache: RwLock<HashMap<String, String>>,
}

// Implementation of database operations
impl Database {
    // Create a new empty database instance with initialized storage
    fn new() -> Self {
        Self {
            // Initialize the main data store with an empty HashMap protected by RwLock
            data: RwLock::new(HashMap::new()),
            // Initialize the cache with an empty HashMap protected by RwLock
            cache: RwLock::new(HashMap::new()),
        }
    }

    // Load existing data from the database file into memory
    fn load_from_file(&self) -> std::io::Result<()> {
        // Attempt to open the database file for reading
        if let Ok(file) = File::open(DB_PATH) {
            // Create a buffered reader for efficient line-by-line reading
            let reader = BufReader::new(file);
            // Acquire write lock on the data HashMap to update it
            let mut data = self.data.write().unwrap();
            
            // Process each line in the file
            for line in reader.lines().flatten() {
                // Split each line into key and value using '|' as separator
                if let Some((k, v)) = line.split_once('|') {
                    // Insert the key-value pair into the main data store
                    data.insert(k.to_string(), v.to_string());
                }
            }
        }
        Ok(())
    }

    // Save the current in-memory data to the database file
    fn save_to_file(&self) -> std::io::Result<()> {
        // Open or create the database file with proper permissions
        let mut file = OpenOptions::new()
            .create(true)     // Create file if it doesn't exist
            .write(true)      // Enable write access
            .truncate(true)   // Clear existing content
            .mode(0o600)      // Set Unix permissions (owner read/write only)
            .open(DB_PATH)?;
        
        // Acquire read lock on the data HashMap
        let data = self.data.read().unwrap();
        
        // Write each key-value pair to the file
        for (k, v) in data.iter() {
            writeln!(file, "{}|{}", k, v)?;
        }
        Ok(())
    }

    // Retrieve a value by its key, checking cache first
    fn get(&self, key: &str) -> Option<String> {
        // First try to get the value from cache
        let cache = self.cache.read().unwrap();
        if let Some(value) = cache.get(key) {
            // Return cached value if found
            return Some(value.clone());
        }
        
        // If not in cache, check the main data store
        let data = self.data.read().unwrap();
        // Return cloned value if found, None if not found
        data.get(key).cloned()
    }

    // Set a key-value pair in both main storage and cache
    fn set(&self, key: &str, value: &str) {
        // Acquire write locks for both data and cache
        let mut data = self.data.write().unwrap();
        let mut cache = self.cache.write().unwrap();
        
        // Update both storages with the new key-value pair
        data.insert(key.to_string(), value.to_string());
        cache.insert(key.to_string(), value.to_string());
    }

    // Delete a key from both main storage and cache
    fn delete(&self, key: &str) -> bool {
        // Acquire write locks for both data and cache
        let mut data = self.data.write().unwrap();
        let mut cache = self.cache.write().unwrap();
        
        // Remove the key from both storages
        // Return true if the key was present in either storage
        data.remove(key).is_some() || cache.remove(key).is_some()
    }

    // Compact the database file by removing deleted entries
    fn compact(&self) -> std::io::Result<()> {
        // Acquire read lock on the data HashMap
        let data = self.data.read().unwrap();
        
        // Create a temporary file for the compacted data
        let temp_path = format!("{}.tmp", DB_PATH);
        let mut temp_file = OpenOptions::new()
            .create(true)     // Create new file
            .write(true)      // Enable write access
            .truncate(true)   // Clear any existing content
            .mode(0o600)      // Set Unix permissions
            .open(&temp_path)?;
        
        // Write all current entries to the temporary file
        for (k, v) in data.iter() {
            writeln!(temp_file, "{}|{}", k, v)?;
        }
        
        // Replace the old database file with the compacted one
        std::fs::rename(&temp_path, DB_PATH)?;
        
        Ok(())
    }
}

// Global mutex for database-wide operations
lazy_static::lazy_static! {
    static ref DB_MUTEX: Mutex<()> = Mutex::new(());
}

// Main entry point of the program
fn main() {
    // Parse command line arguments into a vector
    let args: Vec<String> = env::args().collect();
    
    // Ensure at least one command is provided
    if args.len() < 2 {
        eprintln!("Usage: {} [serve|get|set|delete] [key] [value?]", args[0]);
        return;
    }

    // Create a new thread-safe database instance
    let db = Arc::new(Database::new());
    
    // Load existing data from the database file
    if let Err(e) = db.load_from_file() {
        eprintln!("Error loading database: {}", e);
        return;
    }

    // Process the command based on the first argument
    match args[1].as_str() {
        // Start the database server
        "serve" => {
            serve(db);
        }
        // Handle set command
        "set" => {
            // Validate argument count for set command
            if args.len() != 4 {
                eprintln!("Usage: {} set <key> <value>", args[0]);
                return;
            }
            // Extract key and value from arguments
            let key = &args[2];
            let value = &args[3];
            // Update the database and persist changes
            db.set(key, value);
            if let Err(e) = db.save_to_file() {
                eprintln!("Error saving to file: {}", e);
            }
        }
        // Handle get command
        "get" => {
            // Validate argument count for get command
            if args.len() != 3 {
                eprintln!("Usage: {} get <key>", args[0]);
                return;
            }
            // Extract key and retrieve its value
            let key = &args[2];
            match db.get(key) {
                Some(value) => println!("{}", value),
                None => println!("Key not found"),
            }
        }
        // Handle delete command
        "delete" => {
            // Validate argument count for delete command
            if args.len() < 3 {
                eprintln!("Usage: {} delete <key1> [key2...]", args[0]);
                return;
            }
            // Process each key to be deleted
            let keys = &args[2..];
            for key in keys {
                if db.delete(key) {
                    println!("Deleted key '{}'", key);
                } else {
                    println!("Key '{}' not found", key);
                }
            }
            // Persist changes to the database file
            if let Err(e) = db.save_to_file() {
                eprintln!("Error saving to file: {}", e);
            }
        }
        // Handle unknown commands
        _ => {
            eprintln!("Unknown command: {}", args[1]);
        }
    }
}

// Start the database server and manage worker threads
fn serve(db: Arc<Database>) {
    // Write the server's process ID to a file for external management
    if let Err(e) = std::fs::write(PID_FILE, std::process::id().to_string()) {
        eprintln!("Failed to write PID file: {}", e);
    }

    // Set up signal handler for database compaction
    let compaction_requested = Arc::new(AtomicBool::new(false));
    flag::register(SIGUSR1_HOOK, Arc::clone(&compaction_requested))
        .expect("Failed to set up SIGUSR1 handler");

    // Create TCP listener bound to the configured address
    let listener = TcpListener::bind(SERVER_ADDR).expect("Failed to bind to address");
    println!("Server listening on {}", SERVER_ADDR);

    // Set up graceful shutdown handling using Ctrl-C
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        // Set running flag to false when Ctrl-C is received
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    // Initialize worker thread pool
    let mut handles = Vec::new();
    
    // Spawn the configured number of worker threads
    for _ in 0..NUM_WORKERS {
        // Clone resources needed by the worker thread
        let listener = listener.try_clone().expect("Failed to clone listener");
        let db = db.clone();
        let running = running.clone();
        
        // Spawn a new worker thread
        let handle = thread::spawn(move || {
            // Accept and handle connections while the server is running
            while running.load(Ordering::SeqCst) {
                if let Ok((stream, addr)) = listener.accept() {
                    println!("New connection from {}", addr);
                    // Clone database reference for the connection handler
                    let db = db.clone();
                    // Spawn a dedicated thread for this connection
                    thread::spawn(move || {
                        handle_client(stream, db);
                    });
                }
            }
        });
        handles.push(handle);
    }

    // Main server loop
    while running.load(Ordering::SeqCst) {
        // Check for pending compaction requests
        if compaction_requested.load(Ordering::SeqCst) {
            println!("Compacting database...");
            if let Err(e) = db.compact() {
                eprintln!("Failed to compact database: {}", e);
            } else {
                println!("Database compaction completed");
            }
            compaction_requested.store(false, Ordering::SeqCst);
        }
        // Sleep to prevent busy-waiting
        thread::sleep(time::Duration::from_secs(1));
    }

    // Clean up resources on shutdown
    let _ = std::fs::remove_file(PID_FILE);

    // Wait for all worker threads to finish
    for handle in handles {
        handle.join().unwrap();
    }
}

// Handle client connections and process their commands
fn handle_client(stream: TcpStream, db: Arc<Database>) {
    // Set up buffered reader and writer for efficient I/O
    let mut reader = BufReader::new(&stream);
    let mut writer = BufWriter::new(&stream);
    let mut line = String::new();

    // Process commands until the connection is closed
    while let Ok(n) = reader.read_line(&mut line) {
        // Exit loop if client closed the connection (n == 0)
        if n == 0 {
            break;
        }

        // Parse and execute the command, then prepare response
        let response = match parse_command(&line) {
            Ok((cmd, key, value)) => {
                match cmd.as_str() {
                    // Handle get command: retrieve value for key
                    "get" => {
                        match db.get(&key) {
                            Some(val) => format!("OK {}\n", val),
                            None => "ERROR Key not found\n".to_string(),
                        }
                    }
                    // Handle set command: store key-value pair
                    "set" => {
                        match value {
                            Some(val) => {
                                // Update database and persist changes
                                db.set(&key, &val);
                                if let Err(e) = db.save_to_file() {
                                    format!("ERROR {}\n", e)
                                } else {
                                    "OK\n".to_string()
                                }
                            }
                            None => "ERROR Missing value for set command\n".to_string(),
                        }
                    }
                    // Handle delete command: remove key from database
                    "delete" => {
                        if db.delete(&key) {
                            // Persist changes after successful deletion
                            if let Err(e) = db.save_to_file() {
                                format!("ERROR {}\n", e)
                            } else {
                                "OK\n".to_string()
                            }
                        } else {
                            "ERROR Key not found\n".to_string()
                        }
                    }
                    // Return error for unknown commands
                    _ => "ERROR Unknown command\n".to_string(),
                }
            }
            Err(e) => format!("ERROR {}\n", e),
        };

        // Send response to client
        if let Err(e) = writer.write_all(response.as_bytes()) {
            eprintln!("Error writing to client: {}", e);
            break;
        }
        // Ensure response is sent immediately
        if let Err(e) = writer.flush() {
            eprintln!("Error flushing writer: {}", e);
            break;
        }

        // Clear the line buffer for next command
        line.clear();
    }
}

// Parse a command string into (command, key, optional_value)
fn parse_command(line: &str) -> Result<(String, String, Option<String>), Error> {
    // Split command line into whitespace-separated parts
    let parts: Vec<&str> = line.trim().split_whitespace().collect();
    
    // Return error if command is empty
    if parts.is_empty() {
        return Err(Error::new(ErrorKind::InvalidInput, "Empty command"));
    }

    // Convert command to lowercase for case-insensitive matching
    let cmd = parts[0].to_lowercase();
    
    // Process command based on its type
    match cmd.as_str() {
        // Handle get command format: get <key>
        "get" => {
            if parts.len() != 2 {
                return Err(Error::new(ErrorKind::InvalidInput, "Usage: get <key>"));
            }
            Ok((cmd, parts[1].to_string(), None))
        }
        // Handle set command format: set <key> <value>
        "set" => {
            if parts.len() < 3 {
                return Err(Error::new(ErrorKind::InvalidInput, "Usage: set <key> <value>"));
            }
            // Join remaining parts as value to support spaces in values
            let value = parts[2..].join(" ");
            Ok((cmd, parts[1].to_string(), Some(value)))
        }
        // Handle delete command format: delete <key>
        "delete" => {
            if parts.len() != 2 {
                return Err(Error::new(ErrorKind::InvalidInput, "Usage: delete <key>"));
            }
            Ok((cmd, parts[1].to_string(), None))
        }
        // Return error for unknown commands
        _ => Err(Error::new(ErrorKind::InvalidInput, "Unknown command")),
    }
}

// Retrieve a value from cache if available, otherwise from main storage
fn get_with_cache(key: &str) -> Option<String> {
    // Try to read from cache file first
    if let Ok(file) = File::open(CACHE_PATH) {
        // Acquire shared lock to read cache
        if let Ok(_lock) = file.try_lock_shared() {
            let reader = BufReader::new(&file);
            // Search for key in cache
            for line in reader.lines().flatten() {
                if let Some((k, v)) = line.split_once('|') {
                    if k == key {
                        return Some(v.to_string());
                    }
                }
            }
        }
    }

    // If not found in cache, try main storage
    get(key)
}

// Open existing database file or create a new one with proper permissions
fn open_or_create_db() -> std::io::Result<File> {
    if !Path::new(DB_PATH).exists() {
        // Create new database file with read/write permissions
        let file = OpenOptions::new()
            .create(true)     // Create if doesn't exist
            .write(true)      // Enable write access
            .read(true)       // Enable read access
            .mode(0o600)      // Set Unix permissions (owner read/write only)
            .open(DB_PATH)?;
        Ok(file)
    } else {
        // Open existing database file with read/write/append permissions
        OpenOptions::new()
            .write(true)      // Enable write access
            .read(true)       // Enable read access
            .append(true)     // Enable append mode
            .open(DB_PATH)
    }
}

// Store a key-value pair in the database file
fn set(key: &str, value: &str) -> std::io::Result<()> {
    // Open database file with exclusive lock
    let mut file = open_or_create_db()?;
    FileExt::lock_exclusive(&file)?;
    
    // Read current contents
    let mut content = String::new();
    BufReader::new(&file).read_to_string(&mut content)?;
    
    // Prepare new content, excluding existing entry if any
    let mut new_content = String::new();
    let mut found = false;
    for line in content.lines() {
        if let Some((k, _)) = line.split_once('|') {
            if k != key {
                // Keep all entries except the one being updated
                new_content.push_str(line);
                new_content.push('\n');
            } else {
                found = true;
            }
        }
    }
    
    // Append new key-value pair
    new_content.push_str(&format!("{}|{}\n", key, value));
    
    // Write updated content back to file
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    write!(file, "{}", new_content)?;
    
    // Release exclusive lock
    FileExt::unlock(&file)?;
    
    // Log operation result
    if found {
        println!("Updated key '{}'", key);
    } else {
        println!("Added new key '{}'", key);
    }
    Ok(())
}

// Retrieve a value by key from the database file
fn get(key: &str) -> Option<String> {
    // Open database file with shared lock
    let file = open_or_create_db().ok()?;
    FileExt::lock_shared(&file).ok()?;

    // Search for key in database
    let reader = BufReader::new(&file);
    let mut result = None;
    for line in reader.lines().flatten() {
        if let Some((k, v)) = line.split_once('|') {
            if k == key {
                result = Some(v.to_string());
            }
        }
    }

    // Release shared lock
    FileExt::unlock(&file).ok()?;
    result
}

// Delete multiple keys from the database in parallel
fn delete_keys(keys: &[String]) -> std::io::Result<()> {
    // Vector to store child process handles
    let mut children = Vec::new();

    // Spawn a process for each key to be deleted
    for key in keys {
        let child = Command::new(env::current_exe()?)
            .arg("delete-key")
            .arg(key)
            .spawn()?;
        
        children.push(child);
    }

    // Wait for all child processes to complete
    for mut child in children {
        match child.wait() {
            Ok(status) => {
                if status.success() {
                    println!("Child process {} completed successfully", child.id());
                } else {
                    println!("Child process {} failed with code {:?}", child.id(), status.code());
                }
            }
            Err(e) => {
                eprintln!("Error waiting for child process {}: {}", child.id(), e);
            }
        }
    }

    Ok(())
}

// Delete a single key from the database file
fn delete_key(key: &str) -> std::io::Result<()> {
    // Open database file with exclusive lock
    let mut file = open_or_create_db()?;
    FileExt::lock_exclusive(&file)?;
    
    // Read current contents
    let mut content = String::new();
    BufReader::new(&file).read_to_string(&mut content)?;
    
    // Prepare new content, excluding the key to be deleted
    let mut found = false;
    let mut new_content = String::new();
    
    for line in content.lines() {
        if let Some((k, _)) = line.split_once('|') {
            if k != key {
                // Keep all entries except the one being deleted
                new_content.push_str(line);
                new_content.push('\n');
            } else {
                found = true;
            }
        }
    }
    
    // Update file if key was found
    if found {
        // Write updated content back to file
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        write!(file, "{}", new_content)?;
        
        // Update cache file if it exists
        if let Ok(mut cache_file) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(CACHE_PATH)
        {
            let _lock = cache_file.try_lock_exclusive()?;
            write!(cache_file, "{}", new_content)?;
        }
        
        println!("Key '{}' deleted successfully", key);
    } else {
        println!("Key '{}' not found", key);
    }

    // Release exclusive lock
    FileExt::unlock(&file)?;
    Ok(())
}

// Store a key-value pair in both database and cache files
fn set_with_cache(key: &str, value: &str) -> std::io::Result<()> {
    // Update main database first
    set(key, value)?;
    
    // Open cache file with exclusive lock
    let mut cache_file = OpenOptions::new()
        .create(true)     // Create if doesn't exist
        .write(true)      // Enable write access
        .read(true)       // Enable read access
        .mode(0o600)      // Set Unix permissions
        .open(CACHE_PATH)?;
    
    let _lock = cache_file.try_lock_exclusive()?;
    
    // Read current cache contents
    let mut content = String::new();
    BufReader::new(&cache_file).read_to_string(&mut content)?;
    
    // Prepare new content, excluding existing entry if any
    let mut new_content = String::new();
    for line in content.lines() {
        if let Some((k, _)) = line.split_once('|') {
            if k != key {
                new_content.push_str(line);
                new_content.push('\n');
            }
        }
    }
    
    // Append new key-value pair
    new_content.push_str(&format!("{}|{}\n", key, value));
    
    // Write updated content back to cache file
    cache_file.set_len(0)?;
    cache_file.seek(SeekFrom::Start(0))?;
    write!(cache_file, "{}", new_content)?;
    
    Ok(())
}
