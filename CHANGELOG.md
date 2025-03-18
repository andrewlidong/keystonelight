# Changelog

All notable changes to KeystoneLight will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial implementation of key-value store
- Support for multiple data types (strings, integers, floats, booleans, arrays, objects, null)
- Nested object access using dot notation
- Persistent storage to disk
- Command-line interface (CLI)
- Basic CRUD operations (SET, GET, DELETE, LIST)
- Comprehensive test suite
- Modular architecture with separate library and binary components

### Changed
- Refactored monolithic codebase into modular structure
- Improved error handling with custom error types
- Enhanced command parsing with better support for quoted strings and JSON

### Fixed
- Proper handling of nested object access
- Correct parsing of JSON objects and arrays
- Persistence issues with complex data types 