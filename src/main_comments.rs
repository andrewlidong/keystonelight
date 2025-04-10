// Import necessary standard library modules
use std::env;  // For accessing command line arguments
use std::fs::{File, OpenOptions};  // For file operations
use std::io::{BufRead, BufReader, Write, Seek, SeekFrom, Read};  // For I/O operations
use std::os::unix::fs::OpenOptionsExt;  // For Unix-specific file operations
use std::path::Path;  // For path manipulation
use std::process::{Command, ExitStatus};  // For spawning child processes
use std::{thread, time};  // For thread operations and time delays

use fs2::FileExt;  // For cross-platform file locking

// Define the path to our database file
const DB_PATH: &str = "db.txt";

fn main() {
    // Get command line arguments
    // args[0] is the program name, args[1] is the command, args[2..] are arguments
    let args: Vec<String> = env::args().collect();
    
    // Check if we have at least a command (get/set/delete)
    // We need at least 2 arguments: program name and command
    if args.len() < 2 {
        eprintln!("Usage: {} [get|set|delete] [key] [value?]", args[0]);
        return;
    }

    // Match the command and handle accordingly
    // Each command has different argument requirements
    match args[1].as_str() {
        "set" => {
            // set command requires exactly 4 arguments: program name, "set", key, and value
            if args.len() != 4 {
                eprintln!("Usage: {} set <key> <value>", args[0]);
                return;
            }
            let key = &args[2];
            let value = &args[3];
            // Handle any errors that occur during set operation
            if let Err(e) = set(key, value) {
                eprintln!("Error setting value: {}", e);
            }
        }
        "get" => {
            // get command requires exactly 3 arguments: program name, "get", and key
            if args.len() != 3 {
                eprintln!("Usage: {} get <key>", args[0]);
                return;
            }
            let key = &args[2];
            // Handle both success and failure cases for get operation
            match get(key) {
                Some(value) => println!("{}", value),
                None => println!("Key not found"),
            }
        }
        "delete" => {
            // delete command requires at least 3 arguments: program name, "delete", and at least one key
            if args.len() < 3 {
                eprintln!("Usage: {} delete <key1> [key2...]", args[0]);
                return;
            }
            let keys = &args[2..];
            // Handle any errors that occur during delete operation
            if let Err(e) = delete_keys(keys) {
                eprintln!("Error deleting keys: {}", e);
            }
        }
        "delete-key" => {
            // Internal command used by delete_keys to delete a single key
            // This is not meant to be called directly by users
            if args.len() != 3 {
                eprintln!("Internal error: delete-key requires exactly one key");
                std::process::exit(1);
            }
            let key = &args[2];
            // If deletion fails, exit with error code 1
            if let Err(e) = delete_key(key) {
                eprintln!("Error deleting key: {}", e);
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
        }
    }
}

// Opens the database file, creating it if it doesn't exist
// Returns a Result<File> which can be used to handle potential I/O errors
fn open_or_create_db() -> std::io::Result<File> {
    if !Path::new(DB_PATH).exists() {
        // Create a new file with read/write permissions and Unix mode 600 (owner read/write only)
        // Mode 600 means only the owner can read and write, others have no permissions
        let file = OpenOptions::new()
            .create(true)  // Create the file if it doesn't exist
            .write(true)   // Allow writing to the file
            .read(true)    // Allow reading from the file
            .mode(0o600)   // Set Unix file permissions to 600
            .open(DB_PATH)?;
        Ok(file)
    } else {
        // Open existing file with read/write/append permissions
        // Append mode ensures we don't overwrite existing data
        OpenOptions::new()
            .write(true)    // Allow writing to the file
            .read(true)     // Allow reading from the file
            .append(true)   // Open in append mode
            .open(DB_PATH)
    }
}

// Sets a key-value pair in the database
// Uses exclusive locking to prevent concurrent writes
fn set(key: &str, value: &str) -> std::io::Result<()> {
    // Open the database file
    let mut file = open_or_create_db()?;
    // Get an exclusive lock on the file (blocks other processes from accessing it)
    // This ensures data consistency during writes
    FileExt::lock_exclusive(&file)?;
    // Simulate a long operation by sleeping (for demonstration purposes)
    // This helps demonstrate the locking mechanism
    println!("Holding lock... (sleeping 5 seconds)");
    thread::sleep(time::Duration::from_secs(5));

    // Write the key-value pair to the file
    // The format is "key|value\n"
    writeln!(file, "{}|{}", key, value)?;
    // Release the lock to allow other processes to access the file
    FileExt::unlock(&file)?;
    println!("Lock released.");
    Ok(())
}

// Retrieves a value for a given key from the database
// Uses shared locking to allow concurrent reads while preventing writes
fn get(key: &str) -> Option<String> {
    // Open the database file
    let file = open_or_create_db().ok()?;
    // Get a shared lock (allows other readers but blocks writers)
    // This ensures we read consistent data while allowing concurrent reads
    FileExt::lock_shared(&file).ok()?;

    // Create a buffered reader for efficient line-by-line reading
    // Buffering improves performance by reducing I/O operations
    let reader = BufReader::new(&file);

    // Search for the key in each line
    let mut result = None;
    for line in reader.lines().flatten() {
        // Split each line into key and value using '|' as delimiter
        // The format is "key|value"
        if let Some((k, v)) = line.split_once('|') {
            if k == key {
                result = Some(v.to_string());
            }
        }
    }

    // Release the lock to allow other processes to access the file
    FileExt::unlock(&file).ok()?;
    result
}

// Deletes multiple keys by spawning separate processes for each deletion
// This allows concurrent deletion of different keys
fn delete_keys(keys: &[String]) -> std::io::Result<()> {
    let mut children = Vec::new();

    // Spawn a child process for each key to be deleted
    // Each process handles one key deletion independently
    for key in keys {
        let mut child = Command::new(env::current_exe()?)
            .arg("delete-key")  // Use the internal delete-key command
            .arg(key)
            .spawn()?;
        
        children.push(child);
    }

    // Wait for all child processes to complete
    // This ensures we don't exit before all deletions are done
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

// Internal function to delete a single key from the database
// Uses exclusive locking to ensure data consistency during deletion
fn delete_key(key: &str) -> std::io::Result<()> {
    // Open the database file
    let mut file = open_or_create_db()?;
    // Get an exclusive lock
    // This prevents other processes from accessing the file during deletion
    FileExt::lock_exclusive(&file)?;
    
    // Read the entire file content
    // We need to read all data to find and remove the target key
    let mut content = String::new();
    BufReader::new(&file).read_to_string(&mut content)?;
    
    let mut found = false;
    let mut new_content = String::new();
    
    // Process each line, keeping only the lines that don't match the key to be deleted
    // This effectively removes the key-value pair from the database
    for line in content.lines() {
        if let Some((k, v)) = line.split_once('|') {
            if k != key {
                new_content.push_str(line);
                new_content.push('\n');
            } else {
                found = true;
            }
        }
    }
    
    // If the key was found and deleted, write the new content back to the file
    if found {
        file.set_len(0)?;  // Clear the file by setting its length to 0
        file.seek(std::io::SeekFrom::Start(0))?;  // Move to the start of the file
        write!(file, "{}", new_content)?;  // Write the new content (without the deleted key)
        println!("Key '{}' deleted successfully", key);
    } else {
        println!("Key '{}' not found", key);
    }

    // Release the lock to allow other processes to access the file
    FileExt::unlock(&file)?;
    Ok(())
}
