/*
    FILE: persistence/wal.rs

    PURPOSE:
    Implements Kryon's Write-Ahead Log (WAL).

    A WAL keeps a durable record of mutations before they are applied
    to the in-memory store.

    WRITE WORKFLOW:

    Command
       ↓
    WAL append
       ↓
    Flush to disk
       ↓
    Memory Store mutation
       ↓
    Response

    RECOVERY WORKFLOW:

    Process starts
       ↓
    Open WAL
       ↓
    Read records sequentially
       ↓
    Decode records
       ↓
    Replay mutations
       ↓
    Rebuild in-memory state

    CURRENT RECORDS:
    SET   → key + value
    DEL   → key
    CLEAR → no payload

    TTL persistence will be added separately once its durable
    time representation is finalized.
*/

use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalRecord {
    Set { key: String, value: String },
    Delete { key: String },
    Clear,
    Expire { key: String, deadline_ms: u64 },
}

pub struct Wal {
    path: PathBuf,
    file: File,
}

impl Wal {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;

        Ok(Self { path, file })
    }

    pub fn append(&mut self, record: &WalRecord) -> io::Result<()> {
        let line = encode(record);

        self.file.write_all(line.as_bytes())?;
        self.file.write_all(b"\n")?;
        self.file.flush()?;

        Ok(())
    }

    pub fn replay(&self) -> io::Result<Vec<WalRecord>> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);

        let mut records = Vec::new();

        for line in reader.lines() {
            let line = line?;

            if line.trim().is_empty() {
                continue;
            }

            records.push(decode(&line)?);
        }

        Ok(records)
    }
}

fn encode(record: &WalRecord) -> String {
    match record {
        WalRecord::Set { key, value } => {
            format!("SET\t{}\t{}", escape(key), escape(value))
        }
        WalRecord::Delete { key } => {
            format!("DEL\t{}", escape(key))
        }
        WalRecord::Clear => "CLEAR".to_string(),
        WalRecord::Expire { key, deadline_ms } => {
            format!("EXPIRE\t{}\t{}", escape(key), deadline_ms)
        }
    }
}

fn decode(line: &str) -> io::Result<WalRecord> {
    let parts: Vec<&str> = line.split('\t').collect();

    match parts.as_slice() {
        ["SET", key, value] => Ok(WalRecord::Set {
            key: unescape(key),
            value: unescape(value),
        }),

        ["DEL", key] => Ok(WalRecord::Delete { key: unescape(key) }),

        ["CLEAR"] => Ok(WalRecord::Clear),

        ["EXPIRE", key, deadline_ms] => Ok(WalRecord::Expire {
            key: unescape(key),
            deadline_ms: deadline_ms
                .parse()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid expiration"))?,
        }),

        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid WAL record",
        )),
    }
}

fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

fn unescape(value: &str) -> String {
    let mut output = String::new();
    let mut escaped = false;

    for ch in value.chars() {
        if escaped {
            match ch {
                't' => output.push('\t'),
                'n' => output.push('\n'),
                '\\' => output.push('\\'),
                other => {
                    output.push('\\');
                    output.push(other);
                }
            }

            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else {
            output.push(ch);
        }
    }

    if escaped {
        output.push('\\');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wal_round_trip() {
        let records = [
            WalRecord::Set {
                key: "name".into(),
                value: "Kryon".into(),
            },
            WalRecord::Set {
                key: "message".into(),
                value: "Hello\tKryon\nRust".into(),
            },
            WalRecord::Delete { key: "name".into() },
            WalRecord::Clear,
        ];

        for record in records {
            let encoded = encode(&record);
            let decoded = decode(&encoded).unwrap();
            assert_eq!(record, decoded);
        }
    }

    #[test]
    fn wal_writes_and_replays() {
        let path = std::env::temp_dir().join(format!("kryon-wal-test-{}", std::process::id()));

        let _ = std::fs::remove_file(&path);

        let mut wal = Wal::open(&path).unwrap();

        wal.append(&WalRecord::Set {
            key: "name".into(),
            value: "Kryon".into(),
        })
        .unwrap();

        wal.append(&WalRecord::Delete { key: "old".into() })
            .unwrap();

        wal.append(&WalRecord::Clear).unwrap();

        let records = wal.replay().unwrap();

        assert_eq!(records.len(), 3);

        let _ = std::fs::remove_file(path);
    }
}
