//! Input schema for AgentTool.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// AgentTool input parameters.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentInput {
    /// Name of the agent to spawn.
    pub agent_name: String,
    
    /// Task prompt for the agent.
    pub prompt: String,
    
    /// Tools available to the agent (optional, defaults to agent definition).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    
    /// Model to use (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    
    /// Tools to disallow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disallowed_tools: Option<Vec<String>>,
    
    /// Working directory for the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workdir: Option<String>,
    
    /// Maximum turns for the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    
    /// Timeout in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    
    /// Run in background (don't wait for completion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_in_background: Option<bool>,
}

/// AgentTool output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    /// Unique agent/task ID.
    pub agent_id: String,
    
    /// Current status.
    pub status: AgentStatus,
    
    /// Final result (if completed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    
    /// Error message (if failed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Agent status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Running,
    Completed,
    Failed,
    Timeout,
    Cancelled,
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::Running => write!(f, "running"),
            AgentStatus::Completed => write!(f, "completed"),
            AgentStatus::Failed => write!(f, "failed"),
            AgentStatus::Timeout => write!(f, "timeout"),
            AgentStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Agent definition for built-in and custom agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Agent name.
    pub name: String,
    
    /// Agent description.
    pub description: String,
    
    /// System prompt for the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    
    /// Default tools for the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    
    /// Default model for the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    
    /// Tools always disallowed for this agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disallowed_tools: Option<Vec<String>>,
}

/// JSON schema for AgentTool input.
pub static AGENT_SCHEMA: Value = json!({
    "type": "object",
    "properties": {
        "agent_name": {
            "type": "string",
            "description": "Name of the agent to spawn (e.g., 'researcher', 'coder', 'reviewer')"
        },
        "prompt": {
            "type": "string",
            "description": "Task prompt for the agent to execute"
        },
        "tools": {
            "type": "array",
            "items": { "type": "string" },
            "description": "Tools available to the agent (defaults to agent definition)"
        },
        "model": {
            "type": "string",
            "description": "Model to use (defaults to agent definition or main model)"
        },
        "disallowed_tools": {
            "type": "array",
            "items": { "type": "string" },
            "description": "Tools that should not be available to this agent"
        },
        "workdir": {
            "type": "string",
            "description": "Working directory for the agent"
        },
        "max_turns": {
            "type": "number",
            "description": "Maximum number of turns for the agent",
            "minimum": 1,
            "default": 50
        },
        "timeout": {
            "type": "number",
            "description": "Timeout in milliseconds",
            "default": 300000
        },
        "run_in_background": {
            "type": "boolean",
            "description": "Run agent in background without waiting for completion",
            "default": false
        }
    },
    "required": ["agent_name", "prompt"]
});