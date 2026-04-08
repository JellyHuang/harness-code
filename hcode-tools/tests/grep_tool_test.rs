//! Tests for Grep tool.

use hcode_tools::{GrepTool, Tool, ToolContext};
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_grep_simple_pattern() {
    let temp_dir = TempDir::new().unwrap();
    
    fs::write(temp_dir.path().join("test1.txt"), "hello world\nfoo bar\n").await.unwrap();
    fs::write(temp_dir.path().join("test2.txt"), "no match here\n").await.unwrap();

    let tool = GrepTool;
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "pattern": "hello"
    });

    let result = tool.call(input, context).await.unwrap();
    let output_str = result.content;
    let output: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    
    assert!(output["count"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn test_grep_regex_pattern() {
    let temp_dir = TempDir::new().unwrap();
    
    fs::write(temp_dir.path().join("code.rs"), "fn test() {}\nfn main() {}\n").await.unwrap();

    let tool = GrepTool;
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "pattern": "fn \\w+\\("
    });

    let result = tool.call(input, context).await.unwrap();
    let output_str = result.content;
    let output: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    
    assert!(output["count"].as_u64().unwrap() >= 2);
}

#[tokio::test]
async fn test_grep_with_limit() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create file with many matches
    let content = "match\n".repeat(20);
    fs::write(temp_dir.path().join("big.txt"), &content).await.unwrap();

    let tool = GrepTool;
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "pattern": "match",
        "limit": 5
    });

    let result = tool.call(input, context).await.unwrap();
    let output_str = result.content;
    let output: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    
    assert!(output["count"].as_u64().unwrap() <= 5);
}

#[tokio::test]
async fn test_grep_invalid_regex() {
    let tool = GrepTool;
    let context = ToolContext::new(
        PathBuf::from("."),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "pattern": "[invalid("
    });

    let result = tool.call(input, context).await;
    assert!(result.is_err());
}