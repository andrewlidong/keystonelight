//! A multi-threaded key-value database server implementation.
//!
//! This module provides a server that handles client connections and processes
//! commands to manipulate the underlying key-value store. It includes features
//! like PID file management, signal handling, and graceful shutdown.

use crate::storage::Database;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use libc;
use signal_hook::iterator::Signals;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

/// The address the server listens on
const SERVER_ADDR: &str = "127.0.0.1:7878";
/// The PID file path
const PID_FILE: &str = "keystonelight.pid";
/// Maximum time to wait for port binding
const BIND_TIMEOUT: Duration = Duration::from_secs(5);
/// Interval between port binding retries
const BIND_RETRY_INTERVAL: Duration = Duration::from_millis(100);

/// A server instance that manages client connections and processes commands.
pub struct Server {
    /// The underlying key-value store
    db: Arc<Mutex<Database>>,
    /// The TCP listener for accepting connections
    listener: TcpListener,
    /// Flag indicating if the server should continue running
    running: Arc<AtomicBool>,
}

impl Server {
    fn cleanup_stale_pid_file() -> io::Result<()> {
        if let Ok(pid_str) = fs::read_to_string(PID_FILE) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                if !process_exists(pid) {
                    println!("Cleaning up stale PID file from process {}", pid);
                    fs::remove_file(PID_FILE)?;
                }
            } else {
                // Invalid PID in file, clean it up
                fs::remove_file(PID_FILE)?;
            }
        }
        Ok(())
    }

    pub fn new() -> io::Result<Self> {
        // Clean up any stale PID file
        Self::cleanup_stale_pid_file()?;

        // Check if PID file exists and process is running
        if let Ok(pid_str) = fs::read_to_string(PID_FILE) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                if process_exists(pid) {
                    return Err(io::Error::new(
                        io::ErrorKind::AddrInUse,
                        format!("Server already running with PID {}", pid),
                    ));
                }
            }
        }

        // Write PID file
        let pid = process::id();
        fs::write(PID_FILE, format!("{}\n", pid))?;

        let db = Arc::new(Mutex::new(Database::new()?));
        let start_time = Instant::now();
        let running = Arc::new(AtomicBool::new(true));

        loop {
            match TcpListener::bind(SERVER_ADDR) {
                Ok(listener) => {
                    println!("Server listening on {}", SERVER_ADDR);
                    return Ok(Self {
                        db,
                        listener,
                        running,
                    });
                }
                Err(e) => {
                    if start_time.elapsed() >= BIND_TIMEOUT {
                        // Clean up PID file if we fail to bind
                        let _ = fs::remove_file(PID_FILE);
                        return Err(io::Error::new(
                            io::ErrorKind::AddrInUse,
                            format!(
                                "Failed to bind to {} after {} seconds: {}",
                                SERVER_ADDR,
                                BIND_TIMEOUT.as_secs(),
                                e
                            ),
                        ));
                    }
                    thread::sleep(BIND_RETRY_INTERVAL);
                }
            }
        }
    }

    pub fn run(&self) -> io::Result<()> {
        // Set up signal handlers
        let mut signals = Signals::new(&[libc::SIGTERM, libc::SIGINT])?;
        let running = Arc::clone(&self.running);

        thread::spawn(move || {
            for sig in signals.forever() {
                match sig {
                    libc::SIGTERM | libc::SIGINT => {
                        println!("Received signal {}, shutting down...", sig);
                        // Clean up PID file before setting running to false
                        let _ = fs::remove_file(PID_FILE);
                        running.store(false, Ordering::SeqCst);
                        break;
                    }
                    _ => unreachable!(),
                }
            }
        });

        for stream in self.listener.incoming() {
            if !self.running.load(Ordering::SeqCst) {
                break;
            }
            match stream {
                Ok(stream) => {
                    let db = Arc::clone(&self.db);
                    thread::spawn(move || {
                        if let Err(e) = handle_client(stream, db) {
                            eprintln!("Error handling client: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }

        // Cleanup (in case we exit the loop without a signal)
        let _ = fs::remove_file(PID_FILE);
        Ok(())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        // Clean up PID file when server is dropped
        let _ = fs::remove_file(PID_FILE);
    }
}

fn process_exists(pid: u32) -> bool {
    // On Unix-like systems, sending signal 0 to a process checks if it exists
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None).is_ok()
}

fn handle_client(mut stream: TcpStream, storage: Arc<Mutex<Database>>) -> io::Result<()> {
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let command = line.trim();
    println!("Received raw command: '{}'", command);

    let mut parts = command.splitn(3, ' ');
    let cmd = parts.next().map(|s| s.to_uppercase());
    let key = parts.next().map(|s| s.to_string());
    let value = parts.next().map(|s| s.as_bytes().to_vec());

    println!(
        "Command parts: cmd={:?}, key={:?}, value={:?}",
        cmd, key, value
    );

    let response = match (cmd.as_deref(), key, value) {
        (Some("GET"), Some(key), None) => {
            let storage = storage.lock().unwrap();
            match storage.get(&key) {
                Some(value) => match String::from_utf8(value.clone()) {
                    Ok(text) => format!("OK {}", text),
                    Err(_) => format!("OK base64:{}", BASE64.encode(value)),
                },
                None => "NOT_FOUND".to_string(),
            }
        }
        (Some("SET"), Some(key), Some(value)) => {
            let storage = storage.lock().unwrap();
            storage.set(&key, &value)?;
            "OK".to_string()
        }
        (Some("DELETE"), Some(key), None) => {
            println!("Processing DELETE command for key: {}", key);
            let storage = storage.lock().unwrap();
            storage.delete(&key)?;
            println!("Successfully deleted key: {}", key);
            "OK".to_string()
        }
        (Some("COMPACT"), None, None) => {
            let storage = storage.lock().unwrap();
            storage.compact()?;
            "OK".to_string()
        }
        _ => "ERROR Invalid command".to_string(),
    };

    writeln!(stream, "{}", response)?;
    stream.flush()?;
    Ok(())
}
