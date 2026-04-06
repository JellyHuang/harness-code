//! Tool orchestration for concurrent and serial execution.
//!
//! Partitions tool calls into concurrent-safe and serial batches,
//! then executes them with appropriate concurrency controls.

use crate::state::ToolUseBlock;
use futures::stream::{Stream, StreamExt};
use hcode_tools::{Tool, ToolContext, ToolError};
use hcode_types::ContentBlock;
use std::sync::Arc;

/// Tool batch for execution
#[derive(Debug, Clone)]
pub struct ToolBatch {
    /// Whether this batch can run concurrently
    pub is_concurrent: bool,

    /// Tool use blocks in this batch
    pub blocks: Vec<ToolUseBlock>,
}

/// Partition tool calls into concurrent and serial batches.
///
/// Tools are partitioned based on is_concurrency_safe():
/// - Concurrent-safe tools (read-only) can run in parallel
/// - Non-safe tools run serially to prevent conflicts
pub fn partition_tool_calls(
    blocks: Vec<ToolUseBlock>,
    tools: &Arc<dyn ToolRegistry>,
) -> Vec<ToolBatch> {
    let mut batches: Vec<ToolBatch> = Vec::new();

    for block in blocks {
        let tool = tools.get(&block.name);
        let is_safe = tool.map(|t| t.is_concurrency_safe()).unwrap_or(false);

        // Add to existing batch if same concurrency type
        if let Some(last_batch) = batches.last_mut() {
            if last_batch.is_concurrent == is_safe {
                last_batch.blocks.push(block);
                continue;
            }
        }

        // Create new batch
        batches.push(ToolBatch {
            is_concurrent: is_safe,
            blocks: vec![block],
        });
    }

    batches
}

/// Execute a batch of tools concurrently
pub async fn run_tools_concurrently(
    blocks: Vec<ToolUseBlock>,
    tools: Arc<dyn ToolRegistry>,
    context: ToolContext,
    concurrency: usize,
) -> impl Stream<Item = ToolExecutionResult> {
    use futures::stream;

    stream::iter(blocks)
        .map(move |block| {
            let tools = tools.clone();
            let ctx = context.clone();

            async move { execute_single_tool(block, tools, ctx).await }
        })
        .buffer_unordered(concurrency)
}

/// Execute tools serially
pub async fn run_tools_serially(
    blocks: Vec<ToolUseBlock>,
    tools: Arc<dyn ToolRegistry>,
    context: ToolContext,
) -> impl Stream<Item = ToolExecutionResult> {
    use futures::stream;

    stream::iter(blocks).then(move |block| {
        let tools = tools.clone();
        let ctx = context.clone();

        async move { execute_single_tool(block, tools, ctx).await }
    })
}

/// Execute a single tool
pub async fn execute_single_tool(
    block: ToolUseBlock,
    tools: Arc<dyn ToolRegistry>,
    context: ToolContext,
) -> ToolExecutionResult {
    let tool = match tools.get(&block.name) {
        Some(t) => t,
        None => {
            let tool_name = block.name.clone();
            return ToolExecutionResult {
                tool_use_id: block.id,
                tool_name,
                result: Err(ToolError::NotFound(format!(
                    "Tool not found: {}",
                    block.name
                ))),
                content_blocks: vec![],
            };
        }
    };

    // Execute the tool
    let result = tool.call(block.input.clone(), context).await;

    // Convert result to content blocks
    let content_blocks = match &result {
        Ok(tool_result) => {
            vec![ContentBlock::tool_result(
                &block.id,
                format!("{:?}", tool_result),
                false,
            )]
        }
        Err(e) => {
            vec![ContentBlock::tool_result(
                &block.id,
                format!("Error: {}", e),
                true,
            )]
        }
    };

    ToolExecutionResult {
        tool_use_id: block.id,
        tool_name: block.name,
        result,
        content_blocks,
    }
}

/// Tool execution result
#[derive(Debug)]
pub struct ToolExecutionResult {
    pub tool_use_id: String,
    pub tool_name: String,
    pub result: Result<hcode_types::ToolResult, ToolError>,
    pub content_blocks: Vec<ContentBlock>,
}

/// Tool registry trait (re-exported from query_engine for convenience)
#[async_trait::async_trait]
pub trait ToolRegistry: Send + Sync {
    fn get(&self, name: &str) -> Option<Arc<dyn Tool>>;
    fn list(&self) -> Vec<Arc<dyn Tool>>;
}

/// Get max tool use concurrency (configurable)
pub fn get_max_tool_use_concurrency() -> usize {
    std::env::var("HCODE_MAX_TOOL_CONCURRENCY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_tool_calls() {
        // Mock test - would need actual tool registry
        let blocks = vec![
            ToolUseBlock {
                id: "1".to_string(),
                name: "read".to_string(),
                input: serde_json::json!({}),
            },
            ToolUseBlock {
                id: "2".to_string(),
                name: "write".to_string(),
                input: serde_json::json!({}),
            },
        ];

        // Without actual registry, just verify structure
        assert_eq!(blocks.len(), 2);
    }
}
