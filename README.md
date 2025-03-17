# KeystoneLight

A lightweight key-value store written in Rust that supports nested objects and multiple data types.

## Features

- **Multiple Data Types**: Support for strings, integers, floats, booleans, arrays, and nested objects
- **Nested Access**: Access nested values using dot notation (e.g., `user.address.city`)
- **Persistence**: Automatic persistence to disk
- **JSON Compatible**: Store and retrieve JSON-formatted data
- **Type Safety**: Strong type checking and validation
- **Simple CLI**: Easy-to-use command-line interface

## Installation

```bash
cargo install keystonelight
```

## Usage

Start the KeystoneLight CLI:

```bash
keystonelight
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
SET name "John Doe"

# Store a number
SET age 30

# Store a complex object
SET user {"name": "John", "age": 30, "address": {"city": "New York"}}

# Access nested values
GET user.address.city

# List all stored values
LIST

# Delete a value
DELETE name
```

## Data Types

KeystoneLight supports the following data types:

- Strings: `"Hello, World!"`
- Integers: `42`
- Floats: `3.14159`
- Booleans: `true`, `false`
- Null: `null`
- Arrays: `[1, "two", true]`
- Objects: `{"name": "John", "age": 30}`

## Error Handling

KeystoneLight provides clear error messages for:
- Invalid JSON syntax
- Missing keys
- Invalid data types
- File system errors

## License

This project is licensed under the MIT License - see the LICENSE file for details. 