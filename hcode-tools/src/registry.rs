//! Tool registry.

use crate::{Tool, ToolContext, ToolError};
use crate::{BashTool, FileReadTool, FileWriteTool, FileEditTool, GlobTool, GrepTool};
use crate::{AgentTool, TaskOutputTool, TaskStopTool, SendMessageTool};
use crate::{WebFetchTool, WebSearchTool};
use crate::{TodoWriteTool, AskUserQuestionTool};
use crate::LspTool;
use crate::SkillTool;
use hcode_types::{ToolDefinition, ToolResult};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for managing tools.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Create registry with default tools.
    pub fn with_default_tools() -> Self {
        let mut registry = Self::new();
        
        // Core file tools
        registry.register(Arc::new(BashTool));
        registry.register(Arc::new(FileReadTool));
        registry.register(Arc::new(FileWriteTool));
        registry.register(Arc::new(FileEditTool));
        registry.register(Arc::new(GlobTool));
        registry.register(Arc::new(GrepTool));
        
        // Agent tools
        registry.register(Arc::new(AgentTool));
        registry.register(Arc::new(TaskOutputTool));
        registry.register(Arc::new(TaskStopTool));
        registry.register(Arc::new(SendMessageTool));
        
        // Network tools
        registry.register(Arc::new(WebFetchTool));
        registry.register(Arc::new(WebSearchTool));
        
        // Task & communication tools
        registry.register(Arc::new(TodoWriteTool));
        registry.register(Arc::new(AskUserQuestionTool));
        
        // Intelligence tools
        registry.register(Arc::new(LspTool));
        
        // Extension tools
        registry.register(Arc::new(SkillTool::default()));
        
        registry
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Execute a tool by name.
    pub async fn execute(
        &self,
        name: &str,
        input: Value,
        context: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let tool = self
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;
        tool.call(input, context).await
    }

    /// Filter tools by allowed/disallowed lists.
    pub fn filter(&self, allowed: &[String], disallowed: &[String]) -> Self {
        let tools = self
            .tools
            .iter()
            .filter(|(name, _)| {
                !disallowed.contains(name) && (allowed.is_empty() || allowed.contains(name))
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Self { tools }
    }

    /// Convert to LLM tool definitions.
    pub fn to_llm_tools(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|t| ToolDefinition::new(t.name(), "", t.input_schema().clone()))
            .collect()
    }

    /// List all tool names.
    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    /// Get all tools as Arc references.
    pub fn tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.values().cloned().collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}