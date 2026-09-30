/*
    FILE: persistence/mod.rs

    PURPOSE:
    Exposes Kryon's persistence components.

    WORKFLOW:

    Storage
      ↓
    Persistence
      ├── WAL
      │    ↓
      │   durable mutation log
      │
      └── Snapshot
           ↓
          point-in-time database state
*/

pub mod snapshot;
pub mod wal;
