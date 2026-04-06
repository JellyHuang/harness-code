//! Engine event types.

use hcode_protocol::StreamEvent;
use hcode_types::{TaskNotification, ToolResult, Usage};

/// Events from the engine.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    /// Stream event from provider.
    StreamEvent(StreamEvent),
    /// Tool started.
    ToolStart {
        name: String,
        input: serde_json::Value,
    },
    /// Tool finished.
    ToolResult { name: String, result: ToolResult },
    /// Worker spawned.
    WorkerSpawned { id: String, agent_type: String },
    /// Worker notification.
    WorkerNotification(TaskNotification),
    /// Execution complete.
    Complete { response: String, usage: Usage },
    /// Error occurred.
    Error { message: String },
}
