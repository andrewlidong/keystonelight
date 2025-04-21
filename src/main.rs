// Import standard library modules for command-line argument handling
use std::env;

// Import file system modules for file operations
use std::fs::{File, OpenOptions};

// Import I/O modules for reading, writing, and seeking in files
use std::io::{self, BufRead, BufReader, Read, Write};

// Import Unix-specific file operations for setting file permissions
use std::os::unix::fs::OpenOptionsExt;

// Import thread utilities for concurrent operations
use std::thread;

// Import atomic operations for thread-safe flags
use std::sync::atomic::{AtomicBool, Ordering};

// Import thread synchronization primitives for safe concurrent access
use std::sync::{Arc, Mutex, RwLock};

// Import thread-safe hash map for storing key-value pairs
use std::collections::HashMap;

// Import TCP networking types for client-server communication
use std::net::{TcpListener, TcpStream};

// Import signal handling utilities
use signal_hook::consts::signal::SIGUSR1 as SIGUSR1_HOOK;
use signal_hook::flag;

// Import lazy static initialization
use lazy_static::lazy_static;

// Import Ctrl-C handling
use ctrlc;

// Import process and fork-related modules
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{fork, ForkResult, Pid};
use std::process::exit;

// Import file locking
use fs2::FileExt;

// Define constants for file paths and server configuration
const DB_PATH: &str = "db.txt";
const SERVER_ADDR: &str = "127.0.0.1:7878";
const PID_FILE: &str = "keystonelight.pid";
const CACHE_FILE: &str = "cache.txt";

