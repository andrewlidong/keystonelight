# KeystoneLight Database

A lightweight, concurrent key-value database written in Rust, featuring in-memory storage with file persistence and efficient compaction.

## Features

### Core Functionality
- In-memory key-value storage with persistent file backup
- Thread-safe concurrent operations using RwLock
- Multi-threaded server with configurable worker threads
- TCP-based client-server communication
- File-based storage with immediate persistence

### Data Operations
- `get`: Retrieve values with cache-first lookup
- `set`: Store key-value pairs with immediate persistence
- `delete`: Remove entries with atomic operations
- `compact`: Remove deleted entries and optimize storage

### Performance & Safety
- Thread pool for handling concurrent client connections
- Cache layer for faster read operations
- Atomic file operations for data integrity
- Proper lock management to prevent deadlocks
- Signal-based database management

### System Features
- Process ID tracking for external management
- Graceful shutdown handling
- Error recovery and cleanup
- File-system based persistence
- Database compaction for space optimization

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
cargo build --release
```

### Development Tools
- `cargo fix`: Auto-fix common code issues
- `cargo fmt`: Format code according to Rust style guidelines
- `cargo clippy`: Additional linting checks

## Usage

### Starting the Server

```bash
cargo run --bin database -- serve
```

Expected output:
```
Server listening on 127.0.0.1:7878
```

If the server is already running, you'll see:
```
thread 'main' panicked at src/main.rs:329:51:
Failed to bind to address: Os { code: 48, kind: AddrInUse, message: "Address already in use" }
```

To start a fresh server:
1. Check for existing server process:
   ```bash
   lsof -i :7878
   ```
2. Kill the existing process if found:
   ```bash
   kill <PID>
   ```
3. Remove any stale PID file:
   ```bash
   rm -f keystonelight.pid
   ```
4. Start the server again:
   ```bash
   cargo run --bin database -- serve
   ```

The server will:
- Start listening on `127.0.0.1:7878`
- Create a worker thread pool (default: 4 threads)
- Write its PID to `keystonelight.pid`
- Initialize the database from existing data (if any)

### Client Operations

#### Set a Value
```bash
cargo run --bin client -- set username "Andrew Dong"
```
Expected output:
```
OK
```

#### Get a Value
```bash
cargo run --bin client -- get username
```
Expected output:
```
John Doe
```

#### Delete a Value
```bash
cargo run --bin client -- delete username
```
Expected output:
```
OK
```

#### Compact the Database
```bash
cargo run --bin client -- compact
```
Expected output:
```
Compaction signal sent to server (PID: xxxxx)
```

### Interactive Mode

Start an interactive session with the database:
```bash
cargo run --bin client
```

Example session:
```
Connected to database server at 127.0.0.1:7878
Enter commands (type 'help' for usage, 'quit' to exit):
> set name "Alice"
OK
> get name
Alice
> delete name
OK
> quit
Goodbye!
```

## Implementation Details

### Storage Architecture
- In-memory HashMap protected by RwLock for concurrent access
- Cache layer for frequently accessed data
- File-based persistence with atomic operations
- Immediate write-through to disk on modifications

### Concurrency Model
- Read-Write locks for safe concurrent access
- Worker thread pool for connection handling
- Global mutex for critical operations
- Atomic operations for flag management

### Data Persistence
- Immediate write-through to disk on modifications
- Atomic file operations using temporary files
- Safe compaction with backup preservation
- File permissions:
  - Database file (db.txt): 0o644 (readable by all, writable by owner)
  - Cache file (cache.txt): 0o644 (readable by all, writable by owner)
  - PID file: 0o644 (readable by all, writable by owner)

### Safety Features
- Proper error handling and reporting
- Cleanup of temporary files
- Signal handling for graceful shutdown
- Lock ordering to prevent deadlocks

## File Structure
- `db.txt`: Main database file
- `keystonelight.pid`: Server process ID file
- `db.txt.tmp`: Temporary file for atomic operations
- `cache.txt`: Cache storage (if enabled)

## Error Handling

Common error messages and their meanings:

#### Server Startup Errors
```
Failed to bind to address: Os { code: 48, kind: AddrInUse, message: "Address already in use" }
```
The server port (7878) is already in use. Follow the steps in "Starting the Server" section to resolve.

#### Database Operations
```
Error: Key not found
```