//! KeystoneLight is a lightweight key-value store with support for multiple data types,
//! nested object access, persistence to disk, and a JSON-compatible format.
//!
//! # Features
//!
//! - Support for multiple data types (strings, numbers, booleans, null, arrays, objects)
//! - Nested object access using dot notation
//! - Persistence to disk in JSON format
//! - Command-line interface
//! - Error handling with custom error types
//!
//! # Examples
//!
//! ```rust
//! use keystonelight::{Store, Value};
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
//! store.save("data.db").unwrap();
//!
//! // Load from disk
//! let loaded_store = Store::load("data.db").unwrap();
//! ```
//!
//! # Architecture
//!
//! The library is organized into several modules:
//! - `Store`: Core storage engine with CRUD operations
//! - `Value`: Enum representing different value types
//! - `StoreError`: Custom error types
//! - `cli`: Command-line interface utilities
//!
//! # Error Handling
//!
//! The library uses custom error types for handling I/O and serialization errors.
//! All operations that can fail return a `Result<T, StoreError>`.
//!
//! # Thread Safety
//!
//! The current implementation is not thread-safe. If you need to share the store
//! between threads, you should wrap it in a synchronization primitive like `Mutex`.

pub mod cli;
pub mod error;
pub mod store;
pub mod value;

pub use error::StoreError;
pub use store::Store;
pub use value::Value;
