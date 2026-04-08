//! Agent spawning and execution.

use super::built_in::get_builtin_agent;
use super::schema::{AgentDefinition, AgentInput, AgentOutput, AgentStatus};
use crate::{ToolContext, ToolError};
use hcode_types::ToolResult;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Default timeout for agent execution (5 minutes).
pub const DEFAULT_TIMEOUT_MS: u64 = 300_000;

/// Default max turns for agent execution.
pub const DEFAULT_MAX_TURNS: u32 = 50;

/// Agent spawner configuration.
#[derive(Debug, Clone)]
pub struct SpawnerConfig {
    /// Maximum concurrent agents.
    pub max_concurrent: usize,
    /// Default timeout in milliseconds.
    pub default_timeout_ms: u64,
    /// Default max turns.
    pub default_max_turns: u32,
}

impl Default for SpawnerConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            default_timeout_ms: DEFAULT_TIMEOUT_MS,
            default_max_turns: DEFAULT_MAX_TURNS,
        }
    }
}

/// Result of agent spawning.
#[derive(Debug)]
pub struct SpawnResult {
    pub agent_id: String,
    pub status_tx: mpsc::Sender<AgentStatus>,
}

/// Spawn an agent (placeholder - will integrate with Coordinator).
pub async fn spawn_agent(
    input: AgentInput,
    _context: ToolContext,
) -> Result<ToolResult, ToolError> {
    // Generate agent ID
    let agent_id = Uuid::new_v4().to_string();

    // Get agent definition
    let definition = if get_builtin_agent(&input.agent_name).is_some() {
        get_builtin_agent(&input.agent_name).unwrap()
    } else {
        // Create custom agent definition from input
        AgentDefinition {
            name: input.agent_name.clone(),
            description: format!("Custom agent: {}", input.agent_name),
            system_prompt: None,
            tools: input.tools.clone(),
            model: input.model.clone(),
            disallowed_tools: input.disallowed_tools.clone(),
        }
    };

    // Determine run mode
    let run_in_background = input.run_in_background.unwrap_or(false);
    let _timeout_ms = input.timeout.unwrap_or(DEFAULT_TIMEOUT_MS);
    let _max_turns = input.max_turns.unwrap_or(DEFAULT_MAX_TURNS);

    if run_in_background {
        // Return immediately with agent ID
        Ok(ToolResult::success(
            serde_json::to_value(AgentOutput {
                agent_id: agent_id.clone(),
                status: AgentStatus::Running,
                result: None,
                error: None,
            })
            .unwrap(),
        ))
    } else {
        // For now, return a placeholder result
        // In full implementation, this would:
        // 1. Create a Worker
        // 2. Register with Coordinator
        // 3. Execute the agent
        // 4. Wait for completion or timeout
        
        // Placeholder: simulate agent execution
        let result = format!(
            "Agent '{}' spawned with ID: {}\nDefinition: {:?}\nPrompt: {}",
            input.agent_name, agent_id, definition, input.prompt
        );

        Ok(ToolResult::success(
            serde_json::to_value(AgentOutput {
                agent_id,
                status: AgentStatus::Completed,
                result: Some(result),
                error: None,
            })
            .unwrap(),
        ))
    }
}

/// Get effective tools for an agent (definition tools + input overrides).
pub fn get_effective_tools(definition: &AgentDefinition, input: &AgentInput) -> Vec<String> {
    let mut tools = input.tools.clone().unwrap_or_else(|| {
        definition.tools.clone().unwrap_or_default()
    });

    // Remove disallowed tools
    if let Some(disallowed) = &definition.disallowed_tools {
        tools.retain(|t| !disallowed.contains(t));
    }
    if let Some(disallowed) = &input.disallowed_tools {
        tools.retain(|t| !disallowed.contains(t));
    }

    tools
}

/// Get effective model for an agent.
pub fn get_effective_model(definition: &AgentDefinition, input: &AgentInput) -> Option<String> {
    input.model.clone().or(definition.model.clone())
}