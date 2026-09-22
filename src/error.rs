use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum KryonError {
    EmptyKey,
}

impl fmt::Display for KryonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KryonError::EmptyKey => write!(f, "key cannot be empty"),
        }
    }
}

impl std::error::Error for KryonError {}
