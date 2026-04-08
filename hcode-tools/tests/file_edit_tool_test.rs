//! Tests for FileEdit tool.

use hcode_tools::{FileEditTool, Tool, ToolContext};
use serde_json::json;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use std::io::Write;
use tokio::fs;

#[tokio::test]
async fn test_edit_file_simple() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"Hello, World!\n").unwrap();
    let path = temp_file.path().to_str().unwrap();

    let tool = FileEditTool;
    let context = ToolContext::new(
        PathBuf::from("."),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "file_path": path,
        "old_string": "Hello",
        "new_string": "Hi"
    });

    let result = tool.call(input, context).await.unwrap();
    let output_str = result.content;
    let output: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    
    assert_eq!(output["replacements"], 1);
    
    // Verify file was edited
    let content = fs::read_to_string(temp_file.path()).await.unwrap();
    assert_eq!(content, "Hi, World!\n");
}

#[tokio::test]
async fn test_edit_file_replace_all() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"foo foo foo\n").unwrap();
    let path = temp_file.path().to_str().unwrap();

    let tool = FileEditTool;
    let context = ToolContext::new(
        PathBuf::from("."),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "file_path": path,
        "old_string": "foo",
        "new_string": "bar",
        "replace_all": true
    });

    let result = tool.call(input, context).await.unwrap();
    let output_str = result.content;
    let output: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    
    assert_eq!(output["replacements"], 3);
    
    // Verify all occurrences were replaced
    let content = fs::read_to_string(temp_file.path()).await.unwrap();
    assert_eq!(content, "bar bar bar\n");
}

#[tokio::test]
async fn test_edit_file_not_found() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"Hello\n").unwrap();
    let path = temp_file.path().to_str().unwrap();

    let tool = FileEditTool;
    let context = ToolContext::new(
        PathBuf::from("."),
        "test-session",
        "test-tool-use-id"
    );

    let input = json!({
        "file_path": path,
        "old_string": "NonExistent",
        "new_string": "Replacement"
    });

    let result = tool.call(input, context).await;
    assert!(result.is_err());
}