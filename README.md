# Threaded Key-Value Database

A concurrent key-value database with in-memory storage and file persistence, supporting multiple client connections.

## Features

- In-memory storage with file persistence
- Thread-safe operations using RwLock
- Multiple worker threads for handling client connections
- TCP-based client-server communication
- Support for get, set, and delete operations

## Building

```bash
cargo build
```

## Running the Server

Start the database server:

```bash
cargo run -- serve
```

The server will start listening on `127.0.0.1:7878` with 4 worker threads.

## Client Operations

### Setting a Key-Value Pair

```bash
cargo run -- set <key> <value>
```

Example:
```bash
cargo run -- set name "John Doe"
cargo run -- set age 30
```

### Getting a Value

```bash
cargo run -- get <key>
```

Example:
```bash
cargo run -- get name
```

### Deleting Keys

```bash
cargo run -- delete <key1> [key2...]
```

Example:
```bash
cargo run -- delete name age
```

## Testing with Multiple Clients

You can test concurrent operations using multiple terminal windows:

1. Start the server in one terminal:
```bash
cargo run -- serve
```

2. In another terminal, set some initial data:
```bash
cargo run -- set name "John Doe"
cargo run -- set age 30
cargo run -- set city "New York"
```

3. In multiple other terminals, run concurrent operations:
```bash
# Terminal 1
cargo run -- get name

# Terminal 2
cargo run -- set age 31

# Terminal 3
cargo run -- delete city
```

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