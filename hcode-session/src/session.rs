//! Session types.

use hcode_types::Message;
use uuid::Uuid;

/// A conversation session.
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub messages: Vec<Message>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            messages: Vec::new(),
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}
