//! KeystoneLight - A lightweight key-value store with support for nested objects and multiple data types
//!
//! KeystoneLight is a flexible key-value store that supports:
//! - Multiple data types (strings, integers, floats, booleans, arrays, objects)
//! - Nested object access using dot notation
//! - Persistence to disk
//! - JSON-compatible data format
//!
//! # Example
//! ```rust
//! use keystonelight::{Store, Value};
//! use std::path::Path;
//!
//! let mut store = Store::new();
//! store.set("name".to_string(), Value::String("John".to_string()));
//! assert_eq!(store.get("name").unwrap(), &Value::String("John".to_string()));
//! ```

pub mod error;
pub mod value;
pub mod store;
pub mod cli;

pub use error::StoreError;
pub use value::Value;
pub use store::Store; 