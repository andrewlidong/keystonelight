//! A multi-threaded key-value database server implementation.
//!
//! This module provides a server that handles client connections and processes
//! commands to manipulate the underlying key-value store. It includes features
//! like PID file management, signal handling, and graceful shutdown.

use crate::storage::Database;
use crate::thread_pool::ThreadPool;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use libc;
use signal_hook::iterator::Signals;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

/// The address the server listens on
const SERVER_ADDR: &str = "127.0.0.1:7878";
/// Maximum time to wait for port binding
const BIND_TIMEOUT: Duration = Duration::from_secs(5);
/// Interval between port binding retries
const BIND_RETRY_INTERVAL: Duration = Duration::from_millis(100);
/// Default number of worker threads
const DEFAULT_THREAD_COUNT: usize = 4;

/// A server instance that manages client connections and processes commands.
pub struct Server {
    /// The underlying key-value store
    storage: Arc<Mutex<Database>>,
    /// The TCP listener for accepting connections
    listener: TcpListener,
    /// Flag indicating if the server should continue running
    running: Arc<AtomicBool>,
    /// Path to the PID file
    pid_file: PathBuf,
    /// Thread pool for handling client connections
    thread_pool: ThreadPool,
}

impl Server {
    fn cleanup_stale_pid_file(pid_file: &Path) -> io::Result<()> {
        if let Ok(pid_str) = fs::read_to_string(pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                if !process_exists(pid) {
                    println!("Cleaning up stale PID file from process {}", pid);
                    fs::remove_file(pid_file)?;
                }
            } else {
                // Invalid PID in file, clean it up
                fs::remove_file(pid_file)?;
            }
        }
        Ok(())
    }

    pub fn new() -> io::Result<Self> {
        Self::with_paths(
            "keystonelight.pid",
            "keystonelight.log",
            DEFAULT_THREAD_COUNT,
        )
    }

    pub fn with_paths<P1: AsRef<Path>, P2: AsRef<Path>>(
        pid_file: P1,
        log_file: P2,
        num_threads: usize,
    ) -> io::Result<Self> {
        let pid_file = pid_file.as_ref().to_path_buf();

        // Clean up any stale PID file
        Self::cleanup_stale_pid_file(&pid_file)?;

        // Check if PID file exists and process is running
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
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
        fs::write(&pid_file, format!("{}\n", pid))?;

        let storage = Arc::new(Mutex::new(Database::new()?));
        let thread_pool = ThreadPool::new(num_threads);
        let start_time = Instant::now();
        let running = Arc::new(AtomicBool::new(true));

        loop {
            match TcpListener::bind(SERVER_ADDR) {
                Ok(listener) => {
                    println!(
                        "Server listening on {} with {} worker threads",
                        SERVER_ADDR, num_threads
                    );
                    return Ok(Self {
                        storage,
                        listener,
                        running,
                        pid_file,
                        thread_pool,
                    });
                }
                Err(e) => {
                    if start_time.elapsed() >= BIND_TIMEOUT {
                        // Clean up PID file if we fail to bind
                        let _ = fs::remove_file(&pid_file);
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
        let pid_file = self.pid_file.clone();

        thread::spawn(move || {
            for sig in signals.forever() {
                match sig {
                    libc::SIGTERM | libc::SIGINT => {
                        println!("Received signal {}, shutting down...", sig);
                        // Clean up PID file before setting running to false
                        let _ = fs::remove_file(&pid_file);
                        running.store(false, Ordering::SeqCst);
                        break;
                    }
                    _ => unreachable!(),
                }
            }
        });

        // Set non-blocking mode for the listener
        self.listener.set_nonblocking(true)?;

        while self.running.load(Ordering::SeqCst) {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    let storage = Arc::clone(&self.storage);
                    self.thread_pool.execute(move || {
                        if let Err(e) = handle_client(stream, storage) {
                            eprintln!("Error handling client: {}", e);
                        }
                    });
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // No incoming connection, sleep a bit and continue
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                    break;
                }
            }
        }

        // Cleanup (in case we exit the loop without a signal)
        let _ = fs::remove_file(&self.pid_file);
        Ok(())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        // Clean up PID file when server is dropped
        let _ = fs::remove_file(&self.pid_file);
    }
}

fn process_exists(pid: u32) -> bool {
    // On Unix-like systems, sending signal 0 to a process checks if it exists
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None).is_ok()
}

fn handle_client(stream: TcpStream, storage: Arc<Mutex<Database>>) -> io::Result<()> {
    // Set non-blocking mode for the stream
    stream.set_nonblocking(false)?;

    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    while reader.read_line(&mut line)? > 0 {
        let command = line.trim();
        println!("Received raw command: '{}'", command);

        let response = match crate::protocol::parse_command(command) {
            Some(cmd) => {
                println!("Command parts: {:?}", cmd);
                match cmd {
                    crate::protocol::Command::Get(key) => {
                        let storage = storage.lock().unwrap();
                        match storage.get(&key) {
                            Some(value) => {
                                // Check if the value contains any non-printable characters
                                let is_binary = value
                                    .iter()
                                    .any(|&b| !b.is_ascii_graphic() && !b.is_ascii_whitespace());
                                if is_binary {
                                    format!("VALUE base64:{}\n", BASE64.encode(&value))
                                } else {
                                    match String::from_utf8(value.clone()) {
                                        Ok(text) => format!("VALUE {}\n", text),
                                        Err(_) => {
                                            format!("VALUE base64:{}\n", BASE64.encode(&value))
                                        }
                                    }
                                }
                            }
                            None => "NOT_FOUND\n".to_string(),
                        }
                    }
                    crate::protocol::Command::Set(key, value) => {
                        let mut storage = storage.lock().unwrap();
                        if let Err(e) = storage.set(&key, &value) {
                            format!("ERROR {}\n", e)
                        } else {
                            "OK\n".to_string()
                        }
                    }
                    crate::protocol::Command::Delete(key) => {
                        let mut storage = storage.lock().unwrap();
                        if let Err(e) = storage.delete(&key) {
                            format!("ERROR {}\n", e)
                        } else {
                            "OK\n".to_string()
                        }
                    }
                    crate::protocol::Command::Compact => {
                        let mut storage = storage.lock().unwrap();
                        if let Err(e) = storage.compact() {
                            format!("ERROR {}\n", e)
                        } else {
                            "OK\n".to_string()
                        }
                    }
                }
            }
            None => "ERROR Invalid command\n".to_string(),
        };

        writer.write_all(response.as_bytes())?;
        writer.flush()?;
        line.clear();
    }

    Ok(())
}
