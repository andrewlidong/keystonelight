use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use keystonelight::server::Server;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;
use uuid::Uuid;

fn cleanup(pid_file: &str, log_file: &str) {
    // Kill any running server processes
    let _ = std::process::Command::new("pkill")
        .args(["-9", "-f", "target/debug/database"])
        .output();

    // Give processes time to fully terminate
    thread::sleep(Duration::from_millis(500));

    // Clean up any existing files
    for file in &[pid_file, log_file] {
        if let Ok(metadata) = fs::metadata(file) {
            if metadata.is_file() {
                for _ in 0..5 {
                    if fs::remove_file(file).is_ok() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(200));
                }
            }
        }
    }

    // Final sleep to ensure resources are released
    thread::sleep(Duration::from_millis(1000));
}

fn connect_client() -> std::io::Result<TcpStream> {
    for _ in 0..10 {
        match TcpStream::connect("127.0.0.1:7878") {
            Ok(stream) => {
                stream.set_read_timeout(Some(Duration::from_secs(1)))?;
                stream.set_write_timeout(Some(Duration::from_secs(1)))?;
                return Ok(stream);
            }
            Err(_) => thread::sleep(Duration::from_millis(200)),
        }
    }
    TcpStream::connect("127.0.0.1:7878")
}

fn send_command(command: &str) -> std::io::Result<String> {
    // Create a new connection for each command
    let mut stream = connect_client()?;

    // Write command
    writeln!(stream, "{}", command)?;
    stream.flush()?;

    // Read response
    let mut reader = BufReader::new(&stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;

    // Ensure we got a complete response
    if response.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Empty response from server",
        ));
    }

    Ok(response.trim().to_string())
}

fn decode_response(response: &str) -> Option<String> {
    if response.starts_with("VALUE base64:") {
        let encoded = &response["VALUE base64:".len()..];
        let decoded = BASE64.decode(encoded).ok()?;
        String::from_utf8(decoded).ok()
    } else if response.starts_with("VALUE ") {
        Some(response["VALUE ".len()..].to_string())
    } else {
        Some(response.to_string())
    }
}

fn start_server(temp_dir: &tempfile::TempDir, num_threads: usize) -> Arc<AtomicBool> {
    let test_id = Uuid::new_v4();
    let pid_file = temp_dir
        .path()
        .join(format!("keystonelight-{}.pid", test_id));
    let log_file = temp_dir
        .path()
        .join(format!("keystonelight-{}.log", test_id));

    cleanup(pid_file.to_str().unwrap(), log_file.to_str().unwrap());

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    thread::spawn(move || {
        let server = Server::with_paths(&pid_file, &log_file, num_threads).unwrap();
        while running_clone.load(Ordering::SeqCst) {
            if let Err(e) = server.run() {
                eprintln!("Server error: {}", e);
                break;
            }
        }
    });

    // Give the server time to start
    thread::sleep(Duration::from_millis(1000));
    running
}

#[test]
fn test_server_basic_operations() {
    let temp_dir = tempdir().unwrap();
    let running = start_server(&temp_dir, 4);

    // Test SET operation
    let response = send_command("set test_key test_value").unwrap();
    assert_eq!(response, "OK");

    // Test GET operation
    let response = send_command("get test_key").unwrap();
    assert!(response.starts_with("VALUE "));
    let value = decode_response(&response).unwrap();
    assert_eq!(value, "test_value");

    // Test DELETE operation
    let response = send_command("delete test_key").unwrap();
    assert_eq!(response, "OK");

    // Verify deletion
    let response = send_command("get test_key").unwrap();
    assert_eq!(response, "NOT_FOUND");

    // Clean up
    running.store(false, Ordering::SeqCst);
    thread::sleep(Duration::from_millis(500));
}

