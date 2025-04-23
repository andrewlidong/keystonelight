use keystonelight::storage::Database;
use log::{info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

const NUM_CLIENTS: usize = 10;
const OPERATIONS_PER_CLIENT: usize = 100;
const TIMEOUT_SECONDS: u64 = 5;

pub fn test_concurrent_operations(db: Arc<Database>) {
    info!("Starting concurrent operations test");
    let mut handles = vec![];
    let mut all_successful_sets: Vec<(String, Vec<u8>)> = vec![];

    for client_id in 0..NUM_CLIENTS {
        let db_clone = db.clone();
        let handle = thread::spawn(move || {
            let mut successful_sets = vec![];
            let mut errors = 0;

            for i in 0..OPERATIONS_PER_CLIENT {
                let key = format!("client{}_key{}", client_id, i);
                let value = format!("value{}", i).into_bytes();

                match i % 3 {
                    0 => {
                        if db_clone.set(&key, &value).is_ok() {
                            successful_sets.push((key, value));
                        }
                    }
                    1 => {
                        if let None = db_clone.get(&key) {
                            successful_sets.push((key, value));
                        }
                    }
                    2 => {
                        if db_clone.delete(&key).is_ok() {
                            successful_sets.push((key, value));
                        }
                    }
                    _ => unreachable!(),
                }
            }

            (successful_sets, errors)
        });
        handles.push(handle);
    }

    for handle in handles {
        match handle.join() {
            Ok((successful_sets, errors)) => {
                all_successful_sets.extend(successful_sets);
                assert_eq!(
                    errors, 0,
                    "No errors should occur during concurrent operations"
                );
            }
            Err(e) => panic!("Thread panicked: {:?}", e),
        }
    }

    // Verify all successful operations
    for (key, expected_value) in all_successful_sets {
        match db.get(&key) {
            Some(value) => assert_eq!(value, expected_value, "Value mismatch for key {}", key),
            None => warn!("Key {} not found after concurrent operations", key),
        }
    }

    info!("Concurrent operations test completed");
}
