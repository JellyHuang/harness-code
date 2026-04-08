//! Worker module for sub-agent execution.

mod state;
mod communication;
mod execution;

pub use state::*;
pub use communication::*;
pub use execution::*;

use crate::coordinator::{WorkerHandle, WorkerMessage, WorkerRegistry, WorkerStatus};
use hcode_tools::ToolRegistry;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Worker configuration.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// Maximum turns.
    pub max_turns: u32,
    
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
    
    /// Enable progress notifications.
    pub enable_notifications: bool,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            max_turns: 50,
            timeout_ms: 300_000,
            enable_notifications: true,
        }
    }
}

/// Worker error.
#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Timeout")]
    Timeout,
    
    #[error("Cancelled")]
    Cancelled,
    
    #[error("Channel error: {0}")]
    ChannelError(String),
}

/// Worker for executing sub-agent tasks.
pub struct Worker {
    /// Worker ID.
    id: String,
    
    /// Worker name.
    name: String,
    
    /// Executor.
    executor: WorkerExecutor,
    
    /// State machine.
    state: WorkerStateMachine,
    
    /// Communication channels.
    comm: Option<WorkerCommunication>,
    
    /// Configuration.
    #[allow(dead_code)]
    config: WorkerConfig,
}

impl Worker {
    /// Create a new worker.
    pub fn new(
        name: String,
        prompt: String,
        tools: Option<Vec<String>>,
        model: Option<String>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        let config = WorkerConfig::default();
        
        let executor = WorkerExecutor::new(
            id.clone(),
            name.clone(),
            prompt,
            tools,
            model,
            tool_registry,
        )
        .with_max_turns(config.max_turns)
        .with_timeout(config.timeout_ms);
        
        let state = WorkerStateMachine::new(config.max_turns);
        
        Self {
            id,
            name,
            executor,
            state,
            comm: None,
            config,
        }
    }

    /// Get worker ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get worker name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set up communication channels and register with coordinator.
    pub fn setup_communication(&mut self, registry: &WorkerRegistry) -> mpsc::Sender<WorkerMessage> {
        let (msg_tx, msg_rx) = mpsc::channel(100);
        let notification_tx = registry.notification_sender();
        
        self.comm = Some(WorkerCommunication::new(
            self.id.clone(),
            notification_tx,
            msg_rx,
        ));
        
        // Register with registry
        let handle = WorkerHandle {
            id: self.id.clone(),
            name: self.name.clone(),
            status: WorkerStatus::Running,
            sender: msg_tx.clone(),
            created_at: chrono::Utc::now(),
            prompt: self.executor.prompt.clone(),
            result: None,
            error: None,
        };
        
        registry.register(handle);
        
        msg_tx
    }

    /// Run the worker.
    pub async fn run(mut self) -> WorkerResult {
        let comm = self.comm.take()
            .expect("Worker communication not set up");
        
        // Run executor
        let result = self.executor.run(self.state, comm).await;
        
        result
    }

    /// Get current state.
    pub fn state(&self) -> &WorkerStateMachine {
        &self.state
    }
}