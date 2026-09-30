/*
    FILE: persistence/snapshot.rs

    PURPOSE:
    Stores a point-in-time copy of Kryon's current in-memory state.

    WORKFLOW:

    Store
      ↓
    Snapshot::save()
      ↓
    Temporary snapshot file
      ↓
    Flush + sync
      ↓
    Atomic rename
      ↓
    snapshot file on disk

    RECOVERY:

    snapshot file
      ↓
    Snapshot::load()
      ↓
    Restore key/value state
      ↓
    Replay newer WAL records

    SNAPSHOTS reduce recovery work by avoiding replay of the entire
    historical WAL.
*/

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotEntry {
    pub key: String,
    pub value: String,
    pub expires_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub entries: Vec<SnapshotEntry>,
    pub wal_offset: u64,
}

impl Snapshot {
    pub fn save(
        path: impl AsRef<Path>,
        entries: &[SnapshotEntry],
        wal_offset: u64,
    ) -> io::Result<()> {
        let path = path.as_ref();
        let temp_path = temp_path(path);

        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temp_path)?;

        writeln!(file, "KRYON_SNAPSHOT_V1")?;
        writeln!(file, "WAL_OFFSET\t{wal_offset}")?;

        for entry in entries {
            writeln!(
                file,
                "ENTRY\t{}\t{}\t{}",
                escape(&entry.key),
                escape(&entry.value),
                entry
                    .expires_at_ms
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string())
            )?;
        }

        file.flush()?;
        file.sync_all()?;
        drop(file);

        fs::rename(&temp_path, path)?;

        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = File::open(path)?;
        let mut lines = BufReader::new(file).lines();

        match lines.next() {
            Some(Ok(header)) if header == "KRYON_SNAPSHOT_V1" => {}
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid snapshot header",
                ));
            }
        }

        let wal_offset = match lines.next() {
            Some(Ok(line)) => parse_wal_offset(&line)?,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "missing WAL offset",
                ));
            }
        };

        let mut entries = Vec::new();

        for line in lines {
            let line = line?;

            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split('\t').collect();

            match parts.as_slice() {
                ["ENTRY", key, value, expires_at] => {
                    entries.push(SnapshotEntry {
                        key: unescape(key),
                        value: unescape(value),
                        expires_at_ms: if *expires_at == "-" {
                            None
                        } else {
                            Some(expires_at.parse().map_err(|_| {
                                io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "invalid expiration timestamp",
                                )
                            })?)
                        },
                    });
                }

                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid snapshot record",
                    ));
                }
            }
        }

        Ok(Self {
            entries,
            wal_offset,
        })
    }
}

fn parse_wal_offset(line: &str) -> io::Result<u64> {
    let parts: Vec<&str> = line.split('\t').collect();

    match parts.as_slice() {
        ["WAL_OFFSET", offset] => offset
            .parse()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid WAL offset")),

        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid WAL offset record",
        )),
    }
}

fn temp_path(path: &Path) -> PathBuf {
    let mut temp = path.as_os_str().to_os_string();
    temp.push(".tmp");
    PathBuf::from(temp)
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
    fn snapshot_round_trip() {
        let path = std::env::temp_dir().join(format!("kryon-snapshot-{}.db", std::process::id()));

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(temp_path(&path));

        let entries = vec![
            SnapshotEntry {
                key: "name".into(),
                value: "Kryon".into(),
                expires_at_ms: None,
            },
            SnapshotEntry {
                key: "message".into(),
                value: "Hello\tRust\nKryon".into(),
                expires_at_ms: Some(123456),
            },
        ];

        Snapshot::save(&path, &entries, 987).unwrap();

        let snapshot = Snapshot::load(&path).unwrap();

        assert_eq!(snapshot.wal_offset, 987);
        assert_eq!(snapshot.entries, entries);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn invalid_snapshot_is_rejected() {
        let path =
            std::env::temp_dir().join(format!("kryon-invalid-snapshot-{}.db", std::process::id()));

        fs::write(&path, "BAD_SNAPSHOT\n").unwrap();

        assert!(Snapshot::load(&path).is_err());

        let _ = fs::remove_file(path);
    }
}
