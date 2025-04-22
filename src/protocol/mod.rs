//! Protocol definitions for client-server communication.
//!
//! This module defines the command protocol used between clients and the server.

use std::fmt;
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

#[derive(Debug)]
pub enum Response {
    Ok(Option<String>),
    Error(String),
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

impl Response {
    pub fn send(&self, writer: &mut impl Write) -> io::Result<()> {
        match self {
            Response::Ok(value) => {
                if let Some(v) = value {
                    writeln!(writer, "OK {}", v)
                } else {
                    writeln!(writer, "OK")
                }
            }
            Response::Error(msg) => writeln!(writer, "ERROR {}", msg),
        }
    }

    pub fn receive(stream: &mut TcpStream) -> io::Result<Self> {
        let mut reader = io::BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        let line = line.trim();
        if line.starts_with("OK ") {
            let value = line[3..].to_string();
            Ok(Response::Ok(Some(value)))
        } else if line == "OK" {
            Ok(Response::Ok(None))
        } else if line.starts_with("ERROR ") {
            Ok(Response::Error(line[6..].to_string()))
        } else {
            Ok(Response::Error("Invalid response format".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_parsing() {
        // Test get command
        let cmd = Command::parse("get test_key").unwrap();
        assert!(matches!(cmd, Command::Get(key) if key == "test_key"));

        // Test set command
        let cmd = Command::parse("set test_key test value").unwrap();
        assert!(matches!(cmd, Command::Set(key, value)
            if key == "test_key" && value == "test value"));

        // Test delete command
        let cmd = Command::parse("delete test_key").unwrap();
        assert!(matches!(cmd, Command::Delete(key) if key == "test_key"));

        // Test compact command
        let cmd = Command::parse("compact").unwrap();
        assert!(matches!(cmd, Command::Compact));

        // Test invalid commands
        assert!(Command::parse("").is_err());
        assert!(Command::parse("get").is_err());
        assert!(Command::parse("set key").is_err());
        assert!(Command::parse("delete").is_err());
        assert!(Command::parse("unknown").is_err());
    }
}
