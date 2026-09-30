use kryon::storage::{Store, Value};

use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn basic_command_flow() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kryon"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .env(
            "KRYON_WAL_PATH",
            std::env::temp_dir().join(format!(
                "kryon-test-{}-{}.wal",
                std::process::id(),
                file!().replace("/", "_")
            )),
        )
        .spawn()
        .expect("failed to start Kryon");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"SET name Kryon\nGET name\nEXISTS name\nDEL name\nGET name\nQUIT\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Kryon"));
    assert!(stdout.contains("(nil)"));
}

#[test]
fn store_uses_lsm_storage_layer() {
    let mut store = Store::new();

    store
        .set("name".into(), Value::String("Kryon".into()))
        .unwrap();

    assert_eq!(store.get("name"), Some(Value::String("Kryon".into())));

    assert!(store.exists("name"));
    assert_eq!(store.len(), 1);
}

#[test]
fn store_update_works_through_lsm() {
    let mut store = Store::new();

    store
        .set("name".into(), Value::String("old".into()))
        .unwrap();

    store
        .set("name".into(), Value::String("new".into()))
        .unwrap();

    assert_eq!(store.get("name"), Some(Value::String("new".into())));

    assert_eq!(store.len(), 1);
}

#[test]
fn store_reads_data_after_lsm_flush() {
    use kryon::storage::{Store, Value};

    let mut store = Store::new();

    for i in 0..10 {
        store
            .set(format!("key-{i}"), Value::String("x".repeat(1024)))
            .unwrap();
    }

    assert_eq!(store.get("key-0"), Some(Value::String("x".repeat(1024))));

    assert_eq!(store.get("key-9"), Some(Value::String("x".repeat(1024))));
}

#[test]
fn store_len_remains_correct_after_lsm_flush_and_update() {
    use kryon::storage::{Store, Value};

    let mut store = Store::new();

    for i in 0..10 {
        store
            .set(format!("key-{i}"), Value::String("x".repeat(1024)))
            .unwrap();
    }

    assert_eq!(store.len(), 10);

    store
        .set("key-0".into(), Value::String("updated".into()))
        .unwrap();

    assert_eq!(store.len(), 10);
}

#[test]
fn store_delete_removes_flushed_value() {
    use kryon::storage::{Store, Value};

    let mut store = Store::new();

    for i in 0..10 {
        store
            .set(format!("key-{i}"), Value::String("x".repeat(1024)))
            .unwrap();
    }

    assert_eq!(store.get("key-0"), Some(Value::String("x".repeat(1024))));

    assert!(store.delete("key-0"));

    assert_eq!(store.get("key-0"), None);
    assert!(!store.exists("key-0"));
}
