//! Worker state machine.

use hcode_types::Message;

/// Worker state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerState {
    /// Worker is idle, waiting to start.
    Idle,
    
    /// Worker is running a turn.
    Running { turn: u32 },
    
    /// Worker is waiting for a tool result.
    WaitingForTool { tool_use_id: String },
    
    /// Worker has completed.
    Completed,
    
    /// Worker has failed.
    Failed { error: String },
    
    /// Worker was cancelled.
    Cancelled,
    
    /// Worker timed out.
    Timeout,
}

/// Worker state machine for managing execution flow.
#[derive(Debug)]
pub struct WorkerStateMachine {
    /// Current state.
    state: WorkerState,
    
    /// Message history.
    messages: Vec<Message>,
    
    /// Tool results pending.
    pending_tool_results: std::collections::HashMap<String, hcode_types::ToolResult>,
    
    /// Current turn.
    current_turn: u32,
    
    /// Maximum turns.
    max_turns: u32,
}

impl WorkerStateMachine {
    /// Create a new state machine.
    pub fn new(max_turns: u32) -> Self {
        Self {
            state: WorkerState::Idle,
            messages: Vec::new(),
            pending_tool_results: std::collections::HashMap::new(),
            current_turn: 0,
            max_turns,
        }
    }

    /// Get current state.
    pub fn state(&self) -> &WorkerState {
        &self.state
    }

    /// Check if worker is done.
    pub fn is_done(&self) -> bool {
        matches!(
            self.state,
            WorkerState::Completed | WorkerState::Failed { .. } | WorkerState::Cancelled | WorkerState::Timeout
        )
    }

    /// Start the worker.
    pub fn start(&mut self) {
        if self.state == WorkerState::Idle {
            self.current_turn = 1;
            self.state = WorkerState::Running { turn: 1 };
        }
    }

    /// Advance to next turn.
    pub fn next_turn(&mut self) -> bool {
        if let WorkerState::Running { turn } = self.state {
            let next_turn = turn + 1;
            
            if next_turn > self.max_turns {
                self.state = WorkerState::Completed;
                return false;
            }
            
            self.current_turn = next_turn;
            self.state = WorkerState::Running { turn: next_turn };
            return true;
        }
        false
    }

    /// Wait for tool result.
    pub fn wait_for_tool(&mut self, tool_use_id: String) {
        if let WorkerState::Running { .. } = self.state {
            self.state = WorkerState::WaitingForTool { tool_use_id };
        }
    }

    /// Receive tool result.
    pub fn receive_tool_result(&mut self, tool_use_id: String, result: hcode_types::ToolResult) {
        if let WorkerState::WaitingForTool { tool_use_id: waiting_id } = &self.state {
            if waiting_id == &tool_use_id {
                self.pending_tool_results.insert(tool_use_id, result);
                self.state = WorkerState::Running { turn: self.current_turn };
            }
        }
    }

    /// Complete the worker.
    pub fn complete(&mut self) {
        self.state = WorkerState::Completed;
    }

    /// Fail the worker.
    pub fn fail(&mut self, error: String) {
        self.state = WorkerState::Failed { error };
    }

    /// Cancel the worker.
    pub fn cancel(&mut self) {
        self.state = WorkerState::Cancelled;
    }

    /// Timeout the worker.
    pub fn timeout(&mut self) {
        self.state = WorkerState::Timeout;
    }

    /// Add a message.
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Get messages.
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Get current turn.
    pub fn current_turn(&self) -> u32 {
        self.current_turn
    }

    /// Get pending tool result.
    pub fn get_tool_result(&mut self, tool_use_id: &str) -> Option<hcode_types::ToolResult> {
        self.pending_tool_results.remove(tool_use_id)
    }
}