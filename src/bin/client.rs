use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;
use std::process;
use std::fs;
use nix::sys::signal::Signal;
use nix::unistd::Pid;

const SERVER_ADDR: &str = "127.0.0.1:7878";
const PID_FILE: &str = "keystonelight.pid";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    // Handle compact command separately
    if args.len() > 1 && args[1] == "compact" {
        if let Ok(pid_str) = fs::read_to_string(PID_FILE) {
            if let Ok(pid) = pid_str.parse::<i32>() {
                if let Err(e) = nix::sys::signal::kill(Pid::from_raw(pid), Signal::SIGUSR1) {
                    eprintln!("Failed to send compaction signal: {}", e);
                    process::exit(1);
                }
                println!("Compaction signal sent to server (PID: {})", pid);
                process::exit(0);
            }
        }
        eprintln!("Failed to read server PID from {}", PID_FILE);
        process::exit(1);
    }

    // Handle direct commands
    if args.len() > 1 {
        let command = args[1..].join(" ");
        if let Err(e) = send_command(&command) {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
        process::exit(0);
    }

    // Interactive mode
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

        if let Err(e) = send_command(line) {
            eprintln!("Error: {}", e);
            break;
        }
    }

    println!("Goodbye!");
}

fn send_command(command: &str) -> std::io::Result<()> {
    let mut stream = TcpStream::connect(SERVER_ADDR)?;
    let mut reader = BufReader::new(&stream);
    let mut writer = BufWriter::new(&stream);
    
    writeln!(writer, "{}", command)?;
    writer.flush()?;
    
    let mut response = String::new();
    reader.read_line(&mut response)?;
    
    let response = response.trim();
    if response.starts_with("ERROR") {
        eprintln!("{}", response);
    } else {
        println!("{}", response);
    }
    
    Ok(())
} 