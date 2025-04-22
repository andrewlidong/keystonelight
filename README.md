# KeystoneLight

A lightweight, concurrent key-value database written in Rust, featuring in-memory storage with file persistence and proper Unix service behavior.

## Features

### Core Functionality
- In-memory key-value storage with persistent file backup
- Thread-safe concurrent operations using RwLock
- Multi-threaded server with configurable thread pool (specify number of worker threads)
- TCP-based client-server communication
- File-based storage with immediate persistence
- Case-insensitive command handling
- Interactive client with command history and help system

### Data Operations
- `SET <key> <value>`: Store key-value pairs with immediate persistence
- `GET <key>`: Retrieve values with cache-first lookup
- `DELETE <key>`: Remove entries with atomic operations
- `COMPACT`: Trigger log compaction to optimize storage
- Support for binary data (automatically base64 encoded/decoded)

### Performance & Safety
- Thread pool for handling concurrent client connections
- Graceful thread pool shutdown on server termination
- In-memory cache layer for O(1) read operations
- Atomic file operations for data integrity
- Proper lock management to prevent deadlocks
- Automatic log compaction when log size exceeds 1MB
- Base64 encoding for binary data support
- Comprehensive error handling and recovery

### System Features
- Process ID tracking for external management
- Graceful shutdown handling (SIGTERM, SIGINT)
- Error recovery and cleanup
- File-system based persistence
- Proper Unix service behavior
- Stale PID file cleanup on startup
- File locking to prevent multiple instances

### Documentation
- Comprehensive documentation tests for all public APIs
- Inline code examples in documentation
- Detailed module and function documentation
- Usage examples for all major components
- Thread pool configuration and management examples
- Client-server interaction examples
- Binary data handling examples
- [Technical Blog Post](BLOG.md) - A detailed walkthrough of the project's implementation and design decisions

## Development Setup

### Prerequisites
- Rust (latest stable version)
- Cargo (comes with Rust)
- Unix-like environment (Linux/macOS)

### Installation

```bash
# Clone the repository
git clone https://github.com/andrewlidong/keystonelight.git
cd keystonelight

# Build the project
cargo build
```

### Development Tools
- `cargo fix`: Auto-fix common code issues
- `cargo fmt`: Format code according to Rust style guidelines
- `cargo clippy`: Additional linting checks
- `cargo test`: Run all tests including integration tests
- `cargo test --doc`: Run documentation tests to verify code examples

## Usage

### Starting the Server

```bash
# Start server with default thread pool (4 threads)
cargo run --bin database serve

# Start server with custom thread pool size
cargo run --bin database serve 8  # Uses 8 worker threads
```

Expected output:
```
Creating new log file at keystonelight.log
Log file opened and locked successfully
Replaying log file
Replay complete, found 0 entries
Server listening on 127.0.0.1:7878
```

If the server is already running, you'll see:
```
Server error: Server already running with PID <pid>
```

To start a fresh server:
1. Kill any existing server processes:
   ```bash
   pkill -9 -f "target/debug/database"
   ```
2. Clean up any stale files:
   ```bash
   rm -f keystonelight.pid keystonelight.log
   ```
3. Start the server again with desired thread count:
   ```bash
   cargo run --bin database serve 4  # or any number of threads
   ```

### Performance Tuning

The server's performance can be optimized by adjusting the number of worker threads in the thread pool. Consider the following when choosing the thread count:

- For CPU-bound workloads: Set thread count to the number of CPU cores
- For I/O-bound workloads: You might benefit from more threads than CPU cores
- For mixed workloads: Start with thread count = CPU cores + 1
- For high-concurrency scenarios: Monitor system resources and adjust accordingly

The optimal thread count depends on:
- Your hardware (CPU cores, memory)
- Workload characteristics (read/write ratio, operation size)
- Concurrent client connections
- System resources availability

### Client Operations

Start an interactive session with the database:
```bash
cargo run --bin client
```

The client provides an interactive prompt with the following features:
- Command history (up/down arrows)
- Help system (type 'help' for usage)
- Binary data support (automatically handled)
- Clear error messages
- Command completion

Example session:
```
> help
Available commands:
  SET <key> <value>  - Set a key-value pair
  GET <key>         - Get the value for a key
  DELETE <key>      - Delete a key-value pair
  COMPACT           - Trigger log compaction
  quit/exit         - Exit the client

> SET test_key test_value
OK
> GET test_key
test_value
> DELETE test_key
OK
> COMPACT
OK
> exit
Goodbye!
```

### Binary Data Support

The database supports storing binary data. When using the client, binary data is automatically base64 encoded/decoded:

```bash
> SET binary_key \x00\x01\x02\x03
OK
> GET binary_key
\x00\x01\x02\x03
```

## Architecture

### Server
- Multi-threaded TCP server
- Configurable thread pool for connection handling
- Dynamic worker thread allocation
- Connection queuing and load balancing
- In-memory storage with file persistence
- Log-based storage system
- Automatic compaction

### Client
- Interactive command-line interface
- TCP-based communication
- Binary data support
- Error handling and recovery
- Command history and help system

### Storage
- Log-structured storage
- Immediate persistence
- Atomic operations
- Automatic compaction
- File locking for safety
- Two-tier storage system:
  - In-memory cache (HashMap) for fast reads
  - On-disk log file for persistence
- Cache consistency:
  - Cache rebuilt from log on startup
  - Synchronous updates to cache and log
  - Cache-first reads for optimal performance
  - Automatic cache cleanup during compaction

### Performance Characteristics
- Read Operations (GET):
  - O(1) time complexity using in-memory cache
  - No disk I/O required for cache hits
- Write Operations (SET):
  - O(1) cache update
  - O(1) log append
  - Synchronous disk write for durability
- Delete Operations:
  - O(1) cache removal
  - O(1) log append
  - Synchronous disk write for durability
- Compaction:
  - O(n) where n is the number of unique keys
  - Rebuilds log file from cache state
  - Automatic when log size exceeds 1MB