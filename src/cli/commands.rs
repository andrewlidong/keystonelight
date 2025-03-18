use std::path::Path;
use crate::error::StoreError;
use crate::store::Store;
use crate::value::Value;

/// Parses a value from a string input
pub fn parse_value(s: &str) -> Result<Value, StoreError> {
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

/// Prints the usage information
pub fn print_usage() {
    println!("Welcome to KeystoneLight - A lightweight key-value store!");
    println!("\nAvailable commands:");
    println!("  SET <key> <value>  - Store a key-value pair");
    println!("  GET <key>         - Retrieve a value by key (supports nested access with dots)");
    println!("  DELETE <key>      - Remove a key-value pair");
    println!("  LIST              - Show all key-value pairs");
    println!("  HELP              - Show this help message");
    println!("  EXIT              - Exit the program");
    println!("\nExamples:");
    println!("  SET name \"John Doe\"");
    println!("  SET age 25");
    println!("  SET user {{\"name\": \"John\", \"age\": 30}}");
    println!("  GET user.name");
}

/// Handles the SET command
pub fn handle_set(
    store: &mut Store,
    key: &str,
    value: &str,
    store_path: &Path,
) -> Result<(), StoreError> {
    match parse_value(value) {
        Ok(parsed_value) => {
            store.set(key.to_string(), parsed_value);
            store.save(store_path)?;
            println!("Value set successfully");
            Ok(())
        }
        Err(e) => {
            println!("Error parsing value: {}", e);
            Err(e)
        }
    }
}

/// Handles the GET command
pub fn handle_get(store: &Store, key: &str) {
    match store.get(key) {
        Some(value) => println!("Value: {}", value),
        None => println!("Key not found!"),
    }
}

/// Handles the DELETE command
pub fn handle_delete(store: &mut Store, key: &str, store_path: &Path) -> Result<(), StoreError> {
    match store.delete(key) {
        Some(_) => {
            println!("Key deleted successfully!");
            store.save(store_path)?;
        }
        None => println!("Key not found!"),
    }
    Ok(())
}

/// Handles the LIST command
pub fn handle_list(store: &Store) {
    if store.is_empty() {
        println!("Store is empty");
    } else {
        println!("Store contents:");
        for (key, value) in store.iter() {
            println!("  {} => {}", key, value);
        }
    }
} 