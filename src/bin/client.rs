use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;
use std::process;

const SERVER_ADDR: &str = "127.0.0.1:7878";

fn main() {
    let stream = match TcpStream::connect(SERVER_ADDR) {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("Failed to connect to server: {}", e);
            process::exit(1);
        }
    };

    println!("Connected to database server at {}", SERVER_ADDR);
    println!("Enter commands (type 'help' for usage, 'quit' to exit):");

    let mut reader = BufReader::new(stream.try_clone().expect("Failed to clone stream"));
    let mut writer = BufWriter::new(stream);
    let stdin = io::stdin();
    let mut stdin_lines = stdin.lock().lines();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let line = match stdin_lines.next() {
            Some(Ok(line)) => line,
            Some(Err(e)) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
            None => break,
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match line {
            "quit" | "exit" => break,
            "help" => {
                println!("Available commands:");
                println!("  get <key>           - Retrieve value for key");
                println!("  set <key> <value>   - Set key to value");
                println!("  delete <key>        - Delete key");
                println!("  help                - Show this help");
                println!("  quit                - Exit the client");
                continue;
            }
            _ => {}
        }

        if let Err(e) = writer.write_all(format!("{}\n", line).as_bytes()) {
            eprintln!("Failed to send command: {}", e);
            break;
        }
        if let Err(e) = writer.flush() {
            eprintln!("Failed to flush command: {}", e);
            break;
        }

        let mut response = String::new();
        match reader.read_line(&mut response) {
            Ok(0) => {
                println!("Server closed connection");
                break;
            }
            Ok(_) => {
                let response = response.trim();
                if response.starts_with("ERROR") {
                    eprintln!("{}", response);
                } else {
                    println!("{}", response);
                }
            }
            Err(e) => {
                eprintln!("Failed to read response: {}", e);
                break;
            }
        }
    }

    println!("Goodbye!");
} 