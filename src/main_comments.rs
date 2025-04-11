// Import command-line argument handling
use std::env;
// Import file system operations
use std::fs::{File, OpenOptions};
// Import I/O operations for reading, writing, and seeking
use std::io::{BufRead, BufReader, Write, Seek, SeekFrom, Read};
// Import Unix-specific file operations for setting permissions
use std::os::unix::fs::OpenOptionsExt;
// Import path manipulation utilities
use std::path::Path;
// Import process management for spawning commands
use std::process::{Command, ExitStatus};
// Import thread and timing utilities
use std::{thread, time};
// Import atomic operations for thread-safe flags
use std::sync::atomic::{AtomicBool, Ordering};
// Import thread synchronization primitives
use std::sync::{Arc, Mutex, RwLock};
// Import thread-safe collections
use std::collections::HashMap;
// Import TCP networking types
use std::net::{TcpListener, TcpStream};
// Import I/O error types and buffered I/O
use std::io::{BufWriter, Error, ErrorKind};
// Import message passing types for thread communication
use std::sync::mpsc::{channel, Sender, Receiver};

use nix::unistd;

use fs2::FileExt;

// Path to the main database file
const DB_PATH: &str = "db.txt";
// Path to the cache file for faster access
const CACHE_PATH: &str = "cache.txt";
// Number of worker threads to spawn
const NUM_WORKERS: usize = 4;
// Server address and port for TCP connections
const SERVER_ADDR: &str = "127.0.0.1:7878";

// Thread-safe database structure
struct Database {
    // Main storage using a read-write lock for concurrent access
    data: RwLock<HashMap<String, String>>,
    // Cache storage using a read-write lock for concurrent access
    cache: RwLock<HashMap<String, String>>,
}

// Implementation of database operations
impl Database {
    // Create a new empty database instance
    fn new() -> Self {
        Self {
            // Initialize empty hashmaps with read-write locks
            data: RwLock::new(HashMap::new()),
            cache: RwLock::new(HashMap::new()),
        }
    }

    // Load data from the persistent file into memory
    fn load_from_file(&self) -> std::io::Result<()> {
        // Try to open the database file
        if let Ok(file) = File::open(DB_PATH) {
            // Create a buffered reader for efficient reading
            let reader = BufReader::new(file);
            // Get write access to the data hashmap
            let mut data = self.data.write().unwrap();
            // Read each line and parse key-value pairs
            for line in reader.lines().flatten() {
                if let Some((k, v)) = line.split_once('|') {
                    // Insert the key-value pair into the hashmap
                    data.insert(k.to_string(), v.to_string());
                }
            }
        }
        Ok(())
    }

    // Save in-memory data to the persistent file
    fn save_to_file(&self) -> std::io::Result<()> {
        // Create or open the file with write permissions
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(DB_PATH)?;
        
        // Get read access to the data hashmap
        let data = self.data.read().unwrap();
        // Write each key-value pair to the file
        for (k, v) in data.iter() {
            writeln!(file, "{}|{}", k, v)?;
        }
        Ok(())
    }

    // Retrieve a value by key, checking cache first
    fn get(&self, key: &str) -> Option<String> {
        // Try to get the value from cache first
        let cache = self.cache.read().unwrap();
        if let Some(value) = cache.get(key) {
            return Some(value.clone());
        }
        
        // If not in cache, check the main data store
        let data = self.data.read().unwrap();
        data.get(key).cloned()
    }

    // Set a key-value pair in both main data and cache
    fn set(&self, key: &str, value: &str) {
        // Get write access to both data and cache
        let mut data = self.data.write().unwrap();
        let mut cache = self.cache.write().unwrap();
        // Update both storages
        data.insert(key.to_string(), value.to_string());
        cache.insert(key.to_string(), value.to_string());
    }

    // Delete a key from both main data and cache
    fn delete(&self, key: &str) -> bool {
        // Get write access to both data and cache
        let mut data = self.data.write().unwrap();
        let mut cache = self.cache.write().unwrap();
        // Remove from both storages and return true if either contained the key
        data.remove(key).is_some() || cache.remove(key).is_some()
    }
}

lazy_static::lazy_static! {
    static ref DB_MUTEX: Mutex<()> = Mutex::new(());
}

