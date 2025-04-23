# KeystoneLight
## _Smooth, Never Bitter_

A lightweight, concurrent key-value database written in Rust, featuring in-memory storage with file persistence and proper Unix service behavior.

## Features

### Core Functionality
- In-memory key-value storage with persistent file backup
- Thread-safe concurrent operations using RwLock
- Multi-threaded server with configurable thread pool
- TCP-based client-server communication
- File-based storage with immediate persistence
- Case-insensitive command handling
- Interactive client with command history and help system
- Docker support with persistent storage

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
- Docker containerization with volume support

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
- Docker and Docker Compose (for containerized deployment)

### Installation

#### Local Installation
```bash
# Clone the repository
git clone https://github.com/andrewlidong/keystonelight.git
cd keystonelight

# Build the project
cargo build
```

#### Docker Installation
```bash
# Clone the repository
git clone https://github.com/andrewlidong/keystonelight.git
cd keystonelight

# Build and start the container
docker-compose up -d
```

### Development Tools
- `cargo fix`: Auto-fix common code issues
- `cargo fmt`: Format code according to Rust style guidelines
- `cargo clippy`: Additional linting checks
- `cargo test`: Run all tests including integration tests
- `cargo test --doc`: Run documentation tests to verify code examples

## Testing

The project includes a comprehensive test suite with the following components:

1. **Unit Tests**:
   - Core functionality tests
   - Data structure tests
   - Error handling tests

2. **Integration Tests**:
   - Client-server interaction tests
   - Data persistence tests
   - Log compaction tests
   - Binary data handling tests

3. **Stress Tests**:
   - Concurrent operation tests
   - Resource usage tests
   - Error injection tests

### Local Testing
```bash
# Run all tests
cargo test --all --verbose

# Run specific test categories
cargo test --test integration_tests  # Integration tests
cargo test --test stress_tests      # Stress tests
```

### Docker Testing

The project includes Docker-based testing for consistent test environments:

```bash
# Run regular tests in Docker
docker-compose run --rm test

# Run stress tests in Docker
docker-compose run --rm stress-test
```

The Docker test environment provides:
- Isolated test execution
- Consistent dependencies
- Proper cleanup after tests
- Support for both regular and stress tests
- All necessary build and test dependencies
- Source code mounted as a volume for live updates
- Environment variables for better test output
- Single-threaded test execution by default

## Usage

### Local Usage

#### Starting the Server
```bash
# Start server with default thread pool (4 threads)
cargo run --bin database serve

# Start server with custom thread pool size
cargo run --bin database serve 8  # Uses 8 worker threads
```

#### Client Operations
```bash
# Start an interactive client session
cargo run --bin client
```

### Docker Usage

#### Starting the Service
```bash
# Start the containerized service
docker-compose up -d

# View logs
docker-compose logs -f

# Connect to the database using the client
docker-compose exec keystonelight keystonelight-client

# Stop the service
docker-compose down
```

The Docker version provides:
- Persistent storage using Docker volumes
- Automatic container restart
- Health monitoring
- Process isolation
- Port mapping (7878)
- Non-root user execution

#### Docker Environment Variables
- `RUST_LOG`: Set logging level (default: info)
- `THREAD_COUNT`: Number of worker threads (default: 4)

#### Docker Volume
Data is persisted in the `keystonelight_data` volume. To backup or migrate data:
```bash
# Backup volume
docker run --rm -v keystonelight_keystonelight_data:/source -v $(pwd):/backup alpine tar -czf /backup/keystonelight-backup.tar.gz -C /source .

# Restore volume
docker run --rm -v keystonelight_keystonelight_data:/target -v $(pwd):/backup alpine sh -c "rm -rf /target/* && tar -xzf /backup/keystonelight-backup.tar.gz -C /target"
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
# Local
cargo run --bin client

# Docker
docker-compose exec keystonelight keystonelight-client
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
> SET binary_key \x00\x01\x02\x03
OK
> GET binary_key
\x00\x01\x02\x03
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

### Continuous Integration

The project uses GitHub Actions for continuous integration. The CI pipeline includes:

1. **Testing**:
   - Runs on Ubuntu
   - Uses stable Rust toolchain
   - Comprehensive test suite execution
   - Dependency caching for faster builds

2. **Code Quality Checks**:
   - Rustfmt for consistent code formatting
   - Clippy for linting and catching common mistakes
   - Build verification

3. **Docker Integration**:
   - Automated Docker image building
   - Basic container verification
   - Version compatibility check

The CI pipeline runs automatically on:
- Every push to the main branch
- Every pull request targeting the main branch

The pipeline ensures:
- All code changes are properly tested
- Code style and quality standards are maintained
- The project builds correctly
- Docker configuration remains functional

