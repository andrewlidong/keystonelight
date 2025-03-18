# KeystoneLight

A lightweight key-value store written in Rust.

## Features

- Multiple data types (strings, numbers, booleans, arrays, objects)
- Nested object access with dot notation
- Persistent storage
- Simple CLI interface

## Quick Start

```bash
# Build and run
cargo run

# Example commands
> SET name "John Doe"
> SET user {"name": "Alice", "age": 30}
> GET user.name
> LIST
> DELETE name
```

## Development

```bash
# Run tests
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy
```

## License

MIT License - See [LICENSE](LICENSE) for details. 