/*
    FILE: tests/persistence.rs

    PURPOSE:
    Verifies Kryon's WAL can persist mutations and replay them after
    the original WAL writer is gone.

    WORKFLOW:

    Write records
       ↓
    Close WAL
       ↓
    Reopen WAL
       ↓
    Replay records
       ↓
    Verify recovered mutations
*/

use std::fs;

use kryon::persistence::wal::{Wal, WalRecord};

#[test]
fn wal_survives_reopen() {
    let path = std::env::temp_dir().join(format!(
        "kryon-persistence-integration-{}",
        std::process::id()
    ));

    let _ = fs::remove_file(&path);

    {
        let mut wal = Wal::open(&path).unwrap();

        wal.append(&WalRecord::Set {
            key: "name".into(),
            value: "Kryon".into(),
        })
        .unwrap();

        wal.append(&WalRecord::Set {
            key: "language".into(),
            value: "Rust".into(),
        })
        .unwrap();
    }

    let wal = Wal::open(&path).unwrap();
    let records = wal.replay().unwrap();

    assert_eq!(
        records,
        vec![
            WalRecord::Set {
                key: "name".into(),
                value: "Kryon".into()
            },
            WalRecord::Set {
                key: "language".into(),
                value: "Rust".into()
            }
        ]
    );

    let _ = fs::remove_file(path);
}

#[test]
fn lsm_store_data_survives_reopen() {
    use kryon::storage::{Store, Value};

    let path = std::env::temp_dir().join(format!("kryon-lsm-recovery-{}.wal", std::process::id()));

    {
        let mut store = Store::open(&path).unwrap();

        store
            .set("engine".into(), Value::String("lsm".into()))
            .unwrap();

        assert_eq!(store.get("engine"), Some(Value::String("lsm".into())));
    }

    {
        let store = Store::open(&path).unwrap();

        assert_eq!(store.get("engine"), Some(Value::String("lsm".into())));
    }

    std::fs::remove_file(path).unwrap();
}