// Main entry point of the program
fn main() {
    // Collect command line arguments into a vector
    let args: Vec<String> = env::args().collect();
    
    // Check if at least one command is provided
    if args.len() < 2 {
        eprintln!("Usage: {} [serve|get|set|delete] [key] [value?]", args[0]);
        return;
    }

    // Create a new thread-safe database instance
    let db = Arc::new(Database::new());
    // Load existing data from file
    if let Err(e) = db.load_from_file() {
        eprintln!("Error loading database: {}", e);
        return;
    }

    // Match on the command provided
    match args[1].as_str() {
        // Start the database server
        "serve" => {
            serve(db);
        }
        // Set a key-value pair
        "set" => {
            // Check for correct number of arguments
            if args.len() != 4 {
                eprintln!("Usage: {} set <key> <value>", args[0]);
                return;
            }
            // Extract key and value
            let key = &args[2];
            let value = &args[3];
            // Set the value and save to file
            db.set(key, value);
            if let Err(e) = db.save_to_file() {
                eprintln!("Error saving to file: {}", e);
            }
        }
        // Get a value by key
        "get" => {
            // Check for correct number of arguments
            if args.len() != 3 {
                eprintln!("Usage: {} get <key>", args[0]);
                return;
            }
            // Extract key and retrieve value
            let key = &args[2];
            match db.get(key) {
                Some(value) => println!("{}", value),
                None => println!("Key not found"),
            }
        }
        // Delete one or more keys
        "delete" => {
            // Check for at least one key to delete
            if args.len() < 3 {
                eprintln!("Usage: {} delete <key1> [key2...]", args[0]);
                return;
            }
            // Extract keys and delete each one
            let keys = &args[2..];
            for key in keys {
                if db.delete(key) {
                    println!("Deleted key '{}'", key);
                } else {
                    println!("Key '{}' not found", key);
                }
            }
            // Save changes to file
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

// Start the database server with worker threads
fn serve(db: Arc<Database>) {
    // Bind to the configured address
    let listener = TcpListener::bind(SERVER_ADDR).expect("Failed to bind to address");
    println!("Server listening on {}", SERVER_ADDR);

    // Set up graceful shutdown handling
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        // Set running flag to false on Ctrl-C
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    // Create a vector to store worker thread handles
    let mut handles = Vec::new();
    // Spawn worker threads
    for _ in 0..NUM_WORKERS {
        // Clone resources for the worker thread
        let listener = listener.try_clone().expect("Failed to clone listener");
        let db = db.clone();
        let running = running.clone();
        
        // Spawn a worker thread
        let handle = thread::spawn(move || {
            // Accept connections while running
            while running.load(Ordering::SeqCst) {
                if let Ok((stream, addr)) = listener.accept() {
                    println!("New connection from {}", addr);
                    // Clone database reference for the connection handler
                    let db = db.clone();
                    // Spawn a thread to handle this connection
                    thread::spawn(move || {
                        handle_client(stream, db);
                    });
                }
            }
        });
        handles.push(handle);
    }

    // Keep the main thread alive while running
    while running.load(Ordering::SeqCst) {
        thread::sleep(time::Duration::from_secs(1));
    }

    // Wait for all worker threads to finish
    for handle in handles {
        handle.join().unwrap();
    }
}

// Handle an individual client connection
fn handle_client(stream: TcpStream, db: Arc<Database>) {
    // Create buffered reader and writer for the stream
    let mut reader = BufReader::new(&stream);
    let mut writer = BufWriter::new(&stream);
    let mut line = String::new();

    // Read commands line by line
    while let Ok(n) = reader.read_line(&mut line) {
        // Break if connection is closed
        if n == 0 {
            break;
        }

        // Parse and execute the command
        let response = match parse_command(&line) {
            Ok((cmd, key, value)) => {
                match cmd.as_str() {
                    // Handle get command
                    "get" => {
                        match db.get(&key) {
                            Some(val) => format!("OK {}\n", val),
                            None => "ERROR Key not found\n".to_string(),
                        }
                    }
                    // Handle set command
                    "set" => {
                        match value {
                            Some(val) => {
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
                    // Handle delete command
                    "delete" => {
                        if db.delete(&key) {
                            if let Err(e) = db.save_to_file() {
                                format!("ERROR {}\n", e)
                            } else {
                                "OK\n".to_string()
                            }
                        } else {
                            "ERROR Key not found\n".to_string()
                        }
                    }
                    // Handle unknown commands
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
        if let Err(e) = writer.flush() {
            eprintln!("Error flushing writer: {}", e);
            break;
        }

        // Clear the line buffer for next command
        line.clear();
    }
}

// Parse a command string into its components
fn parse_command(line: &str) -> Result<(String, String, Option<String>), Error> {
    // Split command into whitespace-separated parts
    let parts: Vec<&str> = line.trim().split_whitespace().collect();
    // Check for empty command
    if parts.is_empty() {
        return Err(Error::new(ErrorKind::InvalidInput, "Empty command"));
    }

    // Convert command to lowercase for case-insensitive matching
    let cmd = parts[0].to_lowercase();
    match cmd.as_str() {
        // Handle get command format
        "get" => {
            if parts.len() != 2 {
                return Err(Error::new(ErrorKind::InvalidInput, "Usage: get <key>"));
            }
            Ok((cmd, parts[1].to_string(), None))
        }
        // Handle set command format
        "set" => {
            if parts.len() < 3 {
                return Err(Error::new(ErrorKind::InvalidInput, "Usage: set <key> <value>"));
            }
            // Join remaining parts as value to support spaces
            let value = parts[2..].join(" ");
            Ok((cmd, parts[1].to_string(), Some(value)))
        }
        // Handle delete command format
        "delete" => {
            if parts.len() != 2 {
                return Err(Error::new(ErrorKind::InvalidInput, "Usage: delete <key>"));
            }
            Ok((cmd, parts[1].to_string(), None))
        }
        // Handle unknown commands
        _ => Err(Error::new(ErrorKind::InvalidInput, "Unknown command")),
    }
}

fn get_with_cache(key: &str) -> Option<String> {
    if let Ok(file) = File::open(CACHE_PATH) {
        if let Ok(_lock) = file.try_lock_shared() {
            let reader = BufReader::new(&file);
            for line in reader.lines().flatten() {
                if let Some((k, v)) = line.split_once('|') {
                    if k == key {
                        return Some(v.to_string());
                    }
                }
            }
        }
    }

    get(key)
}

fn open_or_create_db() -> std::io::Result<File> {
    if !Path::new(DB_PATH).exists() {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .mode(0o600)
            .open(DB_PATH)?;
        Ok(file)
    } else {
        OpenOptions::new()
            .write(true)
            .read(true)
            .append(true)
            .open(DB_PATH)
    }
}

fn set(key: &str, value: &str) -> std::io::Result<()> {
    let mut file = open_or_create_db()?;
    FileExt::lock_exclusive(&file)?;
    
    let mut content = String::new();
    BufReader::new(&file).read_to_string(&mut content)?;
    
    let mut new_content = String::new();
    let mut found = false;
    for line in content.lines() {
        if let Some((k, _)) = line.split_once('|') {
            if k != key {
                new_content.push_str(line);
                new_content.push('\n');
            } else {
                found = true;
            }
        }
    }
    
    new_content.push_str(&format!("{}|{}\n", key, value));
    
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    write!(file, "{}", new_content)?;
    
    FileExt::unlock(&file)?;
    
    if found {
        println!("Updated key '{}'", key);
    } else {
        println!("Added new key '{}'", key);
    }
    Ok(())
}

/// Gets a value by key from the database
fn get(key: &str) -> Option<String> {
    let file = open_or_create_db().ok()?;
    FileExt::lock_shared(&file).ok()?;

    let reader = BufReader::new(&file);

    let mut result = None;
    for line in reader.lines().flatten() {
        if let Some((k, v)) = line.split_once('|') {
            if k == key {
                result = Some(v.to_string());
            }
        }
    }

    FileExt::unlock(&file).ok()?;
    result
}

/// Deletes one or more keys from the database
fn delete_keys(keys: &[String]) -> std::io::Result<()> {
    let mut children = Vec::new();

    for key in keys {
        let child = Command::new(env::current_exe()?)
            .arg("delete-key")
            .arg(key)
            .spawn()?;
        
        children.push(child);
    }

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

/// Deletes a single key from the database
fn delete_key(key: &str) -> std::io::Result<()> {
    let mut file = open_or_create_db()?;
    FileExt::lock_exclusive(&file)?;
    
    let mut content = String::new();
    BufReader::new(&file).read_to_string(&mut content)?;
    
    let mut found = false;
    let mut new_content = String::new();
    
    for line in content.lines() {
        if let Some((k, _)) = line.split_once('|') {
            if k != key {
                new_content.push_str(line);
                new_content.push('\n');
            } else {
                found = true;
            }
        }
    }
    
    if found {
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        write!(file, "{}", new_content)?;
        
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

    FileExt::unlock(&file)?;
    Ok(())
}

fn set_with_cache(key: &str, value: &str) -> std::io::Result<()> {
    set(key, value)?;
    
    let mut cache_file = OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .mode(0o600)
        .open(CACHE_PATH)?;
    
    let _lock = cache_file.try_lock_exclusive()?;
    
    let mut content = String::new();
    BufReader::new(&cache_file).read_to_string(&mut content)?;
    
    let mut new_content = String::new();
    for line in content.lines() {
        if let Some((k, _)) = line.split_once('|') {
            if k != key {
                new_content.push_str(line);
                new_content.push('\n');
            }
        }
    }
    
    new_content.push_str(&format!("{}|{}\n", key, value));
    
    cache_file.set_len(0)?;
    cache_file.seek(SeekFrom::Start(0))?;
    write!(cache_file, "{}", new_content)?;
    
    Ok(())
}
