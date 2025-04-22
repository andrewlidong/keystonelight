mod client;
mod protocol;
mod server;
mod storage;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} [serve|client]", args[0]);
        process::exit(1);
    }

    match args[1].as_str() {
        "serve" => {
            if let Err(e) = server::Server::new().and_then(|server| server.run()) {
                eprintln!("Server error: {}", e);
                process::exit(1);
            }
        }
        "client" => {
            if let Err(e) = client::run_interactive() {
                eprintln!("Client error: {}", e);
                process::exit(1);
            }
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            eprintln!("Usage: {} [serve|client]", args[0]);
            process::exit(1);
        }
    }
}
