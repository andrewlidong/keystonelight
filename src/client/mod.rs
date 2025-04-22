//! Client implementation for the key-value database.
//!
//! This module provides both a programmatic client interface and an interactive mode.

use crate::protocol::parse_command;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;

/// The server address to connect to
const SERVER_ADDR: &str = "127.0.0.1:7878";

/// A client connection to the key-value database server.
pub struct Client {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
}

impl Client {
    /// Create a new client connection to the server.
    pub fn new() -> io::Result<Self> {
        let stream = TcpStream::connect(SERVER_ADDR)?;
        let reader = BufReader::new(stream.try_clone()?);
        Ok(Client { stream, reader })
    }

    /// Send a command to the server and receive the response.
    pub fn send_command(&mut self, command: &str) -> io::Result<String> {
        writeln!(&mut self.stream, "{}", command)?;
        self.stream.flush()?;
        let mut response = String::new();
        self.reader.read_line(&mut response)?;
        Ok(response)
    }
}

/// Run the client in interactive mode.
pub fn run_interactive() -> io::Result<()> {
    println!("Connecting to database server at {}...", SERVER_ADDR);
    let mut client = Client::new()?;
    println!("Connected successfully!");
    println!("Enter commands (type 'help' for usage, 'quit' to exit):");

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut line = String::new();

    print!("> ");
    io::stdout().flush()?;

    while reader.read_line(&mut line)? > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            print!("> ");
            io::stdout().flush()?;
            continue;
        }

        match trimmed {
            "quit" | "exit" => {
                println!("Goodbye!");
                break;
            }
            "help" => {
                println!("Available commands:");
                println!("  SET <key> <value>  - Set a key-value pair");
                println!("  GET <key>         - Get the value for a key");
                println!("  DELETE <key>      - Delete a key-value pair");
                println!("  COMPACT           - Trigger log compaction");
                println!("  quit/exit         - Exit the client");
            }
            _ => {
                let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
                match parts.as_slice() {
                    [cmd, key, value] if cmd.to_uppercase() == "SET" => {
                        match client.send_command(&format!("SET {} {}", key, value)) {
                            Ok(response) => print!("{}", response),
                            Err(e) => println!("Error: {}", e),
                        }
                    }
                    [cmd, key] if cmd.to_uppercase() == "GET" => {
                        match client.send_command(&format!("GET {}", key)) {
                            Ok(response) => {
                                if response.starts_with("VALUE ") {
                                    let value = response.trim().split_once(' ').unwrap().1;
                                    if value.starts_with("base64:") {
                                        // Handle base64-encoded binary data
                                        let encoded = &value[7..]; // Skip "base64:" prefix
                                        if let Ok(bytes) = BASE64.decode(encoded) {
                                            println!("<binary data of {} bytes>", bytes.len());
                                        } else {
                                            println!("Error: Invalid base64 encoding");
                                        }
                                    } else {
                                        // Regular text value
                                        println!("{}", value);
                                    }
                                } else {
                                    print!("{}", response);
                                }
                            }
                            Err(e) => println!("Error: {}", e),
                        }
                    }
                    [cmd, key] if cmd.to_uppercase() == "DELETE" => {
                        match client.send_command(&format!("DELETE {}", key)) {
                            Ok(response) => print!("{}", response),
                            Err(e) => println!("Error: {}", e),
                        }
                    }
                    [cmd] if cmd.to_uppercase() == "COMPACT" => {
                        match client.send_command("COMPACT") {
                            Ok(response) => print!("{}", response),
                            Err(e) => println!("Error: {}", e),
                        }
                    }
                    _ => {
                        println!("Error: Invalid command. Type 'help' for usage.");
                    }
                }
            }
        }

        line.clear();
        print!("> ");
        io::stdout().flush()?;
    }

    Ok(())
}
