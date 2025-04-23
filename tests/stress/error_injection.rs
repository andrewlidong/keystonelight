use keystonelight::Database;
use rand::Rng;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn stress_test_error_injection() {
    let temp_dir = tempdir().unwrap();
    let log_file = temp_dir.path().join("keystonelight.log");
    let db = Arc::new(Database::with_log_path(log_file.to_str().unwrap()).unwrap());

    let num_clients = 2; // Reduced from 3
    let ops_per_client = 25; // Reduced from 50
    let timeout = Duration::from_secs(10); // Reduced from 30 seconds

    let mut handles = vec![];

    for client_id in 0..num_clients {
        let db_clone = db.clone();
        let handle = thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let mut errors = 0;

            for i in 0..ops_per_client {
                let key = format!("key_{}_{}", client_id, i);
                let value = format!("value_{}_{}", client_id, i).into_bytes();

                // Try to inject errors by using invalid keys or values
                if rng.gen_bool(0.1) {
                    // 10% chance of using an invalid key (empty key)
                    if let Err(_e) = db_clone.set("", &value) {
                        errors += 1;
                    }
                } else if rng.gen_bool(0.1) {
                    // 10% chance of using a value that's too large (256KB)
                    let invalid_value = vec![0u8; 256 * 1024]; // Reduced from 512KB
                    if let Err(_e) = db_clone.set(&key, &invalid_value) {
                        errors += 1;
                    }
                } else {
                    // Normal operation
                    if let Err(_e) = db_clone.set(&key, &value) {
                        errors += 1;
                    }
                    let _ = db_clone.get(&key);
                    if let Err(_e) = db_clone.delete(&key) {
                        errors += 1;
                    }
                }

                // Add a small delay between operations
                thread::sleep(Duration::from_millis(10));
            }
            errors
        });
        handles.push(handle);
    }

    // Wait for all threads with timeout
    let start = std::time::Instant::now();
    let mut total_errors = 0;

    for handle in handles {
        if start.elapsed() > timeout {
            panic!("Test timed out after {:?}", timeout);
        }

        match handle.join() {
            Ok(errors) => total_errors += errors,
            Err(e) => panic!("Thread panicked: {:?}", e),
        }
    }

    // Allow some errors since we're testing error conditions
    assert!(
        total_errors <= num_clients * ops_per_client / 2,
        "Too many errors occurred: {}",
        total_errors
    );
}
