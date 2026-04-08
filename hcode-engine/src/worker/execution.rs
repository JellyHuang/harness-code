//! Worker execution engine.

use super::communication::WorkerCommunication;
use super::state::WorkerStateMachine;
use hcode_tools::{ToolContext, ToolRegistry};
use std::sync::Arc;
use std::time::Duration;

/// Worker execution result.
#[derive(Debug)]
pub struct WorkerResult {
    /// Final result text.
    pub result: String,
    
    /// Number of turns executed.
    pub turns: u32,
    
    /// Whether the worker completed successfully.
    pub success: bool,
    
    /// Error message (if failed).
    pub error: Option<String>,
}

/// Worker executor for running agent tasks.
pub struct WorkerExecutor {
    /// Worker ID.
    pub worker_id: String,
    
    /// Worker name/agent type.
    pub name: String,
    
    /// Task prompt.
    pub prompt: String,
    
    /// Available tools.
    pub tools: Option<Vec<String>>,
    
    /// Model to use.
    pub model: Option<String>,
    
    /// Working directory.
    pub workdir: Option<String>,
    
    /// Maximum turns.
    pub max_turns: u32,
    
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
    
    /// Tool registry.
    pub tool_registry: Arc<ToolRegistry>,
}

impl WorkerExecutor {
    /// Create a new worker executor.
    pub fn new(
        worker_id: String,
        name: String,
        prompt: String,
        tools: Option<Vec<String>>,
        model: Option<String>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            worker_id,
            name,
            prompt,
            tools,
            model,
            workdir: None,
            max_turns: 50,
            timeout_ms: 300_000,
            tool_registry,
        }
    }

    /// Set working directory.
    pub fn with_workdir(mut self, workdir: String) -> Self {
        self.workdir = Some(workdir);
        self
    }

    /// Set max turns.
    pub fn with_max_turns(mut self, max_turns: u32) -> Self {
        self.max_turns = max_turns;
        self
    }

    /// Set timeout.
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Run the worker (simplified - actual implementation would use QueryEngine).
    pub async fn run(
        mut self,
        mut state: WorkerStateMachine,
        comm: WorkerCommunication,
    ) -> WorkerResult {
        // Notify started
        comm.notify_started().await;
        
        // Start state machine
        state.start();
        
        // Notify progress
        comm.notify_progress(&format!("Starting agent: {}", self.name)).await;
        
        // Simulate execution
        // In a full implementation, this would:
        // 1. Call the LLM provider
        // 2. Process the response
        // 3. Handle tool calls
        // 4. Loop until completion or max turns
        
        // For now, return a placeholder result
        let result = format!(
            "Agent '{}' executed task: {}\n\nCompleted after {} turns.",
            self.name, self.prompt, state.current_turn()
        );
        
        // Complete
        state.complete();
        
        // Notify completion
        comm.notify_completed(&result).await;
        
        WorkerResult {
            result,
            turns: state.current_turn(),
            success: true,
            error: None,
        }
    }
}