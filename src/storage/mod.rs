// Storage layer
//
// Command
//   ↓
// Store
//   ↓
// MemTable
//   ↓
// WAL / persistence
//
// `Store` remains the public storage API while the internal
// implementation is gradually split into MemTable/SSTable layers.

pub mod lsm;
pub mod memtable;
pub mod sstable;
pub mod store;

pub use store::{Store, Value};
