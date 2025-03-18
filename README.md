# KeystoneLight 🗝️

A lightweight, modular key-value store written in Rust that supports nested objects and multiple data types.

## Features ✨

- Multiple data types support:
  - Strings
  - Integers
  - Floating-point numbers
  - Booleans
  - Arrays
  - Nested Objects
  - Null values
- Nested object access using dot notation (e.g., `user.address.city`)
- Persistent storage to disk
- JSON-compatible data format
- Command-line interface (CLI)

## Installation 🚀

### Prerequisites

- Rust 1.70 or higher
- Cargo (Rust's package manager)

### Building from source

```bash
# Clone the repository
git clone https://github.com/yourusername/keystonelight.git
cd keystonelight

# Build the project
cargo build --release

# Run the binary
cargo run --release
```

## Usage 📚

### Starting the Store

```bash
cargo run
```

### Available Commands

- `SET <key> <value>` - Store a key-value pair
- `GET <key>` - Retrieve a value by key (supports nested access with dots)
- `DELETE <key>` - Remove a key-value pair
- `LIST` - Show all key-value pairs
- `HELP` - Show help message
- `EXIT` - Exit the program

### Examples

```bash
# Store a simple string
> SET name "John Doe"

# Store a number
> SET age 30

# Store a JSON object
> SET user {"name": "John", "age": 30, "address": {"city": "New York"}}

# Access nested values
> GET user.address.city
Value: New York

# List all key-value pairs
> LIST

# Delete a key
> DELETE name
```

### Data Types

KeystoneLight supports the following data types:

1. **Strings**: `SET name "John Doe"`
2. **Integers**: `SET age 30`
3. **Floats**: `SET price 19.99`
4. **Booleans**: `SET active true`
5. **Arrays**: `SET numbers [1, 2, 3, "four"]`
6. **Objects**: `SET user {"name": "John", "age": 30}`
7. **Null**: `SET empty null`

## Architecture 🏗️

KeystoneLight is built with a modular architecture:

- `src/lib.rs` - Main library interface
- `src/value.rs` - Value type definitions and operations
- `src/store.rs` - Core storage engine
- `src/error.rs` - Error handling
- `src/cli/` - Command-line interface components
  - `mod.rs` - CLI module definition
  - `commands.rs` - Command handlers
  - `parser.rs` - Input parsing

## Development 🛠️

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

### Code Style

The project follows standard Rust code style guidelines. Please ensure your code is formatted using:

```bash
cargo fmt
```

And check for any linting issues with:

```bash
cargo clippy
```

## Contributing 🤝

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m '✨ Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License 📄

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments 🙏

- Inspired by Redis and other key-value stores
- Built with Rust and its amazing ecosystem 