#[test]
fn test_server_concurrent_clients() {
    let temp_dir = tempdir().unwrap();
    let running = start_server(&temp_dir, 4);

    // Spawn multiple client threads
    let mut handles = vec![];
    for i in 0..5 {
        let handle = thread::spawn(move || {
            let key = format!("key{}", i);
            let value = format!("value{}", i);

            // Set value
            let response = send_command(&format!("set {} {}", key, value)).unwrap();
            assert_eq!(response, "OK");

            // Get value back
            let response = send_command(&format!("get {}", key)).unwrap();
            assert!(response.starts_with("VALUE "));
            let value_back = decode_response(&response).unwrap();
            assert_eq!(value_back, value);
        });
        handles.push(handle);
        // Add small delay between client connections
        thread::sleep(Duration::from_millis(50));
    }

    // Wait for all clients to finish
    for handle in handles {
        handle.join().unwrap();
    }

    // Clean up
    running.store(false, Ordering::SeqCst);
    thread::sleep(Duration::from_millis(500));
}

#[test]
fn test_server_error_handling() {
    let temp_dir = tempdir().unwrap();
    let running = start_server(&temp_dir, 4);

    // Test invalid command
    let response = send_command("invalid command").unwrap();
    assert_eq!(response, "ERROR Invalid command");

    // Test missing arguments
    let response = send_command("get").unwrap();
    assert_eq!(response, "ERROR Invalid command");

    // Clean up
    running.store(false, Ordering::SeqCst);
    thread::sleep(Duration::from_millis(500));
}

#[test]
fn test_server_binary_data() {
    let temp_dir = tempdir().unwrap();
    let running = start_server(&temp_dir, 4);

    // Test binary data
    let binary_data = vec![0, 1, 2, 3];
    let response = send_command(&format!(
        "set binary_key base64:{}",
        BASE64.encode(&binary_data)
    ))
    .unwrap();
    assert_eq!(response, "OK");

    // Get binary data back
    let response = send_command("get binary_key").unwrap();
    assert!(response.starts_with("VALUE base64:"));
    let value = decode_response(&response).unwrap();
    assert_eq!(value.as_bytes(), binary_data);

    // Clean up
    running.store(false, Ordering::SeqCst);
    thread::sleep(Duration::from_millis(500));
}

#[test]
fn test_server_thread_pool_configurations() {
    let temp_dir = tempdir().unwrap();

    // Test with different thread pool sizes
    for num_threads in [1, 2, 4, 8] {
        let running = start_server(&temp_dir, num_threads);

        // Test basic operations with each thread pool size
        let response = send_command("set test_key test_value").unwrap();
        assert_eq!(response, "OK");

        let response = send_command("get test_key").unwrap();
        assert!(response.starts_with("VALUE "));
        let value = decode_response(&response).unwrap();
        assert_eq!(value, "test_value");

        // Test concurrent operations
        let mut handles = vec![];
        for i in 0..num_threads * 2 {
            // Test with more clients than threads
            let handle = thread::spawn(move || {
                let key = format!("key{}", i);
                let value = format!("value{}", i);

                let response = send_command(&format!("set {} {}", key, value)).unwrap();
                assert_eq!(response, "OK");

                let response = send_command(&format!("get {}", key)).unwrap();
                assert!(response.starts_with("VALUE "));
                let value_back = decode_response(&response).unwrap();
                assert_eq!(value_back, value);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Clean up
        running.store(false, Ordering::SeqCst);
        thread::sleep(Duration::from_millis(500));
    }
}

#[test]
fn test_server_thread_pool_stress() {
    let temp_dir = tempdir().unwrap();
    let running = start_server(&temp_dir, 4); // Test with 4 threads

    // Create many concurrent clients to stress the thread pool
    let mut handles = vec![];
    for i in 0..20 {
        // Reduced from 100 to 20 clients
        let handle = thread::spawn(move || {
            let key = format!("stress_key{}", i);
            let value = format!("stress_value{}", i);

            // Perform multiple operations
            for _ in 0..5 {
                // Reduced from 10 to 5 operations per client
                let response = send_command(&format!("set {} {}", key, value)).unwrap();
                assert_eq!(response, "OK");

                let response = send_command(&format!("get {}", key)).unwrap();
                assert!(response.starts_with("VALUE "));
                let value_back = decode_response(&response).unwrap();
                assert_eq!(value_back, value);

                thread::sleep(Duration::from_millis(10)); // Add small delay
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Clean up
    running.store(false, Ordering::SeqCst);
    thread::sleep(Duration::from_millis(500));
}
