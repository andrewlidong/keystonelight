//! Protocol definitions for client-server communication.
//!
//! This module defines the command protocol used between clients and the server.
//!
//! # Examples
//!
//! Basic command parsing:
//!
//! ```
//! use keystonelight::protocol::{Command, parse_command};
//!
//! // Parse a GET command
//! let cmd = parse_command("GET mykey").unwrap();
//! match cmd {
//!     Command::Get(key) => assert_eq!(key, "mykey"),
//!     _ => panic!("Expected GET command"),
//! }
//!
//! // Parse a SET command
//! let cmd = parse_command("SET mykey myvalue").unwrap();
//! match cmd {
//!     Command::Set(key, value) => {
//!         assert_eq!(key, "mykey");
//!         assert_eq!(String::from_utf8(value).unwrap(), "myvalue");
//!     },
//!     _ => panic!("Expected SET command"),
//! }
//! ```
//!
//! Binary data handling:
//!
//! ```
//! use keystonelight::protocol::{Command, parse_command};
//! use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
//!
//! // Handle binary data in SET command
//! let binary_data = vec![0, 1, 2, 3];
//! let encoded = format!("SET mykey base64:{}", BASE64.encode(&binary_data));
//! let cmd = parse_command(&encoded).unwrap();
//! match cmd {
//!     Command::Set(key, value) => {
//!         assert_eq!(key, "mykey");
//!         assert_eq!(value, binary_data);
//!     },
//!     _ => panic!("Expected SET command"),
//! }
//! ```

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::fmt;

/// Commands that can be sent from the client to the server.
///
/// # Examples
///
/// ```
/// use keystonelight::protocol::Command;
///
/// // Create a GET command
/// let get_cmd = Command::Get("mykey".to_string());
///
/// // Create a SET command
/// let set_cmd = Command::Set("mykey".to_string(), b"myvalue".to_vec());
///
/// // Create a DELETE command
/// let delete_cmd = Command::Delete("mykey".to_string());
///
/// // Create a COMPACT command
/// let compact_cmd = Command::Compact;
/// ```
#[derive(Debug)]
pub enum Command {
    /// Get the value associated with a key
    Get(String),
    /// Set a key-value pair
    Set(String, Vec<u8>),
    /// Delete a key-value pair
    Delete(String),
    /// Compact the log file
    Compact,
}

/// Responses that can be sent from the server to the client.
///
/// # Examples
///
/// ```
/// use keystonelight::protocol::Response;
///
/// // Success response
/// let ok = Response::Ok;
/// assert_eq!(ok.to_string(), "OK");
///
/// // Value response with text
/// let value = Response::Value(b"Hello, World!".to_vec());
/// assert_eq!(value.to_string(), "OK Hello, World!");
///
/// // Value response with binary data
/// let binary = Response::Value(vec![0, 1, 2, 3]);
/// assert!(binary.to_string().starts_with("OK base64:"));
///
/// // Not found response
/// let not_found = Response::NotFound;
/// assert_eq!(not_found.to_string(), "NOT_FOUND");
///
/// // Error response
/// let error = Response::Error("Invalid key".to_string());
/// assert_eq!(error.to_string(), "ERROR Invalid key");
/// ```
#[derive(Debug, PartialEq)]
pub enum Response {
    /// Operation successful
    Ok,
    /// Operation successful with a value
    Value(Vec<u8>),
    /// Key not found
    NotFound,
    /// Error occurred
    Error(String),
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Response::Ok => write!(f, "OK"),
            Response::Value(val) => {
                // Check if the value contains any non-printable characters
                let is_binary = val
                    .iter()
                    .any(|&b| !b.is_ascii_graphic() && !b.is_ascii_whitespace());
                if is_binary {
                    write!(f, "OK base64:{}", BASE64.encode(val))
                } else {
                    match String::from_utf8(val.clone()) {
                        Ok(text) => write!(f, "OK {}", text),
                        Err(_) => write!(f, "OK base64:{}", BASE64.encode(val)),
                    }
                }
            }
            Response::NotFound => write!(f, "NOT_FOUND"),
            Response::Error(msg) => write!(f, "ERROR {}", msg),
        }
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Get(key) => write!(f, "get {}", key),
            Command::Set(key, value) => {
                // Check if the value contains any non-printable characters
                let is_binary = value
                    .iter()
                    .any(|&b| !b.is_ascii_graphic() && !b.is_ascii_whitespace());
                if is_binary {
                    write!(f, "set {} [binary data]", key)
                } else {
                    match String::from_utf8(value.clone()) {
                        Ok(text) => write!(f, "set {} {}", key, text),
                        Err(_) => write!(f, "set {} [binary data]", key),
                    }
                }
            }
            Command::Delete(key) => write!(f, "delete {}", key),
            Command::Compact => write!(f, "compact"),
        }
    }
}

/// Parse a command from a string.
///
/// # Arguments
///
/// * `line` - The input line to parse
///
/// # Returns
///
/// A parsed command if successful, None otherwise.
///
/// # Examples
///
/// ```
/// use keystonelight::protocol::{Command, parse_command};
///
/// // Parse GET command
/// let cmd = parse_command("GET mykey").unwrap();
/// match cmd {
///     Command::Get(key) => assert_eq!(key, "mykey"),
///     _ => panic!("Expected GET command"),
/// }
///
/// // Parse SET command with value
/// let cmd = parse_command("SET mykey myvalue").unwrap();
/// match cmd {
///     Command::Set(key, value) => {
///         assert_eq!(key, "mykey");
///         assert_eq!(String::from_utf8(value).unwrap(), "myvalue");
///     },
///     _ => panic!("Expected SET command"),
/// }
///
/// // Parse DELETE command
/// let cmd = parse_command("DELETE mykey").unwrap();
/// match cmd {
///     Command::Delete(key) => assert_eq!(key, "mykey"),
///     _ => panic!("Expected DELETE command"),
/// }
///
/// // Parse COMPACT command
/// let cmd = parse_command("COMPACT").unwrap();
/// match cmd {
///     Command::Compact => {},
///     _ => panic!("Expected COMPACT command"),
/// }
///
/// // Invalid commands return None
/// assert!(parse_command("INVALID").is_none());
/// assert!(parse_command("GET").is_none());
/// assert!(parse_command("SET key").is_some()); // SET with empty value is valid
/// ```
pub fn parse_command(line: &str) -> Option<Command> {
    let mut parts = line.trim().splitn(3, ' ');
    let cmd = parts.next()?.to_uppercase();

    match cmd.as_str() {
        "GET" => {
            let key = parts.next()?;
            if parts.next().is_some() {
                return None;
            } // GET should have exactly one argument
            Some(Command::Get(key.to_string()))
        }
        "SET" => {
            let key = parts.next()?;
            let value = parts.next().unwrap_or("");
            // Try to decode base64 if it starts with "base64:"
            let value = if value.starts_with("base64:") {
                BASE64
                    .decode(&value[7..])
                    .unwrap_or_else(|_| value.as_bytes().to_vec())
            } else {
                value.as_bytes().to_vec()
            };
            Some(Command::Set(key.to_string(), value))
        }
        "DELETE" => {
            let key = parts.next()?;
            if parts.next().is_some() {
                return None;
            } // DELETE should have exactly one argument
            Some(Command::Delete(key.to_string()))
        }
        "COMPACT" => {
            if parts.next().is_some() {
                return None;
            } // COMPACT should have no arguments
            Some(Command::Compact)
        }
        _ => None,
    }
}
