//! Protocol definitions for client-server communication.
//!
//! This module defines the command protocol used between clients and the server.

use std::fmt;
use std::io;
use std::io::{self, BufRead, Write};
use std::net::TcpStream;

/// Commands that can be sent from the client to the server.
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

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Get(key) => write!(f, "get {}", key),
            Command::Set(key, value) => match String::from_utf8(value.clone()) {
                Ok(text) => write!(f, "set {} {}", key, text),
                Err(_) => write!(f, "set {} [binary data]", key),
            },
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
pub fn parse_command(line: &str) -> Option<Command> {
    let mut parts = line.trim().splitn(3, ' ');
    let cmd = parts.next()?.to_uppercase();

    match cmd.as_str() {
        "GET" => {
            let key = parts.next()?.to_string();
            Some(Command::Get(key))
        }
        "SET" => {
            let key = parts.next()?.to_string();
            let value = parts.next().unwrap_or("").as_bytes().to_vec();
            Some(Command::Set(key, value))
        }
        "DELETE" => {
            let key = parts.next()?.to_string();
            Some(Command::Delete(key))
        }
        "COMPACT" => Some(Command::Compact),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_parsing() {
        // Test get command
        let cmd = parse_command("get test_key").unwrap();
        assert!(matches!(cmd, Command::Get(key) if key == "test_key"));

        // Test set command
        let cmd = parse_command("set test_key test_value").unwrap();
        assert!(matches!(cmd, Command::Set(key, value)
            if key == "test_key" && value == b"test_value"));

        // Test delete command
        let cmd = parse_command("delete test_key").unwrap();
        assert!(matches!(cmd, Command::Delete(key) if key == "test_key"));

        // Test compact command
        let cmd = parse_command("compact").unwrap();
        assert!(matches!(cmd, Command::Compact));

        // Test invalid commands
        assert!(parse_command("").is_none());
        assert!(parse_command("get").is_none());
        assert!(parse_command("set key").is_none());
        assert!(parse_command("delete").is_none());
        assert!(parse_command("unknown").is_none());
    }
}
