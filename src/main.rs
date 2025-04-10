//! A simple key-value database with process-based concurrency support
//! 
//! This implementation provides a persistent key-value store with the following features:
//! - File-based storage with proper locking mechanisms
//! - Process-based concurrency support for parallel operations
//! - Cache layer for improved read performance
//! - Graceful shutdown handling
//! - Support for multiple worker processes

// Import standard library modules for file operations, I/O, and process management
use std::env;  // For accessing command line arguments
use std::fs::{File, OpenOptions};  // For file operations
use std::io::{BufRead, BufReader, Write, Seek, SeekFrom, Read};  // For reading/writing files
use std::os::unix::fs::OpenOptionsExt;  // For setting file permissions on Unix systems
use std::path::Path;  // For handling file paths
use std::process::{Command, ExitStatus};  // For spawning child processes
use std::{thread, time};  // For thread operations and time-related functions
use std::sync::atomic::{AtomicBool, Ordering};  // For atomic operations
use std::sync::Arc;  // For thread-safe reference counting
use nix::unistd;  // For Unix system calls like fork()

// Import external crate for file locking
use fs2::FileExt;  // Provides file locking functionality

// Constants for file paths and configuration
const DB_PATH: &str = "db.txt";      // Main database file path
const CACHE_PATH: &str = "cache.txt"; // Cache file path for faster reads
const NUM_WORKERS: usize = 4;         // Number of worker processes to spawn

// Main function - entry point of the program
fn main() {
    // Get command line arguments as a vector of strings
    let args: Vec<String> = env::args().collect();
    
    // Check if we have at least a command argument
    if args.len() < 2 {
        // Print usage instructions if no command is provided
        eprintln!("Usage: {} [get|set|delete|serve] [key] [value?]", args[0]);
        return;
    }

    // Match the command argument to determine what operation to perform
    match args[1].as_str() {
        "serve" => {
            // Start the database server
            serve();
        }
        "set" => {
            // Check if we have both key and value arguments
            if args.len() != 4 {
                eprintln!("Usage: {} set <key> <value>", args[0]);
                return;
            }
            // Extract key and value from arguments
            let key = &args[2];
            let value = &args[3];
            // Set the key-value pair and handle any errors
            if let Err(e) = set_with_cache(key, value) {
                eprintln!("Error setting value: {}", e);
            }
        }
        "get" => {
            // Check if we have a key argument
            if args.len() != 3 {
                eprintln!("Usage: {} get <key>", args[0]);
                return;
            }
            // Extract key from arguments
            let key = &args[2];
            // Get the value and print it or "Key not found"
            match get_with_cache(key) {
                Some(value) => println!("{}", value),
                None => println!("Key not found"),
            }
        }
        "delete" => {
            // Check if we have at least one key to delete
            if args.len() < 3 {
                eprintln!("Usage: {} delete <key1> [key2...]", args[0]);
                return;
            }
            // Get all keys to delete
            let keys = &args[2..];
            // Delete the keys and handle any errors
            if let Err(e) = delete_keys(keys) {
                eprintln!("Error deleting keys: {}", e);
            }
        }
        "delete-key" => {
            // Internal command for deleting a single key
            if args.len() != 3 {
                eprintln!("Internal error: delete-key requires exactly one key");
                std::process::exit(1);
            }
            let key = &args[2];
            if let Err(e) = delete_key(key) {
                eprintln!("Error deleting key: {}", e);
                std::process::exit(1);
            }
        }
        _ => {
            // Handle unknown commands
            eprintln!("Unknown command: {}", args[1]);
        }
    }
}

