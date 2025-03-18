//! KeystoneLight - A lightweight key-value store with support for nested objects and multiple data types
//!
//! KeystoneLight is a flexible key-value store that supports:
//! - Multiple data types (strings, integers, floats, booleans, arrays, objects)
//! - Nested object access using dot notation
//! - Persistence to disk
//! - JSON-compatible data format
//!
//! # Example
//! ```
//! SET user {"name": "John", "age": 30}
//! GET user.name  // Returns: "John"
//! ```

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write, BufRead};
use std::path::Path;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use serde_json;
use keystonelight::{Store, StoreError};
use keystonelight::cli::{
    parse_input,
    print_usage,
    handle_set,
    handle_get,
    handle_delete,
    handle_list,
};

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Key error: {0}")]
    KeyError(String),
}

impl From<serde_json::Error> for StoreError {
    fn from(err: serde_json::Error) -> Self {
        StoreError::Serialization(err.to_string())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum Value {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
    Null,
}

impl Value {
    // Get a nested value using dot notation (e.g., "user.name")
    fn get(&self, key: &str) -> Option<&Value> {
        let parts: Vec<&str> = key.split('.').collect();
        let mut current = self;

        for part in parts {
            match current {
                Value::Object(map) => {
                    current = map.get(part)?;
                }
                _ => return None,
            }
        }

        Some(current)
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, val) in arr.iter().enumerate() {
                    if i > 0 { write!(f, ", ")? }
                    write!(f, "{}", val)?;
                }
                write!(f, "]")
            }
            Value::Object(map) => {
                write!(f, "{{")?;
                for (i, (key, val)) in map.iter().enumerate() {
                    if i > 0 { write!(f, ", ")? }
                    write!(f, "\"{}\": {}", key, val)?;
                }
                write!(f, "}}")
            }
            Value::Null => write!(f, "null"),
        }
    }
}

/// Core storage engine for KeystoneLight
#[derive(Serialize, Deserialize, Debug)]
struct Store {
    data: HashMap<String, Value>,
}

impl Store {
    /// Creates a new empty KeystoneLight store
    fn new() -> Self {
        Store {
            data: HashMap::new(),
        }
    }

    /// Loads a KeystoneLight store from the specified path
    fn load(path: &Path) -> Result<Self, StoreError> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        if contents.is_empty() {
            return Ok(Store::new());
        }
        
        let store = serde_json::from_str(&contents)?;
        Ok(store)
    }

    fn save(&self, path: &Path) -> Result<(), StoreError> {
        let encoded = serde_json::to_string_pretty(self)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        file.write_all(encoded.as_bytes())?;
        Ok(())
    }

    fn set(&mut self, key: String, value: Value) {
        self.data.insert(key, value);
    }

    fn get(&self, key: &str) -> Option<&Value> {
        let parts: Vec<&str> = key.split('.').collect();
        let base_key = parts[0];
        let base_value = self.data.get(base_key)?;

        if parts.len() == 1 {
            Some(base_value)
        } else {
            base_value.get(&parts[1..].join("."))
        }
    }

    fn delete(&mut self, key: &str) -> Option<Value> {
        // For now, only support deleting top-level keys
        self.data.remove(key)
    }
}

