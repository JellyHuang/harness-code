//! JSON file storage for sessions.
//!
//! Implements the Storage trait using JSON files in the user's config directory.

use crate::{Session, Storage, StorageError};
use std::fs;
use std::path::PathBuf;

/// JSON file storage backend.
pub struct JsonStorage {
    /// Base directory for session files
    base_dir: PathBuf,
}

impl JsonStorage {
    /// Create new JSON storage with default path.
    pub fn new() -> Result<Self, StorageError> {
        let base_dir = get_session_dir()?;
        fs::create_dir_all(&base_dir).map_err(StorageError::Io)?;
        Ok(Self { base_dir })
    }

    /// Create JSON storage with custom path.
    pub fn with_path(base_dir: PathBuf) -> Result<Self, StorageError> {
        fs::create_dir_all(&base_dir).map_err(StorageError::Io)?;
        Ok(Self { base_dir })
    }

    /// Get file path for a session.
    fn session_path(&self, id: &str) -> PathBuf {
        self.base_dir.join(format!("{}.json", id))
    }
}

impl Default for JsonStorage {
    fn default() -> Self {
        Self::new().expect("Failed to create JsonStorage")
    }
}

impl Storage for JsonStorage {
    /// Save session to JSON file.
    fn save(&self, session: &Session) -> Result<(), StorageError> {
        let path = self.session_path(&session.id);
        let json = serde_json::to_string_pretty(session)?;
        fs::write(&path, json).map_err(StorageError::Io)?;
        Ok(())
    }

    /// Load session from JSON file.
    fn load(&self, id: &str) -> Result<Option<Session>, StorageError> {
        let path = self.session_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let json = fs::read_to_string(&path)?;
        let session: Session = serde_json::from_str(&json)?;
        Ok(Some(session))
    }

    /// List all session IDs.
    fn list(&self) -> Result<Vec<String>, StorageError> {
        let mut ids = Vec::new();
        if !self.base_dir.exists() {
            return Ok(ids);
        }

        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Some(stem) = path.file_stem() {
                    ids.push(stem.to_string_lossy().to_string());
                }
            }
        }

        ids.sort();
        Ok(ids)
    }
}

/// Get the default session storage directory.
pub fn get_session_dir() -> Result<PathBuf, StorageError> {
    let config_dir = dirs::config_dir().ok_or_else(|| {
        StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find config directory",
        ))
    })?;
    Ok(config_dir.join("hcode").join("sessions"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hcode_types::Message;
    use tempfile::tempdir;

    #[test]
    fn test_save_and_load() {
        let temp = tempdir().expect("Failed to create temp dir");
        let storage =
            JsonStorage::with_path(temp.path().to_path_buf()).expect("Failed to create storage");

        let session = Session {
            id: "test-session-id".to_string(),
            messages: vec![Message::user_text("Hello")],
        };

        storage.save(&session).expect("Failed to save");

        let loaded = storage.load("test-session-id").expect("Failed to load");
        assert!(loaded.is_some());

        let loaded_session = loaded.unwrap();
        assert_eq!(loaded_session.id, "test-session-id");
        assert_eq!(loaded_session.messages.len(), 1);
    }

    #[test]
    fn test_load_nonexistent() {
        let temp = tempdir().expect("Failed to create temp dir");
        let storage =
            JsonStorage::with_path(temp.path().to_path_buf()).expect("Failed to create storage");

        let loaded = storage.load("nonexistent").expect("Failed to load");
        assert!(loaded.is_none());
    }

    #[test]
    fn test_list_sessions() {
        let temp = tempdir().expect("Failed to create temp dir");
        let storage =
            JsonStorage::with_path(temp.path().to_path_buf()).expect("Failed to create storage");

        // Initially empty
        let ids = storage.list().expect("Failed to list");
        assert!(ids.is_empty());

        // Add some sessions
        storage
            .save(&Session {
                id: "session-1".to_string(),
                messages: vec![],
            })
            .expect("Failed to save");
        storage
            .save(&Session {
                id: "session-2".to_string(),
                messages: vec![],
            })
            .expect("Failed to save");
        storage
            .save(&Session {
                id: "session-3".to_string(),
                messages: vec![],
            })
            .expect("Failed to save");

        let ids = storage.list().expect("Failed to list");
        assert_eq!(ids.len(), 3);
        assert_eq!(ids, vec!["session-1", "session-2", "session-3"]);
    }

    #[test]
    fn test_get_session_dir() {
        let dir = get_session_dir().expect("Failed to get session dir");
        assert!(dir.ends_with("hcode") || dir.ends_with("sessions"));
    }
}
