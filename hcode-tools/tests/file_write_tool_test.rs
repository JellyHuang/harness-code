//! Tests for FileWrite tool.

use hcode_tools::{FileWriteTool, Tool, ToolContext};
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_write_file_simple() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let path_str = file_path.to_str().unwrap();

    let tool = FileWriteTool;
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "file_path": path_str,
        "content": "Hello, World!"
    });

    let result = tool.call(input, context).await.unwrap();
    let output_str = result.content;
    let output: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    
    assert_eq!(output["bytes_written"], 13);
    
    // Verify file was written
    let content = fs::read_to_string(&file_path).await.unwrap();
    assert_eq!(content, "Hello, World!");
}

#[tokio::test]
async fn test_write_file_creates_parent_dirs() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("subdir/nested/test.txt");
    let path_str = file_path.to_str().unwrap();

    let tool = FileWriteTool;
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "file_path": path_str,
        "content": "Nested content"
    });

    let result = tool.call(input, context).await.unwrap();
    
    // Verify file and parent dirs were created
    assert!(file_path.exists());
    let content = fs::read_to_string(&file_path).await.unwrap();
    assert_eq!(content, "Nested content");
}

#[tokio::test]
async fn test_write_file_relative_path() {
    let temp_dir = TempDir::new().unwrap();

    let tool = FileWriteTool;
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "file_path": "relative.txt",
        "content": "Relative path content"
    });

    let result = tool.call(input, context).await.unwrap();
    
    // Verify file was created in working directory
    let file_path = temp_dir.path().join("relative.txt");
    assert!(file_path.exists());
}