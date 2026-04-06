//! Worker pool placeholder.

/// Pool for managing workers.
pub struct WorkerPool;

impl WorkerPool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WorkerPool {
    fn default() -> Self {
        Self::new()
    }
}
