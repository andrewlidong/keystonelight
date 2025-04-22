mod client;
mod protocol;
mod server;
mod storage;
mod thread_pool;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} [serve|client] [num_threads]", args[0]);
        process::exit(1);
    }

    match args[1].as_str() {
        "serve" => {
            let num_threads = if args.len() > 2 {
                args[2].parse().unwrap_or(4)
            } else {
                4
            };
            if let Err(e) =
                server::Server::with_paths("keystonelight.pid", "keystonelight.log", num_threads)
                    .and_then(|server| server.run())
            {
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
            eprintln!("Usage: {} [serve|client] [num_threads]", args[0]);
            process::exit(1);
        }
    }
}
