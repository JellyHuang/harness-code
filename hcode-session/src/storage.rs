//! Storage trait.

use crate::Session;

/// Storage backend for sessions.
pub trait Storage: Send + Sync {
    fn save(&self, session: &Session) -> Result<(), StorageError>;
    fn load(&self, id: &str) -> Result<Option<Session>, StorageError>;
    fn list(&self) -> Result<Vec<String>, StorageError>;
}

/// Storage error.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
