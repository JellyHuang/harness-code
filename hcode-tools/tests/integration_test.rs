//! End-to-end integration test for tools.

use hcode_tools::{ToolRegistry, ToolContext};
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_full_workflow() {
    let registry = ToolRegistry::with_default_tools();
    let temp_dir = TempDir::new().unwrap();
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test-session",
        "test-id"
    );

    // Write a file
    let write_result = registry.execute(
        "write",
        json!({
            "file_path": "test.txt",
            "content": "Hello, World!\nThis is line 2.\nGoodbye, World!"
        }),
        context.clone(),
    ).await.unwrap();
    
    let write_output: serde_json::Value = serde_json::from_str(&write_result.content).unwrap();
    assert!(write_output["bytes_written"].as_u64().unwrap() > 0);

    // Read the file
    let read_result = registry.execute(
        "read",
        json!({
            "file_path": "test.txt"
        }),
        context.clone(),
    ).await.unwrap();
    
    let read_output: serde_json::Value = serde_json::from_str(&read_result.content).unwrap();
    assert_eq!(read_output["total_lines"], 3);

    // Edit the file
    let edit_result = registry.execute(
        "edit",
        json!({
            "file_path": "test.txt",
            "old_string": "Hello",
            "new_string": "Hi"
        }),
        context.clone(),
    ).await.unwrap();
    
    let edit_output: serde_json::Value = serde_json::from_str(&edit_result.content).unwrap();
    assert_eq!(edit_output["replacements"], 1);

    // Grep for pattern
    let grep_result = registry.execute(
        "grep",
        json!({
            "pattern": "line",
            "path": temp_dir.path().to_str().unwrap()
        }),
        context.clone(),
    ).await.unwrap();
    
    let grep_output: serde_json::Value = serde_json::from_str(&grep_result.content).unwrap();
    assert!(grep_output["count"].as_u64().unwrap() > 0);

    // Glob for files
    let glob_result = registry.execute(
        "glob",
        json!({
            "pattern": "*.txt",
            "path": temp_dir.path().to_str().unwrap()
        }),
        context.clone(),
    ).await.unwrap();
    
    let glob_output: serde_json::Value = serde_json::from_str(&glob_result.content).unwrap();
    assert!(glob_output["count"].as_u64().unwrap() >= 1);
}

#[test]
fn test_registry_default_tools() {
    let registry = ToolRegistry::with_default_tools();
    
    assert!(registry.get("bash").is_some());
    assert!(registry.get("read").is_some());
    assert!(registry.get("write").is_some());
    assert!(registry.get("edit").is_some());
    assert!(registry.get("glob").is_some());
    assert!(registry.get("grep").is_some());
    
    assert_eq!(registry.list().len(), 6);
}

#[tokio::test]
async fn test_registry_filter() {
    let registry = ToolRegistry::with_default_tools();
    
    // Allow only read tools
    let filtered = registry.filter(
        &["read".to_string(), "glob".to_string(), "grep".to_string()],
        &[]
    );
    
    assert_eq!(filtered.list().len(), 3);
    assert!(filtered.get("read").is_some());
    assert!(filtered.get("write").is_none());
    
    // Disallow write tools
    let filtered2 = registry.filter(&[], &["write".to_string(), "edit".to_string()]);
    
    assert_eq!(filtered2.list().len(), 4);
    assert!(filtered2.get("write").is_none());
}