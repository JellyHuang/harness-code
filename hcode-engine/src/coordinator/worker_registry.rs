//! Worker registry for managing workers.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Worker status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerStatus {
    Running,
    Completed,
    Failed,
    Timeout,
    Cancelled,
}

impl std::fmt::Display for WorkerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkerStatus::Running => write!(f, "running"),
            WorkerStatus::Completed => write!(f, "completed"),
            WorkerStatus::Failed => write!(f, "failed"),
            WorkerStatus::Timeout => write!(f, "timeout"),
            WorkerStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Notification event from a worker.
#[derive(Debug, Clone)]
pub enum NotificationEvent {
    Started,
    Progress { message: String },
    ToolUse { tool: String, input: serde_json::Value },
    ToolResult { tool: String, result: String },
    Completed { result: String },
    Failed { error: String },
}

impl NotificationEvent {
    pub fn status(&self) -> &'static str {
        match self {
            NotificationEvent::Started => "started",
            NotificationEvent::Progress { .. } => "progress",
            NotificationEvent::ToolUse { .. } => "tool_use",
            NotificationEvent::ToolResult { .. } => "tool_result",
            NotificationEvent::Completed { .. } => "completed",
            NotificationEvent::Failed { .. } => "failed",
        }
    }

    pub fn summary(&self) -> String {
        match self {
            NotificationEvent::Started => "Worker started".to_string(),
            NotificationEvent::Progress { message } => message.clone(),
            NotificationEvent::ToolUse { tool, .. } => format!("Using tool: {}", tool),
            NotificationEvent::ToolResult { tool, .. } => format!("Tool result: {}", tool),
            NotificationEvent::Completed { result } => result.clone(),
            NotificationEvent::Failed { error } => format!("Error: {}", error),
        }
    }
}

/// Worker notification.
#[derive(Debug, Clone)]
pub struct WorkerNotification {
    pub worker_id: String,
    pub event: NotificationEvent,
    pub timestamp: DateTime<Utc>,
}

/// Message to send to a worker.
#[derive(Debug, Clone)]
pub enum WorkerMessage {
    Stop,
    Pause,
    Resume,
    UserInput { content: String },
    Custom { data: serde_json::Value },
}

/// Handle to a worker.
#[derive(Debug, Clone)]
pub struct WorkerHandle {
    /// Worker ID.
    pub id: String,
    
    /// Worker name/agent type.
    pub name: String,
    
    /// Current status.
    pub status: WorkerStatus,
    
    /// Channel to send messages to the worker.
    pub sender: mpsc::Sender<WorkerMessage>,
    
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    
    /// Prompt/task for the worker.
    pub prompt: String,
    
    /// Final result (if completed).
    pub result: Option<String>,
    
    /// Error message (if failed).
    pub error: Option<String>,
}

/// Worker registry for managing all workers.
#[derive(Debug)]
pub struct WorkerRegistry {
    /// Map of worker ID to handle.
    workers: RwLock<HashMap<String, WorkerHandle>>,
    
    /// Channel to receive notifications from workers.
    notification_rx: RwLock<Option<mpsc::Receiver<WorkerNotification>>>,
    
    /// Channel to send notifications (cloned for workers).
    notification_tx: mpsc::Sender<WorkerNotification>,
}

impl WorkerRegistry {
    /// Create a new worker registry.
    pub fn new() -> Self {
        let (notification_tx, notification_rx) = mpsc::channel(1000);
        
        Self {
            workers: RwLock::new(HashMap::new()),
            notification_rx: RwLock::new(Some(notification_rx)),
            notification_tx,
        }
    }

    /// Register a new worker.
    pub fn register(&self, handle: WorkerHandle) {
        let mut workers = self.workers.write();
        workers.insert(handle.id.clone(), handle);
    }

    /// Unregister a worker.
    pub fn unregister(&self, worker_id: &str) -> Option<WorkerHandle> {
        let mut workers = self.workers.write();
        workers.remove(worker_id)
    }

    /// Get a worker by ID.
    pub fn get(&self, worker_id: &str) -> Option<WorkerHandle> {
        let workers = self.workers.read();
        workers.get(worker_id).cloned()
    }

    /// Update worker status.
    pub fn update_status(&self, worker_id: &str, status: WorkerStatus) {
        let mut workers = self.workers.write();
        if let Some(handle) = workers.get_mut(worker_id) {
            handle.status = status;
        }
    }

    /// Set worker result.
    pub fn set_result(&self, worker_id: &str, result: String) {
        let mut workers = self.workers.write();
        if let Some(handle) = workers.get_mut(worker_id) {
            handle.result = Some(result);
        }
    }

    /// Set worker error.
    pub fn set_error(&self, worker_id: &str, error: String) {
        let mut workers = self.workers.write();
        if let Some(handle) = workers.get_mut(worker_id) {
            handle.error = Some(error);
        }
    }

    /// List all workers.
    pub fn list(&self) -> Vec<WorkerHandle> {
        let workers = self.workers.read();
        workers.values().cloned().collect()
    }

    /// List workers by status.
    pub fn list_by_status(&self, status: WorkerStatus) -> Vec<WorkerHandle> {
        let workers = self.workers.read();
        workers
            .values()
            .filter(|w| w.status == status)
            .cloned()
            .collect()
    }

    /// Get notification sender (for workers to send notifications).
    pub fn notification_sender(&self) -> mpsc::Sender<WorkerNotification> {
        self.notification_tx.clone()
    }

    /// Take the notification receiver (only once).
    pub fn take_notification_receiver(&self) -> Option<mpsc::Receiver<WorkerNotification>> {
        let mut rx = self.notification_rx.write();
        rx.take()
    }

    /// Get worker count.
    pub fn count(&self) -> usize {
        self.workers.read().len()
    }

    /// Get running worker count.
    pub fn running_count(&self) -> usize {
        self.list_by_status(WorkerStatus::Running).len()
    }
}

impl Default for WorkerRegistry {
    fn default() -> Self {
        Self::new()
    }
}