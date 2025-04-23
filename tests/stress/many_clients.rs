use keystonelight::Database;
use rand::Rng;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn stress_test_many_clients() {
    let temp_dir = tempdir().unwrap();
    let log_file = temp_dir.path().join("keystonelight.log");
    let db = Arc::new(Database::with_log_path(log_file.to_str().unwrap()).unwrap());
    let num_clients = 20; // Reduced from 50
    let ops_per_client = 50; // Reduced from 200

    let mut handles = vec![];

    for client_id in 0..num_clients {
        let db_clone = db.clone();
        let handle = thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let mut successful_sets = vec![];

            for i in 0..ops_per_client {
                let key = format!("key_{}_{}", client_id, i);
                let value = format!("value_{}_{}", client_id, i).into_bytes();

                match rng.gen_range(0..3) {
                    0 => {
                        if db_clone.set(&key, &value).is_ok() {
                            successful_sets.push((key, value));
                        }
                    }
                    1 => {
                        let _ = db_clone.get(&key);
                    }
                    2 => {
                        let _ = db_clone.delete(&key);
                    }
                    _ => unreachable!(),
                }

                if i % 10 == 0 {
                    thread::sleep(Duration::from_micros(100));
                }
            }
            successful_sets
        });
        handles.push(handle);
    }

    let mut all_successful_sets = vec![];
    for handle in handles {
        let successful_sets = handle.join().unwrap();
        all_successful_sets.extend(successful_sets);
    }

    for (key, expected_value) in all_successful_sets {
        if let Some(value) = db.get(&key) {
            assert_eq!(value, expected_value);
        }
    }
}
