// Import necessary modules from the Rust standard library
use std::env; // for handling command line arguments
use std::fs::{File, OpenOptions}; // for file operations
use std::io::{BufRead, BufReader, Write}; // for reading and writing files
use std::os::unix::fs::OpenOptionsExt; // for unix-specific file operations
use std::path::Path; // for handling file paths
use std::{thread, time}; // for thread operations and time handling

// import the FileExt trait which provides file locking functionality
use fs2::FileExt;

// Define a constant for our database file path
// The & in `&str` indicates that DB_PATH is a string reference (or "string slice")
// rather than an owned String. It's more efficient for constant string literals
// since they are stored in the program's read-only memory and never need to be freed
const DB_PATH: &str = "db.txt";

// main function - the entry point of the program
fn main() {
    // collect command line arguments into a vector
    let args: Vec<String> = env::args().collect();
    
    // check if we have enough arguments (program name + command)
    if args.len() < 2 {
        eprintln!("Usage: {} [get|set] [key] [value?]", args[0]);
        return;
    }

    match args[1].as_str() {
        "set" => {
            // for 'set' commmand, we need exactly 4 arguments
            if args.len() != 4 {
                eprintln!("Usage: {} set <key> <value>", args[0]);
                return;
            }
            // extract key and value from arguments
            // The & operator creates references to elements in args
            // This avoids copying the strings and instead borrows them
            // The set() function takes string references as parameters
            let key = &args[2];   // & creates a reference to args[2]
            let value = &args[3]; // & creates a reference to args[3]
            // call set function and handle any errors
            if let Err(e) = set(key, value) {
                eprintln!("Error setting value: {}", e);
            }
        }
        "get" => {
            // for 'get' command, we need exactly 3 arguments
            if args.len() != 3 {
                eprintln!("Usage: {} get <key>", args[0]);
                return;
            }
            // extract key from arguments
            let key = &args[2];
            // call get function and handle the result
            match get(key) {
                Some(value) => println!("{}", value),
                None => println!("Key not found"),
            }
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
        }
    }
}

// Helper function to open or create the database file
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
            .open(DB_PATH)
    }
}

fn set(key: &str, value: &str) -> std::io::Result<()> {
    // An exclusive (write) lock prevents any other process from acquiring either
    // a shared or exclusive lock on the file. This ensures only one process
    // can write to the file at a time.
    //
    // Advisory locks are cooperative - they only work if all processes agree to
    // check for locks before accessing the file. They don't prevent raw file I/O
    // operations from processes that ignore the locks.
    let mut file = open_or_create_db()?;
    FileExt::lock_exclusive(&file)?; // Get exclusive lock for writing
    println!("Holding lock... (sleeping 5 seconds)");
    thread::sleep(time::Duration::from_secs(5)); //simulate contention

    writeln!(file, "{}|{}", key, value)?;
    FileExt::unlock(&file)?;
    println!("Lock released.");
    Ok(())
}

fn get(key: &str) -> Option<String> {
    let file = open_or_create_db().ok()?;
    FileExt::lock_shared(&file).ok()?; // shared lock

    let reader = BufReader::new(&file);

    let mut result = None;
    for line in reader.lines().flatten() {
        if let Some((k, v)) = line.split_once('|') {
            if k == key {
                result = Some(v.to_string());
            }
        }
    }

    // We need to release the shared lock to allow other processes to acquire locks
    // If we don't release it, the lock would be held until the file handle is dropped,
    // potentially blocking other processes unnecessarily after we're done reading
    FileExt::unlock(&file).ok()?;
    result
}
