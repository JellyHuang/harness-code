//! Message router for coordinator-worker communication.

use super::worker_registry::{WorkerMessage, WorkerRegistry};
use std::sync::Arc;

/// Message from coordinator to be routed.
#[derive(Debug, Clone)]
pub enum CoordinatorMessage {
    /// Broadcast to all workers.
    Broadcast { message: WorkerMessage },
    
    /// Send to specific worker.
    Direct { worker_id: String, message: WorkerMessage },
    
    /// Stop all workers.
    StopAll,
}

/// Message router for sending messages to workers.
pub struct MessageRouter {
    registry: Arc<WorkerRegistry>,
}

impl MessageRouter {
    /// Create a new message router.
    pub fn new(registry: Arc<WorkerRegistry>) -> Self {
        Self { registry }
    }

    /// Route a message to a specific worker.
    pub async fn route_to_worker(&self, worker_id: &str, message: WorkerMessage) -> Result<(), String> {
        let handle = self.registry.get(worker_id)
            .ok_or_else(|| format!("Worker not found: {}", worker_id))?;
        
        handle.sender.send(message).await
            .map_err(|e| format!("Failed to send message: {}", e))
    }

    /// Broadcast a message to all workers.
    pub async fn broadcast(&self, message: WorkerMessage) {
        let workers = self.registry.list();
        
        for worker in workers {
            let _ = worker.sender.send(message.clone()).await;
        }
    }

    /// Broadcast to all running workers.
    pub async fn broadcast_running(&self, message: WorkerMessage) {
        let workers = self.registry.list_by_status(super::worker_registry::WorkerStatus::Running);
        
        for worker in workers {
            let _ = worker.sender.send(message.clone()).await;
        }
    }

    /// Stop all workers.
    pub async fn stop_all(&self) {
        self.broadcast(WorkerMessage::Stop).await;
    }

    /// Stop a specific worker.
    pub async fn stop_worker(&self, worker_id: &str) -> Result<(), String> {
        self.route_to_worker(worker_id, WorkerMessage::Stop).await
    }
}