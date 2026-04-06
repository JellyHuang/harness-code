//! Task notification types for agent coordination.
//!
//! These types follow the cc-haha-main XML notification format.

use serde::{Deserialize, Serialize};

/// Notification sent by a worker agent to the coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNotification {
    /// Unique identifier for the task.
    #[serde(rename = "task-id")]
    pub task_id: String,
    /// Current status of the task.
    pub status: TaskStatus,
    /// Human-readable summary of the task result.
    pub summary: String,
    /// The result content (if completed successfully).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// Token usage information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    /// Duration in milliseconds.
    #[serde(rename = "duration-ms", skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl TaskNotification {
    /// Create a new task notification.
    pub fn new(task_id: impl Into<String>, status: TaskStatus, summary: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            status,
            summary: summary.into(),
            result: None,
            usage: None,
            duration_ms: None,
        }
    }

    /// Add a result to the notification.
    pub fn with_result(mut self, result: impl Into<String>) -> Self {
        self.result = Some(result.into());
        self
    }

    /// Add usage information to the notification.
    pub fn with_usage(mut self, usage: Usage) -> Self {
        self.usage = Some(usage);
        self
    }

    /// Add duration to the notification.
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }
}

/// Status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task completed successfully.
    Completed,
    /// Task failed with an error.
    Failed,
    /// Task was killed by the user or coordinator.
    Killed,
    /// Task is currently in progress.
    InProgress,
}

impl TaskStatus {
    /// Check if this is a terminal status (no more updates expected).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Killed)
    }
}

/// Token usage information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    /// Total tokens used (input + output).
    #[serde(rename = "total-tokens", skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
    /// Number of tool uses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_uses: Option<u32>,
    /// Input tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u32>,
    /// Output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u32>,
}

impl Usage {
    /// Create a new usage with total tokens.
    pub fn new(total_tokens: u32) -> Self {
        Self {
            total_tokens: Some(total_tokens),
            ..Default::default()
        }
    }
}
