# KeystoneLight Database

A lightweight, concurrent key-value database written in Rust, featuring in-memory storage with file persistence and efficient compaction.

## Features

### Core Functionality
- In-memory key-value storage with persistent file backup
- Thread-safe concurrent operations using RwLock
- Multi-threaded server with configurable worker threads
- TCP-based client-server communication
- Sorted storage for efficient retrieval

### Data Operations
- `get`: Retrieve values with cache-first lookup
- `set`: Store key-value pairs with immediate persistence
- `delete`: Remove entries with atomic operations
- `compact`: Optimize storage and remove deleted entries

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

The server will:
- Start listening on `127.0.0.1:7878`
- Create a worker thread pool (default: 4 threads)
- Write its PID to `keystonelight.pid`
- Initialize the database from existing data (if any)

### Client Operations

#### Set a Value
```bash
cargo run --bin client -- set username "John Doe"
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
- Sorted storage in the persistence layer

### Concurrency Model
- Read-Write locks for safe concurrent access
- Worker thread pool for connection handling
- Global mutex for critical operations
- Atomic operations for flag management

### Data Persistence
- Immediate write-through to disk on modifications
- Atomic file operations using temporary files
- Safe compaction with backup preservation
- Proper file permissions:
  - Database file (db.txt): 0o600 (read/write for owner only)
  - Cache file (cache.txt): 0o600 (read/write for owner only)
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

#### Database Operations
```
Error: Key not found
```
The requested key does not exist in the database or cache.

```
Error: Failed to acquire lock
```
Concurrent access conflict - retry the operation.

```
Error: Failed to write to database file
Permission denied
```
Check file permissions (should be 0o600) and ownership.

```
Error: Cache file is corrupted
```
The cache file is invalid or corrupted. The server will recreate it.

```
Error: Database file is corrupted
```
The database file needs repair. Use the compact command to fix.

#### Connection Errors
```
Error: Failed to connect to server
Connection refused (os error 61)
```
Server is not running or the port is incorrect.

```
Error: Connection reset by peer
```
Server terminated the connection unexpectedly.

#### System Errors
```
Error: Failed to create PID file
Permission denied
```
Check directory permissions and ownership.

```
Error: Failed to send compaction signal
No such process
```
Server PID file is stale or server is not running.

## Performance Considerations
- Cache-first read operations
- Sorted storage for efficient retrieval
- Atomic file operations for consistency
- Worker thread pool for connection handling
- Efficient compaction strategy

## Troubleshooting

### Common Issues
1. **Server Already Running**
   ```
   Error: Address already in use
   ```
   Solution: Stop existing server or check `keystonelight.pid`

2. **Permission Denied**
   ```
   Error: Permission denied (os error 13)
   ```
   Solution: Check file permissions in the project directory

3. **Connection Refused**
   ```
   Error: Connection refused (os error 61)
   ```
   Solution: Ensure server is running (`cargo run --bin database -- serve`)

### Debug Tips
- Check server logs for connection information
- Verify PID file exists and contains correct process ID
- Use `ps` to check if server process is running
- Monitor `db.txt` for changes during operations