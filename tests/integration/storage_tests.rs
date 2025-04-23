use keystonelight::storage::Database;
use std::fs;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

fn cleanup(log_file: &str) {
    for _ in 0..5 {
        if let Ok(_) = fs::remove_file(log_file) {
            break;
        }
        thread::sleep(Duration::from_millis(200));
    }
    thread::sleep(Duration::from_millis(500));
}

fn wait_for_file_sync() {
    thread::sleep(Duration::from_millis(500));
}

#[test]
fn test_persistence() {
    let temp_dir = tempdir().unwrap();
    let log_file = temp_dir.path().join("keystonelight.log");
    cleanup(log_file.to_str().unwrap());

    let db = Database::with_log_path(log_file.to_str().unwrap()).unwrap();
    db.set("key1", b"value1").unwrap();
    db.set("key2", b"value2").unwrap();
    wait_for_file_sync();
    drop(db);

    // Second instance: verify data
    let db = Database::with_log_path(log_file.to_str().unwrap()).unwrap();
    assert_eq!(db.get("key1"), Some(b"value1".to_vec()));
    assert_eq!(db.get("key2"), Some(b"value2".to_vec()));
}

#[test]
fn test_compaction() {
    let temp_dir = tempdir().unwrap();
    let log_file = temp_dir.path().join("keystonelight.log");
    cleanup(log_file.to_str().unwrap());

    let db = Database::with_log_path(log_file.to_str().unwrap()).unwrap();

    // Write multiple versions of the same key
    for i in 0..100 {
        db.set(&format!("key{}", i % 10), format!("value{}", i).as_bytes())
            .unwrap();
    }
    wait_for_file_sync();

    // Get initial file size
    let initial_size = fs::metadata(&log_file).unwrap().len();

    // Trigger compaction
    db.compact().unwrap();
    wait_for_file_sync();

    // Get compacted file size
    let compacted_size = fs::metadata(&log_file).unwrap().len();

    // Verify compaction reduced file size
    assert!(
        compacted_size < initial_size,
        "Compaction did not reduce file size (initial: {}, compacted: {})",
        initial_size,
        compacted_size
    );

    // Verify data integrity after compaction
    for i in 0..10 {
        let key = format!("key{}", i);
        let expected_value = format!("value{}", 90 + i);
        assert_eq!(db.get(&key), Some(expected_value.as_bytes().to_vec()));
    }
}

#[test]
fn test_delete() {
    let temp_dir = tempdir().unwrap();
    let log_file = temp_dir.path().join("keystonelight.log");
    cleanup(log_file.to_str().unwrap());

    let db = Database::with_log_path(log_file.to_str().unwrap()).unwrap();

    // Set and verify key
    db.set("key1", b"value1").unwrap();
    assert_eq!(db.get("key1"), Some(b"value1".to_vec()));

    // Delete and verify deletion
    db.delete("key1").unwrap();
    assert_eq!(db.get("key1"), None);

    // Verify deletion persists after reopening
    drop(db);
    wait_for_file_sync();

    let db = Database::with_log_path(log_file.to_str().unwrap()).unwrap();
    assert_eq!(db.get("key1"), None);
}

#[test]
fn test_concurrent_operations() {
    let temp_dir = tempdir().unwrap();
    let log_file = temp_dir.path().join("keystonelight.log");
    cleanup(log_file.to_str().unwrap());

    let db = Database::with_log_path(log_file.to_str().unwrap()).unwrap();

    // Perform multiple operations in quick succession
    for i in 0..100 {
        db.set(&format!("key{}", i), format!("value{}", i).as_bytes())
            .unwrap();
    }

    wait_for_file_sync();

    // Verify all operations were successful
    for i in 0..100 {
        assert_eq!(
            db.get(&format!("key{}", i)),
            Some(format!("value{}", i).as_bytes().to_vec())
        );
    }
}

#[test]
fn test_large_values() {
    let temp_dir = tempdir().unwrap();
    let log_file = temp_dir.path().join("keystonelight.log");
    cleanup(log_file.to_str().unwrap());

    let db = Database::with_log_path(log_file.to_str().unwrap()).unwrap();

    // Create a large value
    let large_value = "x".repeat(1024 * 1024); // 1MB string

    // Store and retrieve large value
    db.set("large_key", large_value.as_bytes()).unwrap();
    wait_for_file_sync();

    assert_eq!(db.get("large_key"), Some(large_value.as_bytes().to_vec()));
}

#[test]
fn test_log_compaction_comprehensive() {
    let temp_dir = tempdir().unwrap();
    let log_file = temp_dir.path().join("keystonelight.log");
    cleanup(log_file.to_str().unwrap());

    let db = Database::with_log_path(log_file.to_str().unwrap()).unwrap();

    // Phase 1: Create initial data
    for i in 0..10 {
        db.set(&format!("key{}", i), format!("value{}", i).as_bytes())
            .unwrap();
    }
    wait_for_file_sync();

    // Phase 2: Update some keys multiple times
    for i in 0..5 {
        for j in 0..3 {
            db.set(
                &format!("key{}", i),
                format!("updated_value{}_{}", i, j).as_bytes(),
            )
            .unwrap();
        }
    }
    wait_for_file_sync();

    // Phase 3: Delete some keys
    for i in 5..8 {
        db.delete(&format!("key{}", i)).unwrap();
    }
    wait_for_file_sync();

    // Get initial file size
    let initial_size = fs::metadata(&log_file).unwrap().len();

    // Phase 4: Trigger compaction
    db.compact().unwrap();
    wait_for_file_sync();

    // Get compacted file size
    let compacted_size = fs::metadata(&log_file).unwrap().len();

    // Verify compaction reduced file size
    assert!(
        compacted_size < initial_size,
        "Compaction did not reduce file size (initial: {}, compacted: {})",
        initial_size,
        compacted_size
    );

    // Phase 5: Verify data integrity after compaction
    // Check updated keys
    for i in 0..5 {
        let key = format!("key{}", i);
        let expected_value = format!("updated_value{}_{}", i, 2); // Last update
        assert_eq!(db.get(&key), Some(expected_value.as_bytes().to_vec()));
    }

    // Check unchanged keys
    for i in 8..10 {
        let key = format!("key{}", i);
        let expected_value = format!("value{}", i);
        assert_eq!(db.get(&key), Some(expected_value.as_bytes().to_vec()));
    }

    // Check deleted keys
    for i in 5..8 {
        let key = format!("key{}", i);
        assert_eq!(db.get(&key), None);
    }

    // Phase 6: Verify database can be reopened after compaction
    drop(db);
    wait_for_file_sync();

    let db = Database::with_log_path(log_file.to_str().unwrap()).unwrap();

    // Verify all data is still correct after reopening
    for i in 0..5 {
        let key = format!("key{}", i);
        let expected_value = format!("updated_value{}_{}", i, 2);
        assert_eq!(db.get(&key), Some(expected_value.as_bytes().to_vec()));
    }

    for i in 8..10 {
        let key = format!("key{}", i);
        let expected_value = format!("value{}", i);
        assert_eq!(db.get(&key), Some(expected_value.as_bytes().to_vec()));
    }

    for i in 5..8 {
        let key = format!("key{}", i);
        assert_eq!(db.get(&key), None);
    }
}
