use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl Value {
    /// Get a nested value using dot notation (e.g., "user.name")
    pub fn get(&self, key: &str) -> Option<&Value> {
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

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(n) => write!(f, "{}", n),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, val) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, "]")
            }
            Value::Object(map) => {
                write!(f, "{{")?;
                for (i, (key, val)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?
                    }
                    write!(f, "\"{}\": {}", key, val)?;
                }
                write!(f, "}}")
            }
            Value::Null => write!(f, "null"),
        }
    }
}
