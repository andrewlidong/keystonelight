# Threaded Key-Value Database

A concurrent key-value database with in-memory storage and file persistence, supporting multiple client connections.

## Features

- In-memory storage with file persistence
- Thread-safe operations using RwLock
- Multiple worker threads for handling client connections
- TCP-based client-server communication
- Support for get, set, and delete operations
- Database compaction to reclaim space from deleted entries
- Process ID file for external management
- Signal-based compaction trigger

## Building

```bash
cargo build
```

## Running the Server

Start the database server:

```bash
cargo run --bin database -- serve
```

The server will:
1. Start listening on `127.0.0.1:7878` with 4 worker threads
2. Write its process ID to `keystonelight.pid`
3. Set up a signal handler for compaction requests

## Client Operations

### Setting a Key-Value Pair

```bash
cargo run --bin client -- set <key> <value>
```

Example:
```bash
cargo run --bin client -- set name "John Doe"
cargo run --bin client -- set age 30
```

### Getting a Value

```bash
cargo run --bin client -- get <key>
```

Example:
```bash
cargo run --bin client -- get name
```

### Deleting Keys

```bash
cargo run --bin client -- delete <key1> [key2...]
```

Example:
```bash
cargo run --bin client -- delete name age
```

### Compacting the Database

To compact the database and reclaim space from deleted entries:

```bash
cargo run --bin client -- compact
```

This sends a SIGUSR1 signal to the server process to perform compaction. The server will:
1. Receive the compaction signal
2. Rewrite the database file with only active entries
3. Remove any space used by deleted entries
4. Maintain data consistency during the process

## Testing with Multiple Clients

You can test concurrent operations using multiple terminal windows:

1. Start the server in one terminal:
```bash
cargo run --bin database -- serve
```

2. In another terminal, set some initial data:
```bash
cargo run --bin client -- set name "John Doe"
cargo run --bin client -- set age 30
cargo run --bin client -- set city "New York"
```

3. In multiple other terminals, run concurrent operations:
```bash
# Terminal 1
cargo run --bin client -- get name

# Terminal 2
cargo run --bin client -- set age 31

# Terminal 3
cargo run --bin client -- delete city
```

4. Test compaction by adding and deleting temporary data:
```bash
# Add some temporary entries
cargo run --bin client -- set temp1 "value1"
cargo run --bin client -- set temp2 "value2"

# Delete them to create wasted space
cargo run --bin client -- delete temp1 temp2

# Trigger compaction
cargo run --bin client -- compact

# Verify data is still intact
cargo run --bin client -- get name
cargo run --bin client -- get age
```

## Implementation Details

### Thread Safety
- Uses RwLock for concurrent access to the in-memory data structure
- Implements file locking for safe file operations
- Worker threads handle client connections independently

### Data Persistence
- Data is stored in `db.txt` with key-value pairs separated by '|'
- Each operation is immediately persisted to disk
- Compaction removes deleted entries from the file

### Signal Handling
- Server writes its PID to `keystonelight.pid`
- Compaction is triggered via SIGUSR1 signal
- Signal handler uses atomic flag for thread-safe coordination

### Error Handling
- Graceful handling of file operations
- Proper cleanup of resources
- Informative error messages for client operations

## Network Testing

You can also test the database using a simple TCP client like `netcat`:

```bash
# Set a key-value pair
echo "set name John" | nc 127.0.0.1 7878

# Get a value
echo "get name" | nc 127.0.0.1 7878

# Delete a key
echo "delete name" | nc 127.0.0.1 7878
```

## Data Persistence

The database automatically saves data to `db.txt` after each modification. The file format is:

```
key1|value1
key2|value2
...
```

## Error Handling

The database provides clear error messages for:
- Invalid commands
- Missing arguments
- Non-existent keys
- Network errors
- File system errors

## Performance Considerations

- The database uses in-memory storage with periodic file persistence
- Multiple readers can access data simultaneously
- Write operations are exclusive and block other writers
- Each client connection runs in its own thread
- The server maintains a pool of worker threads for accepting connections 