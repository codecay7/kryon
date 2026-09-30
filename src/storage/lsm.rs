// LSM storage engine.
//
// Workflow:
// Command → Store → LsmEngine → MemTable → SSTable
//
// The engine owns the active MemTable, frozen MemTable,
// SSTable read layers, byte accounting, and flush decisions.
//
// WAL and TTL remain owned by Store.
// Compaction, indexing, and background flushing are deferred.

use crate::error::KryonError;
use std::path::{Path, PathBuf};

use super::Value;
use super::memtable::{ImmutableMemTable, MemTable};
use super::sstable::Sstable;

#[derive(Debug, Clone, PartialEq, Eq)]
enum LookupResult {
    Found(Value),
    Deleted,
    Missing,
}

#[derive(Debug, Default)]
pub struct LsmEngine {
    active: MemTable,
    flushing: Option<ImmutableMemTable>,
    active_bytes: usize,
    flush_threshold: usize,
    sstables: Vec<PathBuf>,
}

impl LsmEngine {
    pub const DEFAULT_FLUSH_THRESHOLD: usize = 4 * 1024 * 1024;

    pub fn new() -> Self {
        Self::with_flush_threshold(Self::DEFAULT_FLUSH_THRESHOLD)
    }

    pub fn with_flush_threshold(flush_threshold: usize) -> Self {
        Self {
            active: MemTable::default(),
            flushing: None,
            active_bytes: 0,
            flush_threshold,
            sstables: Vec::new(),
        }
    }

    pub fn set(&mut self, key: String, value: Value) -> Result<(), KryonError> {
        let previous_bytes = self
            .active
            .get(&key)
            .map(|old_value| entry_size(&key, old_value))
            .unwrap_or(0);

        let new_bytes = entry_size(&key, &value);

        self.active.set(key, value)?;

        self.active_bytes = self
            .active_bytes
            .saturating_sub(previous_bytes)
            .saturating_add(new_bytes);

        Ok(())
    }
    pub fn get(&self, key: &str) -> Option<Value> {
        match self.lookup(key) {
            LookupResult::Found(value) => Some(value),
            LookupResult::Deleted | LookupResult::Missing => None,
        }
    }

    fn lookup(&self, key: &str) -> LookupResult {
        if let Some(value) = self.active.get(key) {
            return match value {
                Value::Tombstone => LookupResult::Deleted,
                _ => LookupResult::Found(value.clone()),
            };
        }

        if let Some(table) = self.flushing.as_ref() {
            if let Some(value) = table.get(key) {
                return match value {
                    Value::Tombstone => LookupResult::Deleted,
                    _ => LookupResult::Found(value.clone()),
                };
            }
        }

        for path in self.sstables.iter().rev() {
            let Ok(table) = Sstable::read(path) else {
                continue;
            };

            if let Some(value) = table.get(key) {
                return match value {
                    Value::Tombstone => LookupResult::Deleted,
                    _ => LookupResult::Found(value.clone()),
                };
            }
        }

        LookupResult::Missing
    }