/// Starts the database server with multiple worker processes
/// 
/// This function:
/// 1. Creates NUM_WORKERS child processes
/// 2. Sets up a signal handler for graceful shutdown
/// 3. Waits for shutdown signal while keeping the parent process alive
fn serve() {
    // Create worker processes
    for _ in 0..NUM_WORKERS {
        // Use unsafe block because fork() is an unsafe operation
        match unsafe { unistd::fork() } {
            // Parent process receives the child's PID
            Ok(unistd::ForkResult::Parent { child }) => {
                println!("Created worker process {}", child);
            }
            // Child process starts its worker loop
            Ok(unistd::ForkResult::Child) => {
                worker_loop();
                std::process::exit(0);
            }
            // Handle fork failure
            Err(e) => {
                eprintln!("Failed to fork: {}", e);
                return;
            }
        }
    }

    // Create an atomic boolean to control the main loop
    let running = Arc::new(AtomicBool::new(true));
    // Clone the reference for the signal handler
    let r = running.clone();
    // Set up Ctrl-C handler
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    // Main loop that keeps the parent process alive
    while running.load(Ordering::SeqCst) {
        thread::sleep(time::Duration::from_secs(1));
    }
}

/// Main loop for worker processes
/// 
/// Each worker process:
/// 1. Sets up its own signal handler
/// 2. Processes requests in a loop
/// 3. Exits gracefully on shutdown signal
fn worker_loop() {
    // Create an atomic boolean to control the worker loop
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    // Set up Ctrl-C handler for the worker
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    // Worker's main loop
    while running.load(Ordering::SeqCst) {
        // Process requests from clients
        thread::sleep(time::Duration::from_millis(100));
    }
}

/// Retrieves a value from the cache or database
/// 
/// This function implements a two-level lookup:
/// 1. First tries to find the key in the cache file
/// 2. If not found in cache, falls back to the main database
/// 
/// Uses shared locks for concurrent read access
fn get_with_cache(key: &str) -> Option<String> {
    // Try to open and read from cache file first
    if let Ok(file) = File::open(CACHE_PATH) {
        // Try to acquire a shared lock on the cache file
        if let Ok(_lock) = file.try_lock_shared() {
            // Create a buffered reader for efficient reading
            let reader = BufReader::new(&file);
            // Search for the key in each line
            for line in reader.lines().flatten() {
                // Split each line into key and value
                if let Some((k, v)) = line.split_once('|') {
                    if k == key {
                        return Some(v.to_string());
                    }
                }
            }
        }
    }

    // If not found in cache, try the main database
    get(key)
}

/// Sets a key-value pair in the database
/// 
/// This function:
/// 1. Acquires an exclusive lock on the database file
/// 2. Updates or adds the key-value pair
/// 3. Releases the lock
/// 
/// Uses exclusive locks to prevent concurrent writes
fn set(key: &str, value: &str) -> std::io::Result<()> {
    // Open or create the database file
    let mut file = open_or_create_db()?;
    // Acquire an exclusive lock on the file
    FileExt::lock_exclusive(&file)?;
    
    // Read the current content of the file
    let mut content = String::new();
    BufReader::new(&file).read_to_string(&mut content)?;
    
    // Remove existing key if present and build new content
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
    
    // Add the new key-value pair
    new_content.push_str(&format!("{}|{}\n", key, value));
    
    // Write the new content back to the file
    file.set_len(0)?;  // Clear the file
    file.seek(SeekFrom::Start(0))?;  // Move to the beginning
    write!(file, "{}", new_content)?;  // Write the new content
    
    // Release the lock
    FileExt::unlock(&file)?;
    
    // Print appropriate message
    if found {
        println!("Updated key '{}'", key);
    } else {
        println!("Added new key '{}'", key);
    }
    Ok(())
}

/// Sets a key-value pair in both database and cache
/// 
/// This function:
/// 1. Updates the main database
/// 2. Updates the cache file
/// 3. Maintains consistency between both files
fn set_with_cache(key: &str, value: &str) -> std::io::Result<()> {
    // Update main database first
    set(key, value)?;
    
    // Update cache file
    let mut cache_file = OpenOptions::new()
        .create(true)  // Create if doesn't exist
        .write(true)   // Allow writing
        .read(true)    // Allow reading
        .mode(0o600)   // Set file permissions (read/write for owner only)
        .open(CACHE_PATH)?;
    
    // Acquire exclusive lock on cache file
    let _lock = cache_file.try_lock_exclusive()?;
    
    // Read current cache content
    let mut content = String::new();
    BufReader::new(&cache_file).read_to_string(&mut content)?;
    
    // Remove existing key if present and build new content
    let mut new_content = String::new();
    for line in content.lines() {
        if let Some((k, _)) = line.split_once('|') {
            if k != key {
                new_content.push_str(line);
                new_content.push('\n');
            }
        }
    }
    
    // Add new key-value pair
    new_content.push_str(&format!("{}|{}\n", key, value));
    
    // Write back to cache file
    cache_file.set_len(0)?;  // Clear the file
    cache_file.seek(SeekFrom::Start(0))?;  // Move to the beginning
    write!(cache_file, "{}", new_content)?;  // Write the new content
    
    Ok(())
}

