# KeystoneLight Database

A lightweight, concurrent key-value database written in Rust, featuring in-memory storage with file persistence and proper Unix service behavior.

## Features

### Core Functionality
- In-memory key-value storage with persistent file backup
- Thread-safe concurrent operations using RwLock
- Multi-threaded server with configurable worker threads
- TCP-based client-server communication
- File-based storage with immediate persistence
- Case-insensitive command handling

### Data Operations
- `get`: Retrieve values with cache-first lookup
- `set`: Store key-value pairs with immediate persistence
- `delete`: Remove entries with atomic operations
- `compact`: Trigger log compaction to optimize storage

### Performance & Safety
- Thread pool for handling concurrent client connections
- Cache layer for faster read operations
- Atomic file operations for data integrity
- Proper lock management to prevent deadlocks
- Automatic log compaction when log size exceeds 1MB
- Base64 encoding for binary data support

### System Features
- Process ID tracking for external management
- Graceful shutdown handling (SIGTERM, SIGINT)
- Error recovery and cleanup
- File-system based persistence
- Proper Unix service behavior
- Stale PID file cleanup on startup
- File locking to prevent multiple instances

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

## Usage

### Starting the Server

```bash
cargo run --bin database serve
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
3. Start the server again:
   ```bash
   cargo run --bin database serve
   ```

### Client Operations

Start an interactive session with the database:
```bash
cargo run --bin client
```

Example session:
```
Connected to database server at 127.0.0.1:7878
Enter commands (type 'help' for usage, 'quit' to exit):
> help
Available commands:
  SET <key> <value>  - Set a key-value pair
  GET <key>         - Get the value for a key
  DELETE <key>      - Delete a key-value pair
  COMPACT           - Trigger log compaction
  quit/exit         - Exit the client

> set andrew dong
OK
> get andrew
dong
> delete andrew
OK
> get andrew
NOT_FOUND
> compact
OK
> quit
Goodbye!
```

### Command Examples

Commands are case-insensitive:
```bash
SET key value
set key value
SeT key value
```

Values with spaces:
```bash
SET key "value with spaces"
SET key value\swith\sspaces
```

Binary data (automatically base64 encoded):
```bash
SET binary_key <binary_data>
```

### Signal Handling

The server handles the following signals:
- SIGTERM: Graceful shutdown
- SIGINT: Graceful shutdown (Ctrl+C)

On receiving these signals, the server will:
1. Stop accepting new connections
2. Complete any in-progress operations
3. Clean up resources (PID file, log file locks)
4. Exit gracefully

### Log Compaction

The server automatically triggers log compaction when:
- The log file size exceeds 1MB
- The `compact` command is issued

Compaction:
1. Creates a new log file
2. Replays all valid operations
3. Removes deleted keys and overwritten values
4. Swaps the new log file with the old one
5. Releases the old log file

## Architecture

### Server
- Multi-threaded TCP server
- Thread pool for handling client connections
- Signal handling for graceful shutdown
- PID file management
- File locking for single instance

### Storage
- In-memory cache with RwLock
- Write-ahead logging for persistence
- Automatic log compaction
- Base64 encoding for binary data

### Client
- Interactive command-line interface
- Case-insensitive command parsing
- Support for spaces in values
- Binary data handling

## Error Handling

The server implements comprehensive error handling:
- Connection errors
- File system errors
- Lock acquisition failures
- Invalid command parsing
- Resource cleanup on errors

## License

MIT License - See LICENSE file for details