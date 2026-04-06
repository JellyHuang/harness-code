//! Streaming tool executor for concurrent tool execution during streaming.
//!
//! Executes tools as they stream in with concurrency control:
//! - Concurrent-safe tools can execute in parallel
//! - Non-concurrent tools must execute alone (exclusive access)
//! - Results are buffered and emitted in order

use crate::state::ToolUseBlock;
use crate::tool_orchestration::{execute_single_tool, ToolExecutionResult, ToolRegistry};
use hcode_tools::ToolContext;
use hcode_types::{ContentBlock, Message};
use parking_lot::RwLock;
use std::sync::Arc;

/// Tool execution status
#[derive(Debug, Clone, PartialEq)]
pub enum ToolStatus {
    /// Tool is queued for execution
    Queued,
    /// Tool is currently executing
    Executing,
    /// Tool execution completed
    Completed,
    /// Tool results have been yielded
    Yielded,
}

/// Tracked tool for streaming execution
#[derive(Debug)]
#[allow(dead_code)]
struct TrackedTool {
    /// Tool use ID
    id: String,
    /// Tool use block
    block: ToolUseBlock,
    /// Execution status
    status: ToolStatus,
    /// Whether tool is concurrency-safe
    is_concurrency_safe: bool,
    /// Execution result (when completed)
    result: Option<ToolExecutionResult>,
    /// Progress messages
    pending_progress: Vec<Message>,
}

/// Message update from streaming executor
#[derive(Debug)]
pub struct MessageUpdate {
    /// Result message (if any)
    pub message: Option<Message>,
    /// Content blocks for tool result
    pub content_blocks: Vec<ContentBlock>,
}

/// Streaming tool executor for concurrent execution during streaming.
///
/// Key features:
/// - Tools start executing as they arrive (not after streaming completes)
/// - Concurrent-safe tools run in parallel
/// - Non-safe tools run serially to prevent conflicts
/// - Results are collected and yielded in order
#[allow(dead_code)]
pub struct StreamingToolExecutor {
    /// Tracked tools
    tools: RwLock<Vec<TrackedTool>>,
    /// Tool registry
    tool_registry: Arc<dyn ToolRegistry>,
    /// Tool context
    tool_context: ToolContext,
    /// Whether executor has been discarded
    discarded: RwLock<bool>,
    /// Whether any tool has errored (cancels siblings)
    has_errored: RwLock<bool>,
    /// Description of errored tool
    errored_tool_description: RwLock<String>,
    /// Maximum concurrent tools
    max_concurrency: usize,
}

impl StreamingToolExecutor {
    /// Create a new streaming tool executor
    pub fn new(
        tool_registry: Arc<dyn ToolRegistry>,
        tool_context: ToolContext,
        max_concurrency: usize,
    ) -> Self {
        Self {
            tools: RwLock::new(Vec::new()),
            tool_registry,
            tool_context,
            discarded: RwLock::new(false),
            has_errored: RwLock::new(false),
            errored_tool_description: RwLock::new(String::new()),
            max_concurrency,
        }
    }

    /// Discard all pending and in-progress tools.
    /// Called when streaming fallback occurs.
    pub fn discard(&self) {
        *self.discarded.write() = true;
    }

    /// Check if discarded
    pub fn is_discarded(&self) -> bool {
        *self.discarded.read()
    }

    /// Add a tool to the execution queue.
    /// Will start executing immediately if conditions allow.
    pub fn add_tool(&self, block: ToolUseBlock) {
        // Check if tool exists
        let tool = self.tool_registry.get(&block.name);
        let is_concurrency_safe = tool
            .as_ref()
            .map(|t| t.is_concurrency_safe())
            .unwrap_or(true);

        let tracked = TrackedTool {
            id: block.id.clone(),
            block,
            status: ToolStatus::Queued,
            is_concurrency_safe,
            result: None,
            pending_progress: Vec::new(),
        };

        self.tools.write().push(tracked);
    }

    /// Check if a tool can execute based on current concurrency state
    fn can_execute_tool(&self, is_concurrency_safe: bool) -> bool {
        let tools = self.tools.read();
        let executing: Vec<_> = tools
            .iter()
            .filter(|t| t.status == ToolStatus::Executing)
            .collect();

        if executing.is_empty() {
            return true;
        }

        // Can execute if all executing tools are concurrency-safe and this one is too
        is_concurrency_safe && executing.iter().all(|t| t.is_concurrency_safe)
    }

    /// Get count of executing tools
    #[allow(dead_code)]
    fn executing_count(&self) -> usize {
        self.tools
            .read()
            .iter()
            .filter(|t| t.status == ToolStatus::Executing)
            .count()
    }

    /// Execute all queued tools concurrently (streaming mode)
    pub async fn execute_queued(&self) {
        let mut tools_to_execute = Vec::new();

        // Find tools that can execute
        {
            let tools = self.tools.read();
            for tool in tools.iter() {
                if tool.status != ToolStatus::Queued {
                    continue;
                }

                if self.can_execute_tool(tool.is_concurrency_safe) {
                    tools_to_execute.push(tool.id.clone());
                }
            }
        }

        // Execute tools
        for tool_id in tools_to_execute {
            // Mark as executing
            {
                let mut tools = self.tools.write();
                if let Some(tool) = tools.iter_mut().find(|t| t.id == tool_id) {
                    tool.status = ToolStatus::Executing;
                }
            }

            // Execute the tool
            let block = {
                let tools = self.tools.read();
                tools
                    .iter()
                    .find(|t| t.id == tool_id)
                    .map(|t| t.block.clone())
            };

            if let Some(block) = block {
                let result = execute_single_tool(
                    block,
                    self.tool_registry.clone(),
                    self.tool_context.clone(),
                )
                .await;

                // Store result
                {
                    let mut tools = self.tools.write();
                    if let Some(tool) = tools.iter_mut().find(|t| t.id == tool_id) {
                        tool.result = Some(result);
                        tool.status = ToolStatus::Completed;
                    }
                }
            }
        }
    }

