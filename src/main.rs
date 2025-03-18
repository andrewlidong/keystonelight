use keystonelight::cli::{
    handle_delete, handle_get, handle_list, handle_set, parse_input, print_usage,
};
use keystonelight::Store;
use std::io::{self, BufRead, Write};
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_new_store() {
        let store = Store::new();
        assert!(store.is_empty());
    }

    #[test]
    fn test_basic_operations() {
        let mut store = Store::new();

        // Test set and get
        store.set("key1".to_string(), Value::String("value1".to_string()));
        assert_eq!(
            store.get("key1"),
            Some(&Value::String("value1".to_string()))
        );

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
        store.set(
            "string".to_string(),
            Value::String("Hello, World!".to_string()),
        );
        assert_eq!(
            store.get("string"),
            Some(&Value::String("Hello, World!".to_string()))
        );

        // Test integers
        store.set("int".to_string(), Value::Integer(42));
        assert_eq!(store.get("int"), Some(&Value::Integer(42)));

        // Test negative numbers
        store.set("negative".to_string(), Value::Integer(-123));
        assert_eq!(store.get("negative"), Some(&Value::Integer(-123)));

        // Test floats
        store.set("float".to_string(), Value::Float(3.14159));
        assert_eq!(store.get("float"), Some(&Value::Float(3.14159)));

        // Test boolean
        store.set("bool_true".to_string(), Value::Boolean(true));
        store.set("bool_false".to_string(), Value::Boolean(false));
        assert_eq!(store.get("bool_true"), Some(&Value::Boolean(true)));
        assert_eq!(store.get("bool_false"), Some(&Value::Boolean(false)));

        // Test null
        store.set("null_value".to_string(), Value::Null);
        assert_eq!(store.get("null_value"), Some(&Value::Null));

        // Test array with mixed types
        let array = Value::Array(vec![
            Value::Integer(1),
            Value::String("two".to_string()),
            Value::Boolean(true),
            Value::Null,
        ]);
        store.set("array".to_string(), array.clone());
        assert_eq!(store.get("array"), Some(&array));

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

        let value = serde_json::from_str(complex_json)?;
        store.set("data".to_string(), value);

        // Test nested access
        assert_eq!(
            store.get("data.user.name"),
            Some(&Value::String("John Doe".to_string()))
        );
        assert_eq!(
            store.get("data.user.name"),
            Some(&Value::String("John Doe".to_string()))
        );
        assert_eq!(store.get("data.user.age"), Some(&Value::Integer(30)));
        assert_eq!(
            store.get("data.user.address.city"),
            Some(&Value::String("New York".to_string()))
        );
        assert_eq!(
            store.get("data.user.address.coordinates.lat"),
            Some(&Value::Float(40.7128))
        );

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
            let object = serde_json::json!({
                "name": "test",
                "value": true
            });
            store.set("object".to_string(), serde_json::from_value(object)?);
            store.save(&file_path)?;
        }

        // Load and verify all data types are preserved
        {
            let store = Store::load(&file_path)?;
            assert_eq!(
                store.get("string"),
                Some(&Value::String("value1".to_string()))
            );
            assert_eq!(store.get("int"), Some(&Value::Integer(42)));
            assert_eq!(
                store.get("object.name"),
                Some(&Value::String("test".to_string()))
            );
            assert_eq!(store.get("object.value"), Some(&Value::Boolean(true)));
        }

        Ok(())
    }

    #[test]
    fn test_error_handling() {
        // Test invalid JSON
        let invalid_json = "{invalid_json}";
        assert!(serde_json::from_str::<Value>(invalid_json).is_err());

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
        assert_eq!(
            parse_input("SET name \"John Doe\""),
            vec!["SET", "name", "\"John Doe\""]
        );

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
