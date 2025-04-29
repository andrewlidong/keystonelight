# KeystoneLight
## _Smooth, Never Bitter_

A lightweight, concurrent key-value database written in Rust. It features in-memory storage with file persistence and proper Unix service behavior.

## Quick Start

### Local Installation
```bash
git clone https://github.com/andrewlidong/keystonelight.git
cd keystonelight
cargo build
```

### Docker Installation
```bash
git clone https://github.com/andrewlidong/keystonelight.git
cd keystonelight
docker-compose up -d
```

## Features

- In-memory key-value storage with file persistence
- Thread-safe concurrent operations
- Multi-threaded server with configurable thread pool
- TCP-based client-server communication
- Case-insensitive command handling
- Interactive client with command history
- Docker support with persistent storage

## Basic Usage

### Server
```bash
# Start server (default: 4 threads)
cargo run --bin database serve

# Start server with custom thread count
cargo run --bin database serve 8
```

### Client
```bash
# Start interactive client
cargo run --bin client
```

### Available Commands
- `SET <key> <value>`: Store a key-value pair
- `GET <key>`: Retrieve a value
- `DELETE <key>`: Remove a key-value pair
- `COMPACT`: Trigger log compaction

## Development

### Testing
```bash
# Run all tests
cargo test

# Run specific test categories
cargo test --test integration_tests
cargo test --test stress_tests
```

### Docker Testing
```bash
docker-compose run --rm test
```