// Helper function to parse a value from string
fn parse_value(s: &str) -> Result<Value, StoreError> {
    // If it starts with { or [, treat it as JSON and require valid JSON
    if s.starts_with('{') || s.starts_with('[') {
        return serde_json::from_str(s)
            .map_err(|e| StoreError::Serialization(format!("Invalid JSON: {}", e)));
    }

    // Try parsing as number
    if let Ok(i) = s.parse::<i64>() {
        return Ok(Value::Integer(i));
    }
    if let Ok(f) = s.parse::<f64>() {
        return Ok(Value::Float(f));
    }

    // Check for boolean and null
    match s.to_lowercase().as_str() {
        "true" => return Ok(Value::Boolean(true)),
        "false" => return Ok(Value::Boolean(false)),
        "null" => return Ok(Value::Null),
        _ => {}
    }

    // If quoted, treat as JSON string to handle escapes properly
    if (s.starts_with('"') && s.ends_with('"')) || 
       (s.starts_with('\'') && s.ends_with('\'')) {
        return serde_json::from_str(s)
            .map_err(|e| StoreError::Serialization(format!("Invalid string: {}", e)));
    }

    // Otherwise, treat as plain string
    Ok(Value::String(s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_new_store() {
        let store = Store::new();
        assert_eq!(store.data.len(), 0);
    }

    #[test]
    fn test_basic_operations() {
        let mut store = Store::new();
        
        // Test set and get
        store.set("key1".to_string(), Value::String("value1".to_string()));
        assert_eq!(store.get("key1"), Some(&Value::String("value1".to_string())));
        
        // Test overwrite
        store.set("key1".to_string(), Value::Integer(42));
        assert_eq!(store.get("key1"), Some(&Value::Integer(42)));
        
        // Test delete
        assert_eq!(store.delete("key1"), Some(Value::Integer(42)));
        assert_eq!(store.get("key1"), None);
        
        // Test delete non-existent key
        assert_eq!(store.delete("nonexistent"), None);
    }

    #[test]
    fn test_value_types() -> Result<(), StoreError> {
        let mut store = Store::new();

        // Test string with spaces
        store.set("string".to_string(), parse_value("\"Hello, World!\"")?);
        assert_eq!(store.get("string"), Some(&Value::String("Hello, World!".to_string())));

        // Test integers
        store.set("int".to_string(), parse_value("42")?);
        assert_eq!(store.get("int"), Some(&Value::Integer(42)));

        // Test negative numbers
        store.set("negative".to_string(), parse_value("-123")?);
        assert_eq!(store.get("negative"), Some(&Value::Integer(-123)));

        // Test floats
        store.set("float".to_string(), parse_value("3.14159")?);
        assert_eq!(store.get("float"), Some(&Value::Float(3.14159)));

        // Test boolean
        store.set("bool_true".to_string(), parse_value("true")?);
        store.set("bool_false".to_string(), parse_value("false")?);
        assert_eq!(store.get("bool_true"), Some(&Value::Boolean(true)));
        assert_eq!(store.get("bool_false"), Some(&Value::Boolean(false)));

        // Test null
        store.set("null_value".to_string(), parse_value("null")?);
        assert_eq!(store.get("null_value"), Some(&Value::Null));

        // Test array with mixed types
        store.set("array".to_string(), parse_value("[1, \"two\", true, null]")?);
        if let Some(Value::Array(arr)) = store.get("array") {
            assert_eq!(arr.len(), 4);
            assert_eq!(arr[0], Value::Integer(1));
            assert_eq!(arr[1], Value::String("two".to_string()));
            assert_eq!(arr[2], Value::Boolean(true));
            assert_eq!(arr[3], Value::Null);
        } else {
            panic!("Expected array value");
        }

        Ok(())
    }

    #[test]
    fn test_nested_objects() -> Result<(), StoreError> {
        let mut store = Store::new();
        
        // Test deeply nested object
        let complex_json = r#"{
            "user": {
                "name": "John Doe",
                "age": 30,
                "address": {
                    "street": "123 Main St",
                    "city": "New York",
                    "zip": "10001",
                    "coordinates": {
                        "lat": 40.7128,
                        "lng": -74.0060
                    }
                },
                "contacts": [
                    {"type": "email", "value": "john@example.com"},
                    {"type": "phone", "value": "555-0123"}
                ]
            }
        }"#;
        
        store.set("data".to_string(), parse_value(complex_json)?);

        // Test nested access
        assert_eq!(store.get("data.user.name"), Some(&Value::String("John Doe".to_string())));
        assert_eq!(store.get("data.user.age"), Some(&Value::Integer(30)));
        assert_eq!(store.get("data.user.address.city"), Some(&Value::String("New York".to_string())));
        assert_eq!(store.get("data.user.address.coordinates.lat"), Some(&Value::Float(40.7128)));

        // Test non-existent paths
        assert_eq!(store.get("data.user.nonexistent"), None);
        assert_eq!(store.get("data.user.address.nonexistent"), None);
        assert_eq!(store.get("nonexistent.path"), None);
        assert_eq!(store.get("data.user.name.nonexistent"), None);

        Ok(())
    }

    #[test]
    fn test_persistence() -> Result<(), StoreError> {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_store.db");

        // Create and save complex data
        {
            let mut store = Store::new();
            store.set("string".to_string(), Value::String("value1".to_string()));
            store.set("int".to_string(), Value::Integer(42));
            store.set("object".to_string(), parse_value(r#"{"name": "test", "value": true}"#)?);
            store.save(&file_path)?;
        }

        // Load and verify all data types are preserved
        {
            let store = Store::load(&file_path)?;
            assert_eq!(store.get("string"), Some(&Value::String("value1".to_string())));
            assert_eq!(store.get("int"), Some(&Value::Integer(42)));
            assert_eq!(store.get("object.name"), Some(&Value::String("test".to_string())));
            assert_eq!(store.get("object.value"), Some(&Value::Boolean(true)));
        }

        Ok(())
    }

    #[test]
    fn test_error_handling() {
        // Test invalid JSON
        assert!(parse_value("{invalid_json}").is_err());
        
        // Test invalid file operations
        let store = Store::new();
        assert!(store.save(Path::new("/nonexistent/path/file.db")).is_err());
        assert!(Store::load(Path::new("/nonexistent/path/file.db")).is_err());
    }

    #[test]
    fn test_input_parsing() {
        // Test basic command parsing
        assert_eq!(parse_input("SET key value"), vec!["SET", "key", "value"]);
        
        // Test quoted strings
        assert_eq!(parse_input("SET name \"John Doe\""), vec!["SET", "name", "\"John Doe\""]);
        
        // Test JSON objects
        assert_eq!(
            parse_input("SET user {\"name\": \"John\"}"),
            vec!["SET", "user", "{\"name\": \"John\"}"]
        );
        
        // Test empty input
        assert_eq!(parse_input(""), Vec::<String>::new());
        
        // Test multiple spaces
        assert_eq!(parse_input("GET    key"), vec!["GET", "key"]);
    }
}

fn main() -> Result<(), StoreError> {
    let store_path = Path::new("store.db");
    let mut store = if store_path.exists() {
        Store::load(store_path)?
    } else {
        Store::new()
    };

    println!("Welcome to the key-value store!");
    print_usage();

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut input = String::new();

    loop {
        print!("> ");
        stdout.flush()?;
        input.clear();
        reader.read_line(&mut input)?;

        let parts = parse_input(&input.trim());
        if parts.is_empty() {
            continue;
        }

        match parts[0].to_uppercase().as_str() {
            "SET" => {
                if parts.len() != 3 {
                    println!("Usage: SET <key> <value>");
                    println!("Examples:");
                    println!("  SET name \"John Doe\"");
                    println!("  SET age 25");
                    println!("  SET user {{\"name\": \"John\", \"age\": 30}}");
                    continue;
                }
                handle_set(&mut store, &parts[1], &parts[2], store_path)?;
            }
            "GET" => {
                if parts.len() != 2 {
                    println!("Usage: GET <key>");
                    println!("Examples:");
                    println!("  GET name");
                    println!("  GET user.name");
                    continue;
                }
                handle_get(&store, &parts[1]);
            }
            "DELETE" => {
                if parts.len() != 2 {
                    println!("Usage: DELETE <key>");
                    continue;
                }
                handle_delete(&mut store, &parts[1], store_path)?;
            }
            "LIST" => handle_list(&store),
            "HELP" => print_usage(),
            "EXIT" => {
                println!("Goodbye!");
                break;
            }
            _ => println!("Unknown command. Type HELP for usage."),
        }
    }

    Ok(())
}
