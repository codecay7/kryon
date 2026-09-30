#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Set { key: String, value: String },
    Get { key: String },
    Del { key: String },
    Exists { key: String },
    Clear,
    Len,
    Expire { key: String, seconds: u64 },
    Ttl { key: String },
}