// Define the number of worker threads to handle concurrent client connections
const NUM_WORKERS: usize = 4;

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
        let db = Self {
            data: RwLock::new(HashMap::new()),
            cache: RwLock::new(HashMap::new()),
        };

        // Load cache if it exists
        if let Ok(file) = File::open(CACHE_FILE) {
            let reader = BufReader::new(file);
            let mut cache = db.cache.write().unwrap();

            for line in reader.lines().map_while(Result::ok) {
                if let Some((k, v)) = line.split_once('|') {
                    cache.insert(k.to_string(), v.to_string());
                }
            }
        }

        db
    }

    // Load existing data from the database file into memory
    fn load_from_file(&self) -> io::Result<()> {
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
            .create(true) // Create file if it doesn't exist
            .write(true) // Enable write access
            .truncate(true) // Clear existing content
            .mode(0o600) // Set Unix permissions (owner read/write only)
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
            return Some(value.clone());
        }
        drop(cache);

        // If not in cache, check the main data store
        let data = self.data.read().unwrap();
        if let Some(value) = data.get(key) {
            // Update cache with the found value
            let mut cache = self.cache.write().unwrap();
            cache.insert(key.to_string(), value.clone());
            drop(cache);
            self.save_cache().ok(); // Ignore errors for cache updates
            Some(value.clone())
        } else {
            None
        }
    }

    // Set a key-value pair in both main storage and cache
    fn set(&self, key: &str, value: &str) -> std::io::Result<()> {
        {
            let mut data = self.data.write().unwrap();
            let mut cache = self.cache.write().unwrap();

            data.insert(key.to_string(), value.to_string());
            cache.insert(key.to_string(), value.to_string());
        }

        // Save both main data and cache
        self.save_to_file()?;
        self.save_cache().ok(); // Ignore cache save errors
        Ok(())
    }

    // Delete multiple keys using child processes
    fn delete_with_children(&self, keys: &[String]) -> io::Result<()> {
        let mut child_pids = Vec::new();

        // Create a file lock to coordinate between processes
        let lock_file = OpenOptions::new()
            .create(true)
            .write(true)
            .open("db.lock")?;

        // Fork a child process for each key
        for key in keys {
            match unsafe { fork() } {
                Ok(ForkResult::Parent { child }) => {
                    println!("Started child process {} for key '{}'", child, key);
                    child_pids.push((child, key.clone()));
                }
                Ok(ForkResult::Child) => {
                    // Child process: delete the key
                    let result = (|| {
                        // Acquire exclusive lock
                        lock_file.lock_exclusive()?;

                        let mut data = self.data.write().unwrap();
                        let mut cache = self.cache.write().unwrap();

                        let found = data.remove(key).is_some();
                        if found {
                            cache.remove(key);
                            drop(data);
                            drop(cache);
                            self.save_to_file()?;
                            self.save_cache().ok();
                        }

                        // Release lock
                        lock_file.unlock()?;

                        Ok::<bool, io::Error>(found)
                    })();

                    // Exit with appropriate status
                    match result {
                        Ok(true) => exit(0),  // Success
                        Ok(false) => exit(1), // Key not found
                        Err(_) => exit(2),    // Error
                    }
                }
                Err(e) => {
                    eprintln!("Fork failed for key '{}': {}", key, e);
                }
            }
        }

        // Parent: wait for all children to complete
        for (pid, key) in child_pids {
            match waitpid(pid, None) {
                Ok(WaitStatus::Exited(_, status)) => match status {
                    0 => println!("Successfully deleted key '{}' (pid: {})", key, pid),
                    1 => println!("Key '{}' not found (pid: {})", key, pid),
                    2 => println!("Error deleting key '{}' (pid: {})", key, pid),
                    _ => println!("Unknown status for key '{}' (pid: {})", key, pid),
                },
                Ok(status) => println!("Child process for key '{}' terminated: {:?}", key, status),
                Err(e) => eprintln!("Error waiting for child process: {}", e),
            }
        }

        // Clean up lock file
        std::fs::remove_file("db.lock").ok();
        Ok(())
    }

    // Compact the database file by removing deleted entries and reorganizing data
    fn compact(&self) {
        let _lock = DB_MUTEX.lock().unwrap();
        let temp_path = format!("{}.tmp", DB_PATH);

        let data = self.data.read().unwrap();

        let result = (|| -> io::Result<()> {
            let mut temp_file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .mode(0o600)
                .open(&temp_path)?;

            let mut entries: Vec<_> = data.iter().collect();
            entries.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));

            for (k, v) in entries {
                writeln!(temp_file, "{}|{}", k, v)?;
            }

            temp_file.sync_all()?;
            std::fs::rename(&temp_path, DB_PATH)?;

            // Update cache after successful compaction
            let mut cache = self.cache.write().unwrap();
            cache.clear();
            for (k, v) in data.iter() {
                cache.insert(k.clone(), v.clone());
            }
            drop(cache);
            self.save_cache()?;

            Ok(())
        })();

        match result {
            Ok(_) => println!("Database compaction completed successfully"),
            Err(e) => {
                eprintln!("Error during compaction: {}", e);
                let _ = std::fs::remove_file(&temp_path);
            }
        }
    }

    fn save_cache(&self) -> io::Result<()> {
        let cache = self.cache.read().unwrap();
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(CACHE_FILE)?;

        for (k, v) in cache.iter() {
            writeln!(file, "{}|{}", k, v)?;
        }
        file.sync_all()?;
        Ok(())
    }
}

// Global mutex for database-wide operations
lazy_static! {
    static ref DB_MUTEX: Mutex<()> = Mutex::new(());
}

// Main entry point of the program
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} [serve|get|set|delete] [key] [value?]", args[0]);
        return;
    }

    let db = Arc::new(Database::new());

    if let Err(e) = db.load_from_file() {
        eprintln!("Error loading database: {}", e);
        return;
    }

    match args[1].as_str() {
        "serve" => {
            serve(db);
        }
        "set" => {
            if args.len() != 4 {
                eprintln!("Usage: {} set <key> <value>", args[0]);
                return;
            }
            let key = &args[2];
            let value = &args[3];
            if let Err(e) = db.set(key, value) {
                eprintln!("Error setting value: {}", e);
            }
        }
        "get" => {
            if args.len() != 3 {
                eprintln!("Usage: {} get <key>", args[0]);
                return;
            }
            let key = &args[2];
            match db.get(key) {
                Some(value) => println!("{}", value),
                None => println!("Key not found"),
            }
        }
        "delete" => {
            if args.len() < 3 {
                eprintln!("Usage: {} delete <key1> [key2 ...]", args[0]);
                return;
            }
            let keys: Vec<String> = args[2..].iter().map(|s| s.to_string()).collect();
            if let Err(e) = db.delete_with_children(&keys) {
                eprintln!("Error during deletion: {}", e);
            }
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            eprintln!("Usage: {} [serve|get|set|delete] [key] [value?]", args[0]);
        }
    }
}

