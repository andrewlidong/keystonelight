use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;

const SERVER_ADDR: &str = "127.0.0.1:7878";
const PID_FILE: &str = "keystonelight.pid";

fn main() -> io::Result<()> {
    println!("Connecting to database server at {}...", SERVER_ADDR);
    let stream = TcpStream::connect(SERVER_ADDR)?;
    let mut stream_writer = stream.try_clone()?;
    let mut stream_reader = BufReader::new(stream);
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
                        let value = value.as_bytes();
                        writeln!(
                            stream_writer,
                            "SET {} {}",
                            key,
                            String::from_utf8_lossy(value)
                        )?;
                        let mut response = String::new();
                        stream_reader.read_line(&mut response)?;
                        print!("{}", response);
                    }
                    [cmd, key] if cmd.to_uppercase() == "GET" => {
                        writeln!(stream_writer, "GET {}", key)?;
                        let mut response = String::new();
                        stream_reader.read_line(&mut response)?;
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
                    [cmd, key] if cmd.to_uppercase() == "DELETE" => {
                        writeln!(stream_writer, "DELETE {}", key)?;
                        let mut response = String::new();
                        stream_reader.read_line(&mut response)?;
                        print!("{}", response);
                    }
                    [cmd] if cmd.to_uppercase() == "COMPACT" => {
                        writeln!(stream_writer, "COMPACT")?;
                        let mut response = String::new();
                        stream_reader.read_line(&mut response)?;
                        print!("{}", response);
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

fn send_command(command: &str) -> std::io::Result<()> {
    let stream = TcpStream::connect(SERVER_ADDR)?;
    let mut reader = BufReader::new(&stream);
    let mut writer = BufWriter::new(&stream);

    // Format the command according to the protocol
    let formatted_command = match command.split_whitespace().collect::<Vec<&str>>().as_slice() {
        ["get", key] => format!("get {}", key),
        ["set", key, value @ ..] => format!("set {} {}", key, value.join(" ")),
        ["delete", key] => format!("delete {}", key),
        ["compact"] => "compact".to_string(),
        _ => {
            eprintln!("Invalid command format");
            return Ok(());
        }
    };

    writeln!(writer, "{}", formatted_command)?;
    writer.flush()?;

    let mut response = String::new();
    reader.read_line(&mut response)?;

    let response = response.trim();
    if response.starts_with("ERROR") {
        eprintln!("{}", response);
    } else if response.starts_with("OK ") {
        println!("{}", &response[3..]);
    } else {
        println!("{}", response);
    }

    Ok(())
}
