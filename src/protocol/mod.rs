//! Protocol definitions for client-server communication.
//!
//! This module defines the command protocol used between clients and the server.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
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

/// Responses that can be sent from the server to the client.
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