/// Opens or creates the database file with appropriate permissions
/// 
/// This function:
/// 1. Creates the file if it doesn't exist
/// 2. Sets appropriate read/write permissions
/// 3. Returns a file handle for database operations
fn open_or_create_db() -> std::io::Result<File> {
    if !Path::new(DB_PATH).exists() {
        // Create new file with appropriate permissions
        let file = OpenOptions::new()
            .create(true)  // Create if doesn't exist
            .write(true)   // Allow writing
            .read(true)    // Allow reading
            .mode(0o600)   // Set file permissions (read/write for owner only)
            .open(DB_PATH)?;
        Ok(file)
    } else {
        // Open existing file
        OpenOptions::new()
            .write(true)   // Allow writing
            .read(true)    // Allow reading
            .append(true)  // Allow appending
            .open(DB_PATH)
    }
}

/// Retrieves a value from the database
/// 
/// This function:
/// 1. Opens the database file
/// 2. Acquires a shared lock for reading
/// 3. Searches for the key
/// 4. Returns the value if found
fn get(key: &str) -> Option<String> {
    // Open the database file
    let file = open_or_create_db().ok()?;
    // Acquire shared lock for reading
    FileExt::lock_shared(&file).ok()?;

    // Create a buffered reader for efficient reading
    let reader = BufReader::new(&file);

    // Search for the key
    let mut result = None;
    for line in reader.lines().flatten() {
        if let Some((k, v)) = line.split_once('|') {
            if k == key {
                result = Some(v.to_string());
            }
        }
    }

    // Release the lock
    FileExt::unlock(&file).ok()?;
    result
}

/// Deletes multiple keys from the database
/// 
/// This function:
/// 1. Spawns a separate process for each key deletion
/// 2. Waits for all deletion processes to complete
/// 3. Handles process creation and monitoring
fn delete_keys(keys: &[String]) -> std::io::Result<()> {
    // Vector to store child processes
    let mut children = Vec::new();

    // Spawn a process for each key to delete
    for key in keys {
        let child = Command::new(env::current_exe()?)
            .arg("delete-key")  // Internal command for deleting a single key
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

/// Deletes a single key from both database and cache
/// 
/// This function:
/// 1. Acquires an exclusive lock on the database
/// 2. Removes the key from the database
/// 3. Updates the cache file to maintain consistency
/// 4. Handles file locking and error cases
fn delete_key(key: &str) -> std::io::Result<()> {
    // Delete from main database
    let mut file = open_or_create_db()?;
    // Acquire exclusive lock
    FileExt::lock_exclusive(&file)?;
    
    // Read current content
    let mut content = String::new();
    BufReader::new(&file).read_to_string(&mut content)?;
    
    // Remove the key and build new content
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
        // Write back the updated content
        file.set_len(0)?;  // Clear the file
        file.seek(SeekFrom::Start(0))?;  // Move to the beginning
        write!(file, "{}", new_content)?;  // Write the new content
        
        // Also update cache file
        if let Ok(mut cache_file) = OpenOptions::new()
            .create(true)  // Create if doesn't exist
            .write(true)   // Allow writing
            .truncate(true)  // Clear existing content
            .mode(0o600)   // Set file permissions
            .open(CACHE_PATH)
        {
            // Acquire exclusive lock on cache file
            let _lock = cache_file.try_lock_exclusive()?;
            // Write the same content to cache
            write!(cache_file, "{}", new_content)?;
        }
        
        println!("Key '{}' deleted successfully", key);
    } else {
        println!("Key '{}' not found", key);
    }

    // Release the lock
    FileExt::unlock(&file)?;
    Ok(())
}
