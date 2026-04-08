//! Tests for FileRead tool.

use hcode_tools::{FileReadTool, Tool, ToolContext};
use serde_json::json;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use std::io::Write;

#[tokio::test]
async fn test_read_file_simple() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"line 1\nline 2\nline 3\n").unwrap();
    let path = temp_file.path().to_str().unwrap();

    let tool = FileReadTool;
    let context = ToolContext::new(
        PathBuf::from("."),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "file_path": path
    });

    let result = tool.call(input, context).await.unwrap();
    let output_str = result.content;
    let output: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    
    assert_eq!(output["total_lines"], 3);
    assert!(output["content"].as_str().unwrap().contains("line 1"));
}

#[tokio::test]
async fn test_read_file_with_offset() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"line 1\nline 2\nline 3\n").unwrap();
    let path = temp_file.path().to_str().unwrap();

    let tool = FileReadTool;
    let context = ToolContext::new(
        PathBuf::from("."),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "file_path": path,
        "offset": 2
    });

    let result = tool.call(input, context).await.unwrap();
    let output_str = result.content;
    let output: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    
    assert_eq!(output["start_line"], 2);
    assert!(!output["content"].as_str().unwrap().contains("line 1"));
    assert!(output["content"].as_str().unwrap().contains("line 2"));
}

#[tokio::test]
async fn test_read_file_not_found() {
    let tool = FileReadTool;
    let context = ToolContext::new(
        PathBuf::from("."),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "file_path": "/nonexistent/file.txt"
    });

    let result = tool.call(input, context).await;
    assert!(result.is_err());
}