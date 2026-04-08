//! Tests for Glob tool.

use hcode_tools::{GlobTool, Tool, ToolContext};
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_glob_simple_pattern() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create test files
    fs::write(temp_dir.path().join("test1.js"), "content1").await.unwrap();
    fs::write(temp_dir.path().join("test2.js"), "content2").await.unwrap();
    fs::write(temp_dir.path().join("test.txt"), "text").await.unwrap();

    let tool = GlobTool;
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "pattern": "*.js"
    });

    let result = tool.call(input, context).await.unwrap();
    let output_str = result.content;
    let output: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    
    assert!(output["count"].as_u64().unwrap() >= 2);
}

#[tokio::test]
async fn test_glob_with_path() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("sub");
    fs::create_dir_all(&subdir).await.unwrap();
    
    fs::write(subdir.join("file.rs"), "rust code").await.unwrap();

    let tool = GlobTool;
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "pattern": "*.rs",
        "path": "sub"
    });

    let result = tool.call(input, context).await.unwrap();
    let output_str = result.content;
    let output: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    
    assert!(output["count"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn test_glob_with_limit() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create many files
    for i in 0..20 {
        fs::write(temp_dir.path().join(format!("file{}.txt", i)), "content").await.unwrap();
    }

    let tool = GlobTool;
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "pattern": "*.txt",
        "limit": 5
    });

    let result = tool.call(input, context).await.unwrap();
    let output_str = result.content;
    let output: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    
    assert!(output["count"].as_u64().unwrap() <= 5);
}