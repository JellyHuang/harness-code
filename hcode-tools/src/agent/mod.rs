//! Agent tool for spawning sub-agents.

mod schema;
mod spawner;
mod built_in;

use crate::{Tool, ToolContext, ToolError};
use async_trait::async_trait;
use hcode_types::ToolResult;
use serde_json::Value;
pub use schema::*;
pub use spawner::*;
pub use built_in::*;

/// Agent tool for spawning sub-agents.
pub struct AgentTool;

#[async_trait]
impl Tool for AgentTool {
    fn name(&self) -> &str {
        "agent"
    }

    fn description(&self) -> &str {
        "Spawn a sub-agent to handle a specific task"
    }

    fn input_schema(&self) -> &Value {
        &AGENT_SCHEMA
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: AgentInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        // Validate agent name
        if params.agent_name.is_empty() {
            return Err(ToolError::InvalidInput("agent_name is required".to_string()));
        }

        // Validate prompt
        if params.prompt.is_empty() {
            return Err(ToolError::InvalidInput("prompt is required".to_string()));
        }

        // Validate timeout
        if let Some(timeout) = params.timeout {
            if timeout > 600_000 {
                return Err(ToolError::InvalidInput(
                    "timeout exceeds maximum of 600000ms".to_string(),
                ));
            }
        }

        spawner::spawn_agent(params, context).await
    }
}