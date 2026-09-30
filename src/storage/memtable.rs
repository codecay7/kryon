// MemTable — mutable in-memory storage layer.
//
// Write: Command → Store → MemTable
// Read:  Command → Store → MemTable
// Later: MemTable → Immutable MemTable → SSTable
//
// This layer owns key/value entries only.
// WAL, TTL, snapshots, and persistence remain outside it.

use std::collections::BTreeMap;

use crate::error::KryonError;
use crate::storage::Value;

#[derive(Debug, Default)]
pub struct MemTable {
    data: BTreeMap<String, Value>,
}

impl MemTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: String, value: Value) -> Result<(), KryonError> {
        if key.trim().is_empty() {
            return Err(KryonError::EmptyKey);
        }

        self.data.insert(key, value);
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    pub fn delete(&mut self, key: &str) -> Option<Value> {
        self.data.remove(key)
    }

    pub fn exists(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn iter_keys(&self) -> impl Iterator<Item = &String> {
        self.data.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.data.iter()
    }

    pub fn freeze(self) -> ImmutableMemTable {
        ImmutableMemTable { data: self.data }
    }
}

#[derive(Debug, Clone)]
pub struct ImmutableMemTable {
    data: BTreeMap<String, Value>,
}

impl ImmutableMemTable {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    pub fn exists(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.data.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get() {
        let mut table = MemTable::new();

        table
            .set("name".to_string(), Value::String("kryon".to_string()))
            .unwrap();

        assert_eq!(table.get("name"), Some(&Value::String("kryon".to_string())));
    }

    #[test]
    fn update_existing_key() {
        let mut table = MemTable::new();

        table
            .set("key".to_string(), Value::String("old".to_string()))
            .unwrap();

        table
            .set("key".to_string(), Value::String("new".to_string()))
            .unwrap();

        assert_eq!(table.get("key"), Some(&Value::String("new".to_string())));
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn delete_key() {
        let mut table = MemTable::new();

        table
            .set("key".to_string(), Value::String("value".to_string()))
            .unwrap();

        assert!(table.delete("key").is_some());
        assert!(!table.exists("key"));
        assert!(table.is_empty());
    }

    #[test]
    fn freeze_creates_immutable_table() {
        let mut table = MemTable::new();

        table
            .set("a".to_string(), Value::String("1".to_string()))
            .unwrap();

        table
            .set("b".to_string(), Value::String("2".to_string()))
            .unwrap();

        let frozen = table.freeze();

        assert_eq!(frozen.len(), 2);
        assert_eq!(frozen.get("a"), Some(&Value::String("1".to_string())));
        assert!(frozen.exists("b"));
    }

    #[test]
    fn empty_key_is_rejected() {
        let mut table = MemTable::new();

        assert_eq!(
            table.set("   ".to_string(), Value::String("value".to_string())),
            Err(KryonError::EmptyKey)
        );
    }
}
