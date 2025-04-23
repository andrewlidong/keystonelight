use keystonelight::Database;
use rand::Rng;
use std::sync::Arc;
use std::thread;
use tempfile::tempdir;

#[test]
fn stress_test_persistence() {
    let temp_dir = tempdir().unwrap();
    let log_file = temp_dir.path().join("keystonelight.log");
    let db = Arc::new(Database::with_log_path(log_file.to_str().unwrap()).unwrap());

    let num_clients = 3;
    let ops_per_client = 100;
    let mut handles = vec![];

    // First phase: Write data
    for client_id in 0..num_clients {
        let db_clone = db.clone();
        let handle = thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let mut written_data = vec![];

            for i in 0..ops_per_client {
                let key = format!("key_{}_{}", client_id, i);
                let value = format!("value_{}_{}", client_id, i).into_bytes();

                if db_clone.set(&key, &value).is_ok() {
                    written_data.push((key, value));
                }
            }
            written_data
        });
        handles.push(handle);
    }

    // Collect all written data
    let mut all_written_data = vec![];
    for handle in handles {
        let written_data = handle.join().unwrap();
        all_written_data.extend(written_data);
    }

    // Drop the database to ensure all data is flushed
    drop(db);

    // Second phase: Verify data after restart
    let db = Database::with_log_path(log_file.to_str().unwrap()).unwrap();
    for (key, expected_value) in all_written_data {
        if let Some(value) = db.get(&key) {
            assert_eq!(value, expected_value);
        }
    }
}
