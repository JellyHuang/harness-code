//! Tests for AgentTool.

use hcode_tools::{AgentTool, Tool, ToolContext};
use serde_json::json;
use std::path::PathBuf;

#[tokio::test]
async fn test_agent_spawn_builtin() {
    let tool = AgentTool;
    let context = ToolContext::new(
        PathBuf::from("."),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "agent_name": "researcher",
        "prompt": "Find all Rust files in the project"
    });

    let result = tool.call(input, context).await.unwrap();
    let output_str = result.content;
    let output: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    assert_eq!(output["status"], "completed");
    assert!(output["agent_id"].as_str().unwrap().len() > 0);
}

#[tokio::test]
async fn test_agent_spawn_custom() {
    let tool = AgentTool;
    let context = ToolContext::new(
        PathBuf::from("."),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "agent_name": "custom-agent",
        "prompt": "Do something custom",
        "tools": ["read", "write"],
        "model": "claude-sonnet-4-20250514"
    });

    let result = tool.call(input, context).await.unwrap();
    let output_str = result.content;
    let output: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    assert_eq!(output["status"], "completed");
}

#[tokio::test]
async fn test_agent_spawn_background() {
    let tool = AgentTool;
    let context = ToolContext::new(
        PathBuf::from("."),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "agent_name": "coder",
        "prompt": "Implement a new feature",
        "run_in_background": true
    });

    let result = tool.call(input, context).await.unwrap();
    let output_str = result.content;
    let output: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    assert_eq!(output["status"], "running");
}

#[tokio::test]
async fn test_agent_missing_name() {
    let tool = AgentTool;
    let context = ToolContext::new(
        PathBuf::from("."),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "prompt": "Do something"
    });

    let result = tool.call(input, context).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_agent_missing_prompt() {
    let tool = AgentTool;
    let context = ToolContext::new(
        PathBuf::from("."),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "agent_name": "researcher"
    });

    let result = tool.call(input, context).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_agent_invalid_timeout() {
    let tool = AgentTool;
    let context = ToolContext::new(
        PathBuf::from("."),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "agent_name": "researcher",
        "prompt": "Do something",
        "timeout": 1000000
    });

    let result = tool.call(input, context).await;
    assert!(result.is_err());
}