/*
    FILE: storage.rs

    PURPOSE:
    Core in-memory key-value storage for Kryon.

    WORKFLOW:

    Command
       ↓
    Store
       ↓
    Key validation
       ↓
    TTL check
       ↓
    HashMap
       ↓
    Result

    PERSISTENT WORKFLOW:

    Command
       ↓
    WAL
       ↓
    Store mutation
       ↓
    Response

    RECOVERY:

    WAL file
       ↓
    Replay records
       ↓
    Rebuild Store
       ↓
    Kryon ready

    Store::new()
    → Creates an in-memory store without persistence.

    Store::open(path)
    → Opens a WAL and rebuilds the store from existing records.
*/

use std::collections::HashMap;

use super::lsm::LsmEngine;
use std::io;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::persistence::wal::{Wal, WalRecord};

use crate::error::KryonError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    String(String),
    Tombstone,
}

pub struct Store {
    lsm: LsmEngine,
    expirations: HashMap<String, SystemTime>,
    wal: Option<Wal>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            lsm: LsmEngine::new(),
            expirations: HashMap::new(),
            wal: None,
        }
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

impl Store {
    pub fn open(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
        let wal = Wal::open(path)?;
        let records = wal.replay()?;

        let mut store = Self {
            lsm: LsmEngine::new(),
            expirations: HashMap::new(),
            wal: Some(wal),
        };

        for record in records {
            store.apply_record(record);
        }

        Ok(store)
    }

    fn apply_record(&mut self, record: WalRecord) {
        match record {
            WalRecord::Set { key, value } => {
                self.lsm
                    .set(key.clone(), Value::String(value))
                    .expect("WAL replay produced an invalid key");
                self.expirations.remove(&key);
            }

            WalRecord::Delete { key } => {
                self.lsm.delete(&key);
                self.expirations.remove(&key);
            }

            WalRecord::Clear => {
                self.lsm.clear();
                self.expirations.clear();
            }

            WalRecord::Expire { key, deadline_ms } => {
                if self.lsm.exists(&key) {
                    self.expirations
                        .insert(key, unix_ms_to_system_time(deadline_ms));
                }
            }
        }
    }

    fn validate_key(key: &str) -> Result<(), KryonError> {
        if key.trim().is_empty() {
            return Err(KryonError::EmptyKey);
        }

        Ok(())
    }

    fn append(&mut self, record: &WalRecord) -> Result<(), KryonError> {
        if let Some(wal) = &mut self.wal {
            wal.append(record)
                .map_err(|error| KryonError::Io(error.to_string()))?;
        }

        Ok(())
    }

    pub fn set(&mut self, key: String, value: Value) -> Result<(), KryonError> {
        Self::validate_key(&key)?;

        let value_string = match &value {
            Value::String(value) => value.clone(),
            Value::Tombstone => {
                return Err(KryonError::Io(
                    "tombstone cannot be stored through Store::set".into(),
                ));
            }
        };

        self.append(&WalRecord::Set {
            key: key.clone(),
            value: value_string,
        })?;

        self.lsm.set(key.clone(), value)?;
        self.expirations.remove(&key);

        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        if self.is_expired(key) {
            return None;
        }

        self.lsm.get(key)
    }

    fn is_expired(&self, key: &str) -> bool {
        self.expirations
            .get(key)
            .is_some_and(|deadline| SystemTime::now() >= *deadline)
    }

    pub fn expire(&mut self, key: &str, seconds: u64) -> bool {
        if !self.lsm.exists(key) || self.is_expired(key) {
            return false;
        }

        let deadline = SystemTime::now() + Duration::from_secs(seconds);
        let deadline_ms = system_time_to_unix_ms(deadline);

        if self
            .append(&WalRecord::Expire {
                key: key.to_string(),
                deadline_ms,
            })
            .is_err()
        {
            return false;
        }

        self.expirations.insert(key.to_string(), deadline);

        true
    }

    pub fn ttl(&self, key: &str) -> i64 {
        if !self.lsm.exists(key) || self.is_expired(key) {
            return -2;
        }

        let Some(deadline) = self.expirations.get(key) else {
            return -1;
        };

        deadline
            .duration_since(SystemTime::now())
            .map(|remaining| remaining.as_secs() as i64)
            .unwrap_or(-2)
    }

    pub fn delete(&mut self, key: &str) -> bool {
        if !self.lsm.exists(key) {
            return false;
        }

        if self
            .append(&WalRecord::Delete {
                key: key.to_string(),
            })
            .is_err()
        {
            return false;
        }

        self.expirations.remove(key);
        self.lsm.delete(key).is_some()
    }

    pub fn exists(&self, key: &str) -> bool {
        !self.is_expired(key) && self.lsm.exists(key)
    }

    pub fn clear(&mut self) {
        if self.append(&WalRecord::Clear).is_err() {
            return;
        }

        self.lsm.clear();
        self.expirations.clear();
    }

    pub fn len(&self) -> usize {
        self.lsm.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn system_time_to_unix_ms(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn unix_ms_to_system_time(ms: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get() {
        let mut store = Store::new();

        store
            .set("name".into(), Value::String("Kryon".into()))
            .unwrap();

        assert_eq!(store.get("name"), Some(Value::String("Kryon".into())));
    }

    #[test]
    fn update_existing_key() {
        let mut store = Store::new();

        store
            .set("name".into(), Value::String("Kryon".into()))
            .unwrap();

        store
            .set("name".into(), Value::String("Rust".into()))
            .unwrap();

        assert_eq!(store.get("name"), Some(Value::String("Rust".into())));
    }

    #[test]
    fn delete_key() {
        let mut store = Store::new();

        store
            .set("name".into(), Value::String("Kryon".into()))
            .unwrap();

        assert!(store.delete("name"));
        assert!(!store.exists("name"));
    }

    #[test]
    fn exists() {
        let mut store = Store::new();

        store
            .set("name".into(), Value::String("Kryon".into()))
            .unwrap();

        assert!(store.exists("name"));
        assert!(!store.exists("missing"));
    }

    #[test]
    fn clear_store() {
        let mut store = Store::new();

        store.set("a".into(), Value::String("1".into())).unwrap();

        store.set("b".into(), Value::String("2".into())).unwrap();

        store.clear();

        assert_eq!(store.len(), 0);
    }

    #[test]
    fn empty_key_is_rejected() {
        let mut store = Store::new();

        assert_eq!(
            store.set("".into(), Value::String("value".into())),
            Err(KryonError::EmptyKey)
        );
    }

    #[test]
    fn whitespace_key_is_rejected() {
        let mut store = Store::new();

        assert_eq!(
            store.set("   ".into(), Value::String("value".into())),
            Err(KryonError::EmptyKey)
        );
    }
}
