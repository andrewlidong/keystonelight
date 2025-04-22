//! Client implementation for the key-value database.
//!
//! This module provides both a programmatic client interface and an interactive mode.

use crate::protocol::{parse_command, Command};
use std::io::{self, Write};
use std::net::TcpStream;

/// The server address to connect to
const SERVER_ADDR: &str = "127.0.0.1:7878";

/// A client connection to the key-value database server.
pub struct Client {
    stream: TcpStream,
}

impl Client {
    /// Create a new client connection to the server.
    pub fn new() -> io::Result<Self> {
        let stream = TcpStream::connect(SERVER_ADDR)?;
        Ok(Client { stream })
    }

    /// Send a command to the server and receive the response.
    pub fn send_command(&mut self, command: &str) -> io::Result<String> {
        writeln!(&mut self.stream, "{}", command)?;
        let mut response = String::new();
        io::Read::read_to_string(&mut self.stream, &mut response)?;
        Ok(response)
    }
}

/// Run the client in interactive mode.
pub fn run_interactive() -> io::Result<()> {
    let mut client = Client::new()?;
    let mut input = String::new();

    println!("Connected to server at {}", SERVER_ADDR);
    println!("Type 'quit' or 'exit' to exit");

    loop {
        print!("> ");
        io::stdout().flush()?;
        input.clear();
        io::stdin().read_line(&mut input)?;

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input == "quit" || input == "exit" {
            break;
        }

        match parse_command(input) {
            Some(command) => match client.send_command(&command.to_string()) {
                Ok(response) => print!("{}", response),
                Err(e) => eprintln!("Error: {}", e),
            },
            None => eprintln!("Error: Invalid command"),
        }
    }

    Ok(())
}
