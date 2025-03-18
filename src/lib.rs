//! KeystoneLight - A lightweight key-value store with support for nested objects and multiple data types
//!
//! KeystoneLight is a flexible key-value store that supports:
//! - Multiple data types (strings, integers, floats, booleans, arrays, objects)
//! - Nested object access using dot notation
//! - Persistence to disk
//! - JSON-compatible data format
//!
//! # Examples
//!
//! ```rust
//! use keystonelight::{Store, Value};
//! use std::path::Path;
//!
//! // Create a new store
//! let mut store = Store::new();
//!
//! // Store simple values
//! store.set("name".to_string(), Value::String("John Doe".to_string()));
//! store.set("age".to_string(), Value::Integer(30));
//!
//! // Store nested objects
//! let address = serde_json::json!({
//!     "street": "123 Main St",
//!     "city": "New York",
//!     "zip": "10001"
//! });
//! store.set("address".to_string(), serde_json::from_value(address).unwrap());
//!
//! // Access nested values
//! assert_eq!(
//!     store.get("address.city"),
//!     Some(&Value::String("New York".to_string()))
//! );
//!
//! // Save to disk
//! store.save(Path::new("store.db")).unwrap();
//!
//! // Load from disk
//! let loaded_store = Store::load(Path::new("store.db")).unwrap();
//! assert_eq!(loaded_store.get("name"), store.get("name"));
//! ```
//!
//! # Architecture
//!
//! KeystoneLight is built with a modular architecture:
//!
//! - `Store`: The main storage engine that manages key-value pairs
//! - `Value`: An enum representing different supported data types
//! - `StoreError`: Custom error types for the library
//! - `cli`: Command-line interface components
//!
//! # Error Handling
//!
//! The library uses custom error types to handle various failure cases:
//!
//! - IO errors during file operations
//! - Serialization errors for JSON data
//! - Key errors for invalid operations
//!
//! # Thread Safety
//!
//! The current implementation is not thread-safe. When using KeystoneLight in a
//! multi-threaded context, ensure proper synchronization is implemented.

pub mod cli;
pub mod error;
pub mod store;
pub mod value;

pub use error::StoreError;
pub use store::Store;
pub use value::Value; 