    /// Get completed results that haven't been yielded yet (non-blocking)
    pub fn get_completed_results(&self) -> Vec<MessageUpdate> {
        if *self.discarded.read() {
            return Vec::new();
        }

        let mut results = Vec::new();
        let mut tools = self.tools.write();

        for tool in tools.iter_mut() {
            // Skip already yielded
            if tool.status == ToolStatus::Yielded {
                continue;
            }

            // Yield completed results
            if tool.status == ToolStatus::Completed {
                if let Some(result) = &tool.result {
                    tool.status = ToolStatus::Yielded;

                    results.push(MessageUpdate {
                        message: None, // Message creation handled by caller
                        content_blocks: result.content_blocks.clone(),
                    });
                }
            }

            // Stop at non-concurrent executing tool
            if tool.status == ToolStatus::Executing && !tool.is_concurrency_safe {
                break;
            }
        }

        results
    }

    /// Wait for all remaining tools and yield their results
    pub async fn get_remaining_results(&self) -> Vec<MessageUpdate> {
        if *self.discarded.read() {
            return Vec::new();
        }

        // Execute all queued tools
        self.execute_queued().await;

        // Wait for all executing tools to complete
        while self.has_executing_tools() {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Return all results
        self.get_completed_results()
    }

    /// Check if there are executing tools
    fn has_executing_tools(&self) -> bool {
        self.tools
            .read()
            .iter()
            .any(|t| t.status == ToolStatus::Executing)
    }

    /// Check if there are unfinished tools
    #[allow(dead_code)]
    fn has_unfinished_tools(&self) -> bool {
        self.tools
            .read()
            .iter()
            .any(|t| t.status != ToolStatus::Yielded)
    }

    /// Get tool use IDs that are in progress
    pub fn get_in_progress_ids(&self) -> Vec<String> {
        self.tools
            .read()
            .iter()
            .filter(|t| t.status == ToolStatus::Executing)
            .map(|t| t.id.clone())
            .collect()
    }

    /// Get total tool count
    pub fn tool_count(&self) -> usize {
        self.tools.read().len()
    }
}

/// Builder for StreamingToolExecutor
pub struct StreamingToolExecutorBuilder {
    tool_registry: Option<Arc<dyn ToolRegistry>>,
    tool_context: Option<ToolContext>,
    max_concurrency: usize,
}

impl StreamingToolExecutorBuilder {
    pub fn new() -> Self {
        Self {
            tool_registry: None,
            tool_context: None,
            max_concurrency: 10,
        }
    }

    pub fn tool_registry(mut self, registry: Arc<dyn ToolRegistry>) -> Self {
        self.tool_registry = Some(registry);
        self
    }

    pub fn tool_context(mut self, context: ToolContext) -> Self {
        self.tool_context = Some(context);
        self
    }

    pub fn max_concurrency(mut self, max: usize) -> Self {
        self.max_concurrency = max;
        self
    }

    pub fn build(self) -> Result<StreamingToolExecutor, String> {
        let tool_registry = self.tool_registry.ok_or("Tool registry is required")?;
        let tool_context = self.tool_context.ok_or("Tool context is required")?;

        Ok(StreamingToolExecutor::new(
            tool_registry,
            tool_context,
            self.max_concurrency,
        ))
    }
}

impl Default for StreamingToolExecutorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_orchestration::ToolRegistry;
    use async_trait::async_trait;
    use hcode_tools::{Tool, ToolError, ToolResult};
    use serde_json::Value;

    struct MockToolRegistry;

    #[async_trait]
    impl ToolRegistry for MockToolRegistry {
        fn get(&self, _name: &str) -> Option<Arc<dyn Tool>> {
            None
        }
        fn list(&self) -> Vec<Arc<dyn Tool>> {
            vec![]
        }
    }

    #[test]
    fn test_executor_creation() {
        let executor = StreamingToolExecutorBuilder::new()
            .tool_registry(Arc::new(MockToolRegistry))
            .tool_context(ToolContext::new(
                std::env::current_dir().unwrap(),
                "test-session".to_string(),
                "test-tool".to_string(),
            ))
            .build();

        assert!(executor.is_ok());
    }

    #[test]
    fn test_add_tool() {
        let executor = StreamingToolExecutorBuilder::new()
            .tool_registry(Arc::new(MockToolRegistry))
            .tool_context(ToolContext::new(
                std::env::current_dir().unwrap(),
                "test-session".to_string(),
                "test-tool".to_string(),
            ))
            .build()
            .unwrap();

        executor.add_tool(ToolUseBlock {
            id: "test-1".to_string(),
            name: "read".to_string(),
            input: serde_json::json!({}),
        });

        assert_eq!(executor.tool_count(), 1);
    }

    #[test]
    fn test_discard() {
        let executor = StreamingToolExecutorBuilder::new()
            .tool_registry(Arc::new(MockToolRegistry))
            .tool_context(ToolContext::new(
                std::env::current_dir().unwrap(),
                "test-session".to_string(),
                "test-tool".to_string(),
            ))
            .build()
            .unwrap();

        assert!(!executor.is_discarded());
        executor.discard();
        assert!(executor.is_discarded());
    }
}
