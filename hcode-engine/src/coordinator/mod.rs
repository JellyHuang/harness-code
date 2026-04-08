//! Coordinator module for multi-agent orchestration.

mod worker_registry;
mod notification;
mod message_router;

pub use worker_registry::*;
pub use notification::*;
pub use message_router::*;

use hcode_tools::CoordinatorRef;
use parking_lot::RwLock;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Coordinator configuration.
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    /// Maximum concurrent workers.
    pub max_concurrent_workers: usize,
    
    /// Default timeout for worker completion.
    pub default_timeout_ms: u64,
    
    /// Enable XML notifications.
    pub enable_notifications: bool,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_workers: 10,
            default_timeout_ms: 300_000,
            enable_notifications: true,
        }
    }
}

/// Worker specification for spawning.
#[derive(Debug, Clone)]
pub struct WorkerSpec {
    /// Worker name (agent type).
    pub name: String,
    
    /// Task prompt.
    pub prompt: String,
    
    /// Tools available to the worker.
    pub tools: Option<Vec<String>>,
    
    /// Model to use.
    pub model: Option<String>,
    
    /// Working directory.
    pub workdir: Option<String>,
    
    /// Maximum turns.
    pub max_turns: Option<u32>,
    
    /// Timeout in milliseconds.
    pub timeout_ms: Option<u64>,
}

/// Coordinator error.
#[derive(Debug, thiserror::Error)]
pub enum CoordinatorError {
    #[error("Worker not found: {0}")]
    WorkerNotFound(String),
    
    #[error("Maximum concurrent workers reached")]
    MaxWorkersReached,
    
    #[error("Worker failed: {0}")]
    WorkerFailed(String),
    
    #[error("Timeout waiting for worker")]
    Timeout,
    
    #[error("Channel error: {0}")]
    ChannelError(String),
}

/// Worker info for listing.
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub prompt: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub result: Option<String>,
    pub error: Option<String>,
}

/// Main coordinator for managing workers.
pub struct Coordinator {
    /// Worker registry.
    registry: Arc<WorkerRegistry>,
    
    /// Message router.
    router: MessageRouter,
    
    /// Configuration.
    config: CoordinatorConfig,
    
    /// Worker specs for tracking.
    specs: RwLock<HashMap<String, WorkerSpec>>,
}

impl Coordinator {
    /// Create a new coordinator.
    pub fn new(config: CoordinatorConfig) -> Self {
        let registry = Arc::new(WorkerRegistry::new());
        let router = MessageRouter::new(registry.clone());
        
        Self {
            registry,
            router,
            config,
            specs: RwLock::new(HashMap::new()),
        }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(CoordinatorConfig::default())
    }

    /// Get the worker registry.
    pub fn registry(&self) -> Arc<WorkerRegistry> {
        self.registry.clone()
    }

    /// Get the message router.
    pub fn router(&self) -> &MessageRouter {
        &self.router
    }

    /// Spawn a new worker.
    pub async fn spawn_worker(&self, spec: WorkerSpec) -> Result<String, CoordinatorError> {
        // Check max concurrent workers
        if self.registry.running_count() >= self.config.max_concurrent_workers {
            return Err(CoordinatorError::MaxWorkersReached);
        }

        // Generate worker ID
        let worker_id = Uuid::new_v4().to_string();

        // Create message channel
        let (sender, _receiver) = mpsc::channel(100);

        // Create worker handle
        let handle = WorkerHandle {
            id: worker_id.clone(),
            name: spec.name.clone(),
            status: WorkerStatus::Running,
            sender,
            created_at: chrono::Utc::now(),
            prompt: spec.prompt.clone(),
            result: None,
            error: None,
        };

        // Register worker
        self.registry.register(handle);
        
        // Store spec
        self.specs.write().insert(worker_id.clone(), spec);

        Ok(worker_id)
    }

    /// Stop a worker.
    pub async fn stop_worker(&self, worker_id: &str) -> Result<(), CoordinatorError> {
        self.router.stop_worker(worker_id).await
            .map_err(CoordinatorError::ChannelError)?;
        
        self.registry.update_status(worker_id, WorkerStatus::Cancelled);
        
        Ok(())
    }

    /// Get worker status.
    pub fn get_worker_status(&self, worker_id: &str) -> Option<WorkerStatus> {
        self.registry.get(worker_id).map(|h| h.status)
    }

    /// Get worker result.
    pub fn get_worker_result(&self, worker_id: &str) -> Option<String> {
        self.registry.get(worker_id).and_then(|h| h.result)
    }

    /// List all workers.
    pub fn list_workers(&self) -> Vec<WorkerInfo> {
        self.registry.list()
            .into_iter()
            .map(|h| WorkerInfo {
                id: h.id,
                name: h.name,
                status: h.status.to_string(),
                prompt: h.prompt,
                created_at: h.created_at,
                result: h.result,
                error: h.error,
            })
            .collect()
    }

    /// Wait for worker completion with timeout.
    pub async fn wait_for_completion(&self, worker_id: &str, timeout: Duration) -> Result<String, CoordinatorError> {
        let start = std::time::Instant::now();
        
        loop {
            // Check if worker exists
            let handle = self.registry.get(worker_id)
                .ok_or_else(|| CoordinatorError::WorkerNotFound(worker_id.to_string()))?;
            
            // Check status
            match handle.status {
                WorkerStatus::Completed => {
                    return handle.result.ok_or_else(|| {
                        CoordinatorError::WorkerFailed("No result".to_string())
                    });
                }
                WorkerStatus::Failed => {
                    return Err(CoordinatorError::WorkerFailed(
                        handle.error.unwrap_or_else(|| "Unknown error".to_string())
                    ));
                }
                WorkerStatus::Cancelled => {
                    return Err(CoordinatorError::WorkerFailed("Worker cancelled".to_string()));
                }
                WorkerStatus::Timeout => {
                    return Err(CoordinatorError::Timeout);
                }
                WorkerStatus::Running => {
                    // Continue waiting
                }
            }
            
            // Check timeout
            if start.elapsed() >= timeout {
                self.registry.update_status(worker_id, WorkerStatus::Timeout);
                return Err(CoordinatorError::Timeout);
            }
            
            // Wait a bit before checking again
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Stop all workers.
    pub async fn stop_all(&self) {
        self.router.stop_all().await;
        
        // Update all running workers to cancelled
        let workers = self.registry.list_by_status(WorkerStatus::Running);
        for worker in workers {
            self.registry.update_status(&worker.id, WorkerStatus::Cancelled);
        }
    }

    /// Get worker count.
    pub fn worker_count(&self) -> usize {
        self.registry.count()
    }

    /// Get running worker count.
    pub fn running_count(&self) -> usize {
        self.registry.running_count()
    }
}

impl Default for Coordinator {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Implement CoordinatorRef for Coordinator to allow type-erased references.
impl CoordinatorRef for Coordinator {
    fn as_any(&self) -> &dyn Any {
        self
    }
}