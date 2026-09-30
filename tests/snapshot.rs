/*
    FILE: tests/snapshot.rs

    PURPOSE:
    Verifies snapshot persistence independently from the live server.

    WORKFLOW:

    Create state
      ↓
    Save snapshot
      ↓
    Remove in-memory state
      ↓
    Load snapshot
      ↓
    Verify recovered state
*/

use std::fs;

use kryon::persistence::snapshot::{Snapshot, SnapshotEntry};

#[test]
fn snapshot_survives_reopen() {
    let path = std::env::temp_dir().join(format!(
        "kryon-snapshot-integration-{}.db",
        std::process::id()
    ));

    let _ = fs::remove_file(&path);

    let entries = vec![
        SnapshotEntry {
            key: "name".into(),
            value: "Kryon".into(),
            expires_at_ms: None,
        },
        SnapshotEntry {
            key: "language".into(),
            value: "Rust".into(),
            expires_at_ms: Some(9_999_999),
        },
    ];

    Snapshot::save(&path, &entries, 42).unwrap();

    let recovered = Snapshot::load(&path).unwrap();

    assert_eq!(recovered.wal_offset, 42);
    assert_eq!(recovered.entries, entries);

    let _ = fs::remove_file(path);
}
