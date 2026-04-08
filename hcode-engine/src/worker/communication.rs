//! Worker communication with coordinator.

use crate::coordinator::{NotificationEvent, WorkerMessage, WorkerNotification};
use chrono::Utc;
use tokio::sync::mpsc;

/// Worker communication channels.
pub struct WorkerCommunication {
    /// Channel to send notifications to coordinator.
    notification_tx: mpsc::Sender<WorkerNotification>,
    
    /// Channel to receive messages from coordinator.
    message_rx: mpsc::Receiver<WorkerMessage>,
    
    /// Worker ID.
    worker_id: String,
}

impl WorkerCommunication {
    /// Create new communication channels.
    pub fn new(
        worker_id: String,
        notification_tx: mpsc::Sender<WorkerNotification>,
        message_rx: mpsc::Receiver<WorkerMessage>,
    ) -> Self {
        Self {
            notification_tx,
            message_rx,
            worker_id,
        }
    }

    /// Send a notification to the coordinator.
    pub async fn send_notification(&self, event: NotificationEvent) {
        let notification = WorkerNotification {
            worker_id: self.worker_id.clone(),
            event,
            timestamp: Utc::now(),
        };
        
        let _ = self.notification_tx.send(notification).await;
    }

    /// Receive a message from the coordinator (non-blocking).
    pub fn try_receive_message(&mut self) -> Option<WorkerMessage> {
        self.message_rx.try_recv().ok()
    }

    /// Receive a message from the coordinator (blocking).
    pub async fn receive_message(&mut self) -> Option<WorkerMessage> {
        self.message_rx.recv().await
    }

    /// Send started notification.
    pub async fn notify_started(&self) {
        self.send_notification(NotificationEvent::Started).await;
    }

    /// Send progress notification.
    pub async fn notify_progress(&self, message: &str) {
        self.send_notification(NotificationEvent::Progress {
            message: message.to_string(),
        }).await;
    }

    /// Send tool use notification.
    pub async fn notify_tool_use(&self, tool: &str, input: serde_json::Value) {
        self.send_notification(NotificationEvent::ToolUse {
            tool: tool.to_string(),
            input,
        }).await;
    }

    /// Send tool result notification.
    pub async fn notify_tool_result(&self, tool: &str, result: &str) {
        self.send_notification(NotificationEvent::ToolResult {
            tool: tool.to_string(),
            result: result.to_string(),
        }).await;
    }

    /// Send completed notification.
    pub async fn notify_completed(&self, result: &str) {
        self.send_notification(NotificationEvent::Completed {
            result: result.to_string(),
        }).await;
    }

    /// Send failed notification.
    pub async fn notify_failed(&self, error: &str) {
        self.send_notification(NotificationEvent::Failed {
            error: error.to_string(),
        }).await;
    }
}