use std::collections::HashMap;

use crate::error::KryonError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    String(String),
}

#[derive(Debug, Default)]
pub struct Store {
    data: HashMap<String, Value>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: Value) -> Result<(), KryonError> {
        let key = key.into();

        if key.trim().is_empty() {
            return Err(KryonError::EmptyKey);
        }

        self.data.insert(key, value);
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    pub fn delete(&mut self, key: &str) -> bool {
        self.data.remove(key).is_some()
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get() {
        let mut store = Store::new();
        store.set("name", Value::String("Kryon".into())).unwrap();

        assert_eq!(store.get("name"), Some(&Value::String("Kryon".into())));
    }

    #[test]
    fn update_existing_key() {
        let mut store = Store::new();

        store.set("name", Value::String("Old".into())).unwrap();
        store.set("name", Value::String("New".into())).unwrap();

        assert_eq!(store.get("name"), Some(&Value::String("New".into())));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn delete_key() {
        let mut store = Store::new();

        store.set("name", Value::String("Kryon".into())).unwrap();

        assert!(store.delete("name"));
        assert!(!store.exists("name"));
        assert_eq!(store.get("name"), None);
    }

    #[test]
    fn exists() {
        let mut store = Store::new();

        assert!(!store.exists("name"));

        store.set("name", Value::String("Kryon".into())).unwrap();

        assert!(store.exists("name"));
    }

    #[test]
    fn empty_key_is_rejected() {
        let mut store = Store::new();

        assert_eq!(
            store.set("", Value::String("Kryon".into())),
            Err(KryonError::EmptyKey)
        );
    }

    #[test]
    fn whitespace_key_is_rejected() {
        let mut store = Store::new();

        assert_eq!(
            store.set("   ", Value::String("Kryon".into())),
            Err(KryonError::EmptyKey)
        );
    }

    #[test]
    fn clear_store() {
        let mut store = Store::new();

        store.set("a", Value::String("1".into())).unwrap();
        store.set("b", Value::String("2".into())).unwrap();

        store.clear();

        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }
}