// Start the database server and manage worker threads
fn serve(db: Arc<Database>) {
    // Write PID to file for external management
    let pid = std::process::id().to_string();
    std::fs::write(PID_FILE, pid).expect("Failed to write PID file");

    // Set up compaction signal handler
    let compact_flag = Arc::new(AtomicBool::new(false));
    flag::register(SIGUSR1_HOOK, Arc::clone(&compact_flag))
        .expect("Failed to register signal handler");

    // Set up Ctrl-C handler for graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // Create thread pool
    let mut handles = Vec::with_capacity(NUM_WORKERS);
    let listener = TcpListener::bind(SERVER_ADDR).expect("Failed to bind to address");
    println!("Server listening on {}", SERVER_ADDR);

    // Spawn worker threads
    for _ in 0..NUM_WORKERS {
        let db = Arc::clone(&db);
        let listener = listener.try_clone().expect("Failed to clone listener");
        let running = Arc::clone(&running);
        let compact_flag = Arc::clone(&compact_flag);

        let handle = thread::spawn(move || {
            for stream in listener.incoming() {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                // Check if compaction was requested
                if compact_flag.load(Ordering::Relaxed) {
                    db.compact();
                    compact_flag.store(false, Ordering::Relaxed);
                }

                match stream {
                    Ok(stream) => {
                        println!("New connection from: {}", stream.peer_addr().unwrap());
                        if let Err(e) = handle_client(stream, Arc::clone(&db)) {
                            eprintln!("Error handling client: {}", e);
                        }
                    }
                    Err(e) => eprintln!("Error accepting connection: {}", e),
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all worker threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Cleanup
    std::fs::remove_file(PID_FILE).ok();
}

// Handle client connections and process their commands
fn handle_client(mut stream: TcpStream, db: Arc<Database>) -> io::Result<()> {
    let mut buffer = [0; 1024];

    // Read from the stream
    let n = stream.read(&mut buffer)?;
    let command = String::from_utf8_lossy(&buffer[..n]);

    // Parse and execute the command
    let response = match parse_command(&command) {
        Ok((cmd, key, value)) => match cmd.as_str() {
            "get" => match db.get(&key) {
                Some(val) => format!("OK {}", val),
                None => String::from("ERROR Key not found"),
            },
            "set" => {
                if let Some(val) = value {
                    if let Err(e) = db.set(&key, &val) {
                        format!("ERROR {}", e)
                    } else {
                        String::from("OK")
                    }
                } else {
                    String::from("ERROR Missing value")
                }
            }
            "delete" => match db.delete_with_children(&[key.to_string()]) {
                Ok(_) => String::from("OK"),
                Err(e) => format!("ERROR {}", e),
            },
            _ => String::from("ERROR Unknown command"),
        },
        Err(e) => format!("ERROR {}", e),
    };

    // Write the response back to the client
    writeln!(stream, "{}", response)?;
    stream.flush()?;

    Ok(())
}

// Parse a command string into (command, key, optional_value)
fn parse_command(line: &str) -> Result<(String, String, Option<String>), io::Error> {
    // Split command line into whitespace-separated parts
    let parts: Vec<&str> = line.trim().split_whitespace().collect();

    // Return error if command is empty
    if parts.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Empty command"));
    }

    // Convert command to lowercase for case-insensitive matching
    let cmd = parts[0].to_lowercase();

    // Process command based on its type
    match cmd.as_str() {
        // Handle get command format: get <key>
        "get" => {
            if parts.len() != 2 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Usage: get <key>",
                ));
            }
            Ok((cmd, parts[1].to_string(), None))
        }
        // Handle set command format: set <key> <value>
        "set" => {
            if parts.len() < 3 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Usage: set <key> <value>",
                ));
            }
            // Join remaining parts as value to support spaces in values
            let value = parts[2..].join(" ");
            Ok((cmd, parts[1].to_string(), Some(value)))
        }
        // Handle delete command format: delete <key>
        "delete" => {
            if parts.len() != 2 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Usage: delete <key>",
                ));
            }
            Ok((cmd, parts[1].to_string(), None))
        }
        // Return error for unknown commands
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Unknown command",
        )),
    }
}
