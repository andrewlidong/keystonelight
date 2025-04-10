use std::env;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::{thread, time};

use fs2::FileExt;

const DB_PATH: &str = "db.txt";

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} [get|set] [key] [value?]", args[0]);
        return;
    }

    match args[1].as_str() {
        "set" => {
            if args.len() != 4 {
                eprintln!("Usage: {} set <key> <value>", args[0]);
                return;
            }
            let key = &args[2];
            let value = &args[3];
            if let Err(e) = set(key, value) {
                eprintln!("Error setting value: {}", e);
            }
        }
        "get" => {
            if args.len() != 3 {
                eprintln!("Usage: {} get <key>", args[0]);
                return;
            }
            let key = &args[2];
            match get(key) {
                Some(value) => println!("{}", value),
                None => println!("Key not found"),
            }
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
        }
    }
}

fn open_or_create_db() -> std::io::Result<File> {
    if !Path::new(DB_PATH).exists() {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .mode(0o600)
            .open(DB_PATH)?;
        Ok(file)
    } else {
        OpenOptions::new()
            .write(true)
            .read(true)
            .open(DB_PATH)
    }
}

fn set(key: &str, value: &str) -> std::io::Result<()> {
    let mut file = open_or_create_db()?;
    FileExt::lock_exclusive(&file)?;
    println!("Holding lock... (sleeping 5 seconds)");
    thread::sleep(time::Duration::from_secs(5));

    writeln!(file, "{}|{}", key, value)?;
    FileExt::unlock(&file)?;
    println!("Lock released.");
    Ok(())
}

fn get(key: &str) -> Option<String> {
    let file = open_or_create_db().ok()?;
    FileExt::lock_shared(&file).ok()?;

    let reader = BufReader::new(&file);

    let mut result = None;
    for line in reader.lines().flatten() {
        if let Some((k, v)) = line.split_once('|') {
            if k == key {
                result = Some(v.to_string());
            }
        }
    }

    FileExt::unlock(&file).ok()?;
    result
}
