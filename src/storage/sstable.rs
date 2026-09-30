// SSTable — immutable on-disk representation of sorted key/value data.
//
// Workflow:
// ImmutableMemTable → SSTable::write() → disk
// disk → SSTable::read() → sorted entries
//
// Format:
// [magic: 4 bytes]
// [version: u8]
// repeated entries:
//   [key_len: u32 LE]
//   [value_len: u32 LE]
//   [key bytes]
//   [value bytes]

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use crate::error::KryonError;
use crate::storage::Value;
use crate::storage::memtable::ImmutableMemTable;

const MAGIC: &[u8; 4] = b"RKV1";
const VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sstable {
    entries: Vec<(String, Value)>,
}

impl Sstable {
    pub fn write<P: AsRef<Path>>(table: &ImmutableMemTable, path: P) -> Result<(), KryonError> {
        let path = path.as_ref();

        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let file_name = path
            .file_name()
            .ok_or_else(|| KryonError::Io("SSTable path has no file name".to_string()))?;

        let temp_name = format!(
            ".{}.tmp-{}",
            file_name.to_string_lossy(),
            std::process::id()
        );
        let temp_path = parent.join(temp_name);

        let result = (|| -> Result<(), KryonError> {
            let mut file = File::create(&temp_path).map_err(|e| KryonError::Io(e.to_string()))?;

            file.write_all(MAGIC)
                .map_err(|e| KryonError::Io(e.to_string()))?;
            file.write_all(&[VERSION])
                .map_err(|e| KryonError::Io(e.to_string()))?;

            for (key, value) in table.iter() {
                let key_bytes = key.as_bytes();

                let (value_type, value_bytes) = match value {
                    Value::String(value) => (0u8, value.as_bytes()),
                    Value::Tombstone => (1u8, &[] as &[u8]),
                };

                let key_len = u32::try_from(key_bytes.len()).map_err(|_| {
                    KryonError::Io(
                        io::Error::new(io::ErrorKind::InvalidInput, "key is too large").to_string(),
                    )
                })?;

                let value_len = u32::try_from(value_bytes.len()).map_err(|_| {
                    KryonError::Io(
                        io::Error::new(io::ErrorKind::InvalidInput, "value is too large")
                            .to_string(),
                    )
                })?;

                file.write_all(&key_len.to_le_bytes())
                    .map_err(|e| KryonError::Io(e.to_string()))?;
                file.write_all(&[value_type])
                    .map_err(|e| KryonError::Io(e.to_string()))?;
                file.write_all(&value_len.to_le_bytes())
                    .map_err(|e| KryonError::Io(e.to_string()))?;
                file.write_all(key_bytes)
                    .map_err(|e| KryonError::Io(e.to_string()))?;
                file.write_all(value_bytes)
                    .map_err(|e| KryonError::Io(e.to_string()))?;
            }

            file.flush().map_err(|e| KryonError::Io(e.to_string()))?;
            file.sync_all().map_err(|e| KryonError::Io(e.to_string()))?;

            std::fs::rename(&temp_path, path).map_err(|e| KryonError::Io(e.to_string()))?;

            Ok(())
        })();

        if result.is_err() {
            let _ = std::fs::remove_file(&temp_path);
        }

        result
    }

    pub fn read<P: AsRef<Path>>(path: P) -> Result<Self, KryonError> {
        let mut file = File::open(path).map_err(|e| KryonError::Io(e.to_string()))?;
        let mut bytes = Vec::new();

        file.read_to_end(&mut bytes)
            .map_err(|e| KryonError::Io(e.to_string()))?;

        if bytes.len() < 5 {
            return Err(KryonError::Io(
                io::Error::new(io::ErrorKind::InvalidData, "SSTable header is incomplete")
                    .to_string(),
            ));
        }

        if &bytes[0..4] != MAGIC {
            return Err(KryonError::Io(
                io::Error::new(io::ErrorKind::InvalidData, "invalid SSTable magic").to_string(),
            ));
        }

        if bytes[4] != VERSION {
            return Err(KryonError::Io(
                io::Error::new(io::ErrorKind::InvalidData, "unsupported SSTable version")
                    .to_string(),
            ));
        }

        let mut offset = 5;
        let mut entries = Vec::new();

        while offset < bytes.len() {
            // Each entry:
            // [key_len: u32][value_type: u8][value_len: u32][key][value]

            if bytes.len() - offset < 9 {
                return Err(KryonError::Io(
                    io::Error::new(io::ErrorKind::InvalidData, "truncated SSTable entry header")
                        .to_string(),
                ));
            }

            let key_len =
                u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;

            let value_type = bytes[offset + 4];

            let value_len =
                u32::from_le_bytes(bytes[offset + 5..offset + 9].try_into().unwrap()) as usize;

            offset += 9;

            let entry_len = key_len.checked_add(value_len).ok_or_else(|| {
                KryonError::Io(
                    io::Error::new(io::ErrorKind::InvalidData, "SSTable entry length overflow")
                        .to_string(),
                )
            })?;

            if bytes.len() - offset < entry_len {
                return Err(KryonError::Io(
                    io::Error::new(io::ErrorKind::InvalidData, "truncated SSTable entry")
                        .to_string(),
                ));
            }

            let key =
                String::from_utf8(bytes[offset..offset + key_len].to_vec()).map_err(|_| {
                    KryonError::Io(
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "SSTable contains invalid UTF-8 key",
                        )
                        .to_string(),
                    )
                })?;

            offset += key_len;

            let value = match value_type {
                0 => {
                    let value = String::from_utf8(bytes[offset..offset + value_len].to_vec())
                        .map_err(|_| {
                            KryonError::Io(
                                io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "SSTable contains invalid UTF-8 value",
                                )
                                .to_string(),
                            )
                        })?;

                    Value::String(value)
                }

                1 => {
                    if value_len != 0 {
                        return Err(KryonError::Io(
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "invalid tombstone value length",
                            )
                            .to_string(),
                        ));
                    }

                    Value::Tombstone
                }