    pub fn exists(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn delete(&mut self, key: &str) -> Option<Value> {
        let previous = self.get(key);

        previous.as_ref()?;

        self.active
            .set(key.to_string(), Value::Tombstone)
            .expect("validated key must be accepted by MemTable");

        Some(Value::Tombstone)
    }

    pub fn clear(&mut self) {
        self.active.clear();
        self.flushing = None;
        self.active_bytes = 0;
        self.sstables.clear();
    }

    pub fn len(&self) -> usize {
        use std::collections::HashSet;

        let mut seen = HashSet::new();
        let mut live = 0;

        // Newer layers override older layers.
        for (key, value) in self.active.iter() {
            if seen.insert(key.clone()) && !matches!(value, Value::Tombstone) {
                live += 1;
            }
        }

        if let Some(table) = self.flushing.as_ref() {
            for (key, value) in table.iter() {
                if seen.insert(key.clone()) && !matches!(value, Value::Tombstone) {
                    live += 1;
                }
            }
        }

        for path in self.sstables.iter().rev() {
            if let Ok(table) = Sstable::read(path) {
                for (key, value) in table.iter() {
                    if seen.insert(key.clone()) && !matches!(value, Value::Tombstone) {
                        live += 1;
                    }
                }
            }
        }

        live
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn flush_active<P: AsRef<Path>>(&mut self, path: P) -> Result<(), KryonError> {
        if self.active.is_empty() {
            return Ok(());
        }

        let active = std::mem::take(&mut self.active);
        let immutable = active.freeze();

        Sstable::write(&immutable, path.as_ref())?;

        self.flushing = Some(immutable);
        self.active_bytes = 0;
        self.sstables.push(path.as_ref().to_path_buf());

        Ok(())
    }

    pub fn freeze_active(&mut self) {
        let active = std::mem::take(&mut self.active);
        self.flushing = Some(active.freeze());
        self.active_bytes = 0;
    }

    pub fn has_flushing_table(&self) -> bool {
        self.flushing.is_some()
    }

    pub fn active_len(&self) -> usize {
        self.active.len()
    }

    pub fn active_bytes(&self) -> usize {
        self.active_bytes
    }

    pub fn flush_threshold(&self) -> usize {
        self.flush_threshold
    }

    pub fn should_flush(&self) -> bool {
        self.active_bytes >= self.flush_threshold
    }

    pub fn sstable_count(&self) -> usize {
        self.sstables.len()
    }
}

fn value_size(value: &Value) -> usize {
    match value {
        Value::String(value) => value.len(),
        Value::Tombstone => 0,
    }
}

fn entry_size(key: &str, value: &Value) -> usize {
    key.len() + value_size(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_reads_from_active_table() {
        let mut engine = LsmEngine::new();

        engine
            .set("name".into(), Value::String("Kryon".into()))
            .unwrap();

        assert_eq!(engine.get("name"), Some(Value::String("Kryon".into())));
    }

    #[test]
    fn frozen_table_remains_readable() {
        let mut engine = LsmEngine::new();

        engine
            .set("name".into(), Value::String("Kryon".into()))
            .unwrap();

        engine.freeze_active();

        assert!(engine.has_flushing_table());
        assert_eq!(engine.get("name"), Some(Value::String("Kryon".into())));
        assert_eq!(engine.active_len(), 0);
    }

    #[test]
    fn new_writes_override_frozen_data() {
        let mut engine = LsmEngine::new();

        engine
            .set("name".into(), Value::String("old".into()))
            .unwrap();

        engine.freeze_active();

        engine
            .set("name".into(), Value::String("new".into()))
            .unwrap();

        assert_eq!(engine.get("name"), Some(Value::String("new".into())));
    }

    #[test]
    fn byte_accounting_tracks_new_value_size() {
        let mut engine = LsmEngine::new();

        engine
            .set("key".into(), Value::String("value".into()))
            .unwrap();

        assert_eq!(engine.active_bytes(), 8);
    }

    #[test]
    fn updating_key_replaces_previous_byte_count() {
        let mut engine = LsmEngine::new();

        engine
            .set("key".into(), Value::String("long-value".into()))
            .unwrap();

        engine.set("key".into(), Value::String("x".into())).unwrap();

        assert_eq!(engine.active_bytes(), 4);
    }

    #[test]
    fn threshold_controls_flush_decision() {
        let mut engine = LsmEngine::with_flush_threshold(9);

        engine
            .set("key".into(), Value::String("value".into()))
            .unwrap();

        assert!(!engine.should_flush());

        engine.set("x".into(), Value::String("y".into())).unwrap();

        assert!(engine.should_flush());
    }

    #[test]
    fn freezing_resets_active_byte_count() {
        let mut engine = LsmEngine::new();

        engine
            .set("key".into(), Value::String("value".into()))
            .unwrap();

        assert!(engine.active_bytes() > 0);

        engine.freeze_active();

        assert_eq!(engine.active_bytes(), 0);
        assert!(engine.has_flushing_table());
    }

    #[test]
    fn get_reads_from_sstable() {
        let dir = std::env::temp_dir().join(format!("kryon-lsm-read-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let path = dir.join("000001.sst");

        let mut engine = LsmEngine::new();

        engine
            .set("name".into(), Value::String("Kryon".into()))
            .unwrap();

        engine.flush_active(&path).unwrap();

        // Remove the in-memory layers so the SSTable is the only source.
        engine.flushing = None;

        assert_eq!(engine.get("name"), Some(Value::String("Kryon".into())));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn flush_writes_sstable() {
        let dir = std::env::temp_dir().join(format!("kryon-lsm-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let path = dir.join("000001.sst");

        let mut engine = LsmEngine::new();

        engine
            .set("name".into(), Value::String("Kryon".into()))
            .unwrap();

        engine.flush_active(&path).unwrap();

        assert!(path.exists());
        assert_eq!(engine.active_len(), 0);
        assert_eq!(engine.active_bytes(), 0);
        assert_eq!(engine.sstable_count(), 1);
        assert!(engine.has_flushing_table());

        let table = Sstable::read(&path).unwrap();

        assert_eq!(table.get("name"), Some(&Value::String("Kryon".into())));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn delete_only_affects_active_table_for_now() {
        let mut engine = LsmEngine::new();

        engine
            .set("name".into(), Value::String("Kryon".into()))
            .unwrap();

        assert!(engine.delete("name").is_some());
        assert!(!engine.exists("name"));
    }
}

#[cfg(test)]
mod delete_tombstone_tests {
    use super::*;
    use crate::storage::Value;

    #[test]
    fn delete_creates_tombstone_for_flushed_key() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("kryon-delete-tombstone-{}.sst", std::process::id()));

        let mut engine = LsmEngine::with_flush_threshold(1);

        engine
            .set("key".into(), Value::String("value".into()))
            .unwrap();

        engine.flush_active(&path).unwrap();

        assert_eq!(engine.get("key"), Some(Value::String("value".into())));

        engine.delete("key");

        // Tombstone is stored internally, but reads must expose the key as deleted.
        assert_eq!(engine.active.get("key"), Some(&Value::Tombstone));
        assert_eq!(engine.get("key"), None);
        assert!(!engine.exists("key"));

        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn tombstone_hides_flushed_value() {
    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join(format!("kryon-tombstone-hide-{}.sst", std::process::id()));

    let mut engine = LsmEngine::with_flush_threshold(1);

    engine
        .set("key".into(), Value::String("value".into()))
        .unwrap();

    engine.flush_active(&path).unwrap();

    engine.delete("key");

    assert_eq!(engine.get("key"), None);
    assert!(!engine.exists("key"));

    std::fs::remove_file(path).unwrap();
}

#[test]
fn persisted_tombstone_hides_older_sstable_value() {
    let temp_dir = std::env::temp_dir();
    let old_path = temp_dir.join(format!("kryon-old-{}.sst", std::process::id()));
    let tombstone_path = temp_dir.join(format!("kryon-tombstone-{}.sst", std::process::id()));

    let mut engine = LsmEngine::with_flush_threshold(1);

    engine
        .set("key".into(), Value::String("old".into()))
        .unwrap();
    engine.flush_active(&old_path).unwrap();

    engine.delete("key");
    engine.flush_active(&tombstone_path).unwrap();

    assert_eq!(engine.get("key"), None);
    assert!(!engine.exists("key"));

    std::fs::remove_file(old_path).unwrap();
    std::fs::remove_file(tombstone_path).unwrap();
}

#[test]
fn persisted_tombstone_blocks_older_value_through_newer_sstable() {
    let temp_dir = std::env::temp_dir().join(format!(
        "kryon-tombstone-regression-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let old_path = temp_dir.join("old.sst");
    let tombstone_path = temp_dir.join("tombstone.sst");
    let newer_path = temp_dir.join("newer.sst");

    let mut engine = LsmEngine::with_flush_threshold(1);

    engine
        .set("key".into(), Value::String("old".into()))
        .unwrap();
    engine.flush_active(&old_path).unwrap();

    engine.delete("key");
    engine.flush_active(&tombstone_path).unwrap();

    engine
        .set("other".into(), Value::String("value".into()))
        .unwrap();
    engine.flush_active(&newer_path).unwrap();

    assert_eq!(engine.get("key"), None);
    assert!(!engine.exists("key"));
    assert_eq!(engine.get("other"), Some(Value::String("value".into())));

    std::fs::remove_file(old_path).unwrap();
    std::fs::remove_file(tombstone_path).unwrap();
    std::fs::remove_file(newer_path).unwrap();
    std::fs::remove_dir(temp_dir).unwrap();
}

#[cfg(test)]
mod len_tests {
    use super::*;
    use crate::storage::Value;

    #[test]
    fn len_counts_flushed_keys() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("kryon-len-flushed-{}.sst", std::process::id()));

        let mut engine = LsmEngine::with_flush_threshold(1);

        engine
            .set("key-1".into(), Value::String("one".into()))
            .unwrap();
        engine
            .set("key-2".into(), Value::String("two".into()))
            .unwrap();

        engine.flush_active(&path).unwrap();

        assert_eq!(engine.len(), 2);

        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn len_ignores_deleted_flushed_keys() {
    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join(format!("kryon-len-delete-{}.sst", std::process::id()));

    let mut engine = LsmEngine::with_flush_threshold(1);

    engine
        .set("key-1".into(), Value::String("one".into()))
        .unwrap();
    engine
        .set("key-2".into(), Value::String("two".into()))
        .unwrap();

    engine.flush_active(&path).unwrap();

    engine.delete("key-1");

    assert_eq!(engine.len(), 1);
    assert_eq!(engine.get("key-1"), None);
    assert_eq!(engine.get("key-2"), Some(Value::String("two".into())));

    std::fs::remove_file(path).unwrap();
}

#[test]
fn len_ignores_deleted_key_across_multiple_sstables() {
    let temp_dir = std::env::temp_dir();
    let path1 = temp_dir.join(format!("kryon-len-multi-1-{}.sst", std::process::id()));
    let path2 = temp_dir.join(format!("kryon-len-multi-2-{}.sst", std::process::id()));

    let mut engine = LsmEngine::new();

    engine
        .set("key-1".into(), Value::String("one".into()))
        .unwrap();
    engine
        .set("key-2".into(), Value::String("two".into()))
        .unwrap();
    engine.flush_active(&path1).unwrap();

    engine.delete("key-1");
    engine
        .set("key-3".into(), Value::String("three".into()))
        .unwrap();
    engine.flush_active(&path2).unwrap();

    assert_eq!(engine.len(), 2);
    assert_eq!(engine.get("key-1"), None);
    assert_eq!(engine.get("key-2"), Some(Value::String("two".into())));
    assert_eq!(engine.get("key-3"), Some(Value::String("three".into())));

    std::fs::remove_file(path1).unwrap();
    std::fs::remove_file(path2).unwrap();
}
