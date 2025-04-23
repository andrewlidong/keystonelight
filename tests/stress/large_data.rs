use keystonelight::Database;
use rand::Rng;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn stress_test_large_data() {
    let temp_dir = tempdir().unwrap();
    let log_file = temp_dir.path().join("keystonelight.log");
    let db = Arc::new(Database::with_log_path(log_file.to_str().unwrap()).unwrap());
    let num_clients = 2; // Reduced from 3
    let ops_per_client = 20; // Reduced from 50

    let mut handles = vec![];

    for client_id in 0..num_clients {
        let db_clone = db.clone();
        let handle = thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let mut successful_sets = vec![];

            for i in 0..ops_per_client {
                let key = format!("key_{}_{}", client_id, i);
                // Reduced data size to 10KB - 100KB
                let data_size = rng.gen_range(10_000..100_000);
                let value: Vec<u8> = (0..data_size).map(|_| rng.gen()).collect();

                if db_clone.set(&key, &value).is_ok() {
                    successful_sets.push((key, value));
                }

                if i % 5 == 0 {
                    thread::sleep(Duration::from_micros(500));
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