                _ => {
                    return Err(KryonError::Io(
                        io::Error::new(io::ErrorKind::InvalidData, "unknown SSTable value type")
                            .to_string(),
                    ));
                }
            };

            offset += value_len;

            entries.push((key, value));
        }

        Ok(Self { entries })
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries
            .iter()
            .find(|(entry_key, _)| entry_key == key)
            .map(|(_, value)| value)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.entries.iter().map(|(key, value)| (key, value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memtable::MemTable;

    #[test]
    fn sstable_round_trip() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("kryon-sstable-{}.db", std::process::id()));

        let mut table = MemTable::new();

        table
            .set("b".to_string(), Value::String("two".to_string()))
            .unwrap();

        table
            .set("a".to_string(), Value::String("one".to_string()))
            .unwrap();

        let frozen = table.freeze();

        Sstable::write(&frozen, &path).unwrap();

        let loaded = Sstable::read(&path).unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.get("a"), Some(&Value::String("one".to_string())));
        assert_eq!(loaded.get("b"), Some(&Value::String("two".to_string())));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn atomic_write_leaves_only_final_sstable() {
        let temp_dir =
            std::env::temp_dir().join(format!("kryon-atomic-write-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let path = temp_dir.join("data.sst");

        let mut table = MemTable::new();
        table
            .set("key".to_string(), Value::String("value".to_string()))
            .unwrap();

        let frozen = table.freeze();

        Sstable::write(&frozen, &path).unwrap();

        assert!(path.exists());
        assert!(!temp_dir.join(".data.sst.tmp-").exists());

        let temp_files: Vec<_> = std::fs::read_dir(&temp_dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();

        assert_eq!(temp_files, vec![path.clone()]);

        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(temp_dir).unwrap();
    }

    #[test]
    fn tombstone_round_trip() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("kryon-tombstone-sstable-{}.db", std::process::id()));

        let mut table = MemTable::new();

        table.set("deleted".to_string(), Value::Tombstone).unwrap();

        table
            .set("alive".to_string(), Value::String("value".to_string()))
            .unwrap();

        let frozen = table.freeze();

        Sstable::write(&frozen, &path).unwrap();

        let loaded = Sstable::read(&path).unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.get("deleted"), Some(&Value::Tombstone));
        assert_eq!(
            loaded.get("alive"),
            Some(&Value::String("value".to_string()))
        );

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn unknown_sstable_value_type_is_rejected() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "kryon-unknown-value-type-{}.db",
            std::process::id()
        ));

        // Header + key_len(1) + unknown type(9) + value_len(0) + key("x")
        let bytes = b"RKV1\x01\x01\x09\x00\x00\x00x";
        std::fs::write(&path, bytes).unwrap();

        assert!(Sstable::read(&path).is_err());

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn tombstone_with_value_bytes_is_rejected() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("kryon-invalid-tombstone-{}.db", std::process::id()));

        // Header + key_len(1) + tombstone type(1) + value_len(1) + key("x") + value("y")
        let bytes = b"RKV1\x01\x01\x01\x01\x00\x00\x00xy";
        std::fs::write(&path, bytes).unwrap();

        assert!(Sstable::read(&path).is_err());

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn invalid_sstable_is_rejected() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("kryon-invalid-sstable-{}.db", std::process::id()));

        std::fs::write(&path, b"BAD!").unwrap();

        assert!(Sstable::read(&path).is_err());

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn truncated_sstable_is_rejected() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("kryon-truncated-sstable-{}.db", std::process::id()));

        std::fs::write(&path, b"RKV1\x01\x01").unwrap();

        assert!(Sstable::read(&path).is_err());

        std::fs::remove_file(path).unwrap();
    }
}
