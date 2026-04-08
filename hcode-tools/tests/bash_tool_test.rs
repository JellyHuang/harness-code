//! Tests for Bash tool.

use hcode_tools::{BashTool, Tool, ToolContext};
use serde_json::json;
use std::path::PathBuf;

#[tokio::test]
async fn test_bash_simple_command() {
    let tool = BashTool;
    let context = ToolContext::new(
        PathBuf::from("."),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "command": "echo hello"
    });

    let result = tool.call(input, context).await.unwrap();
    let output_str = result.content;
    let output: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    
    assert_eq!(output["exit_code"], 0);
    assert!(output["stdout"].as_str().unwrap().contains("hello"));
}

#[tokio::test]
async fn test_bash_with_timeout() {
    let tool = BashTool;
    let context = ToolContext::new(
        PathBuf::from("."),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "command": "sleep 0.1",
        "timeout": 1000
    });

    let result = tool.call(input, context).await.unwrap();
    let output_str = result.content;
    let output: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    assert!(!output["timed_out"].as_bool().unwrap());
}

#[tokio::test]
async fn test_bash_timeout_exceeded() {
    let tool = BashTool;
    let context = ToolContext::new(
        PathBuf::from("."),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "command": "sleep 10",
        "timeout": 100
    });

    let result = tool.call(input, context).await.unwrap();
    let output_str = result.content;
    let output: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    assert!(output["timed_out"].as_bool().unwrap());
}

#[tokio::test]
async fn test_bash_invalid_timeout() {
    let tool = BashTool;
    let context = ToolContext::new(
        PathBuf::from("."),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "command": "echo test",
        "timeout": 1000000  // Exceeds MAX_TIMEOUT_MS
    });

    let result = tool.call(input, context).await;
    assert!(result.is_err());
}