# Tools Migration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Migrate core tools from cc-haha TypeScript to hcode-rust, enabling file operations and bash command execution.

**Architecture:** Implement tools as Rust structs implementing the Tool trait defined in hcode-tools/src/tool.rs. Each tool will have input schema (JSON Schema), permission checking, and async execution. Tools register with ToolRegistry for orchestration.

**Tech Stack:** Rust async (tokio), serde_json for schemas, thiserror for errors, tempfile for testing

---

## Migration Overview

### Phase 1: Core File Tools (Priority: HIGH)
These tools are **essential** for basic functionality - the system currently cannot modify files.

| Tool | Purpose | Est. Complexity |
|------|---------|-----------------|
| BashTool | Execute shell commands | High (sandboxing, timeout) |
| FileReadTool | Read files/images/PDFs | Medium (multi-format) |
| FileWriteTool | Create/overwrite files | Low |
| FileEditTool | Diff-based file editing | Medium |
| GlobTool | Pattern-based file search | Low |
| GrepTool | Content search in files | Medium |

### Phase 2: Agent Communication Tools (Priority: HIGH)
Required for multi-agent coordination.

| Tool | Purpose | Est. Complexity |
|------|---------|-----------------|
| AgentTool | Spawn sub-agents | High |
| SendMessageTool | Send messages to coordinator | Low |
| TaskOutputTool | Background task output | Medium |

### Phase 3: Information Tools (Priority: MEDIUM)
Network and external resource access.

| Tool | Purpose | Est. Complexity |
|------|---------|-----------------|
| WebFetchTool | Fetch web content | Medium |
| WebSearchTool | Search web | Medium |

### Phase 4: UX Tools (Priority: LOW)
User interaction helpers.

| Tool | Purpose | Est. Complexity |
|------|---------|-----------------|
| TodoWriteTool | Todo management | Low |
| AskUserQuestionTool | Interactive prompts | Low |
| SkillTool | Skill execution | Medium |

---

## Task 1: BashTool Foundation

**Files:**
- Create: `hcode-tools/src/bash/mod.rs`
- Create: `hcode-tools/src/bash/schema.rs`
- Create: `hcode-tools/src/bash/executor.rs`
- Create: `hcode-tools/src/bash/sandbox.rs`
- Test: `hcode-tools/tests/bash_tool_test.rs`

### Step 1: Create bash module structure

Create the module files:

```rust
// hcode-tools/src/bash/mod.rs
//! Bash tool implementation.

mod schema;
mod executor;
mod sandbox;

use crate::{Tool, ToolContext, ToolError};
use async_trait::async_trait;
use hcode_types::ToolResult;
use serde_json::Value;
pub use schema::*;

/// Bash tool for executing shell commands.
pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a bash command and return its output"
    }

    fn input_schema(&self) -> &Value {
        &BASH_SCHEMA
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: BashInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;
        
        executor::execute(params, context).await
    }
}
```

```rust
// hcode-tools/src/bash/schema.rs
//! Input schema for Bash tool.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Default timeout in milliseconds (2 minutes).
pub const DEFAULT_TIMEOUT_MS: u64 = 120_000;

/// Maximum timeout in milliseconds (10 minutes).
pub const MAX_TIMEOUT_MS: u64 = 600_000;

/// Bash tool input parameters.
#[derive(Debug, Deserialize, Serialize)]
pub struct BashInput {
    /// The command to execute.
    pub command: String,
    
    /// Working directory for the command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workdir: Option<String>,
    
    /// Timeout in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    
    /// Run in background (don't wait for completion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_in_background: Option<bool>,
}

/// Bash tool output.
#[derive(Debug, Deserialize, Serialize)]
pub struct BashOutput {
    /// stdout content.
    pub stdout: String,
    
    /// stderr content.
    pub stderr: String,
    
    /// Exit code.
    pub exit_code: i32,
    
    /// Whether the command timed out.
    pub timed_out: bool,
}

/// JSON schema for Bash tool input.
pub static BASH_SCHEMA: Value = json!({
    "type": "object",
    "properties": {
        "command": {
            "type": "string",
            "description": "The command to execute"
        },
        "workdir": {
            "type": "string",
            "description": "Working directory for the command"
        },
        "timeout": {
            "type": "number",
            "description": "Timeout in milliseconds (max 600000)",
            "minimum": 1000,
            "maximum": MAX_TIMEOUT_MS
        },
        "run_in_background": {
            "type": "boolean",
            "description": "Run in background without waiting"
        }
    },
    "required": ["command"]
});
```

### Step 2: Implement basic executor (no sandbox yet)

```rust
// hcode-tools/src/bash/executor.rs
//! Bash command executor.

use super::schema::{BashInput, BashOutput, DEFAULT_TIMEOUT_MS, MAX_TIMEOUT_MS};
use crate::{ToolContext, ToolError};
use hcode_types::ToolResult;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Execute a bash command.
pub async fn execute(input: BashInput, context: ToolContext) -> Result<ToolResult, ToolError> {
    // Validate timeout
    let timeout_ms = input.timeout.unwrap_or(DEFAULT_TIMEOUT_MS);
    if timeout_ms > MAX_TIMEOUT_MS {
        return Err(ToolError::InvalidInput(
            format!("Timeout exceeds maximum of {}ms", MAX_TIMEOUT_MS)
        ));
    }

    // Determine working directory
    let workdir = input.workdir
        .map(|p| p.into())
        .unwrap_or(context.working_dir.clone());

    // Build command
    // On Windows, use cmd.exe /C; on Unix, use sh -c
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", &input.command]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", &input.command]);
        c
    };

    cmd.current_dir(&workdir);

    // Execute with timeout
    let duration = Duration::from_millis(timeout_ms);
    let result = timeout(duration, cmd.output()).await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(-1);

            Ok(ToolResult::success(
                serde_json::to_value(BashOutput {
                    stdout,
                    stderr,
                    exit_code,
                    timed_out: false,
                }).unwrap()
            ))
        }
        Ok(Err(e)) => {
            Err(ToolError::Execution(format!("Failed to execute command: {}", e)))
        }
        Err(_) => {
            // Timeout elapsed
            Ok(ToolResult::success(
                serde_json::to_value(BashOutput {
                    stdout: String::new(),
                    stderr: format!("Command timed out after {}ms", timeout_ms),
                    exit_code: -1,
                    timed_out: true,
                }).unwrap()
            ))
        }
    }
}
```

### Step 3: Create placeholder sandbox module

```rust
// hcode-tools/src/bash/sandbox.rs
//! Sandbox integration (placeholder for now).

/// Check if sandbox is enabled (placeholder).
pub fn is_sandbox_enabled() -> bool {
    false
}

/// Validate command against sandbox rules (placeholder).
pub fn validate_command(_command: &str) -> Result<(), String> {
    Ok(())
}
```

### Step 4: Write failing test

```rust
// hcode-tools/tests/bash_tool_test.rs
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
    let output: serde_json::Value = result.content;
    
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
    assert!(!result.content["timed_out"].as_bool().unwrap());
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
    assert!(result.content["timed_out"].as_bool().unwrap());
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
```

### Step 5: Update lib.rs to export BashTool

```rust
// hcode-tools/src/lib.rs
//! HCode Tools - Tool trait and implementations.

pub mod context;
pub mod registry;
pub mod result;
pub mod tool;
pub mod bash;  // Add this line

pub use registry::*;
pub use result::*;
pub use tool::*;
pub use bash::BashTool;  // Export BashTool
```

### Step 6: Run tests to verify they pass

Run: `cargo test --package hcode-tools`
Expected: PASS for bash_tool_test

### Step 7: Commit

```bash
git add hcode-tools/src/bash/ hcode-tools/src/lib.rs hcode-tools/tests/
git commit -m "feat(tools): implement BashTool with timeout support"
```

---

## Task 2: FileReadTool

**Files:**
- Create: `hcode-tools/src/file_read/mod.rs`
- Create: `hcode-tools/src/file_read/schema.rs`
- Create: `hcode-tools/src/file_read/reader.rs`
- Test: `hcode-tools/tests/file_read_tool_test.rs`

### Step 1: Create file_read module structure

```rust
// hcode-tools/src/file_read/mod.rs
//! File read tool implementation.

mod schema;
mod reader;

use crate::{Tool, ToolContext, ToolError};
use async_trait::async_trait;
use hcode_types::ToolResult;
use serde_json::Value;
pub use schema::*;

/// File read tool.
pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        "Read a file from the local filesystem"
    }

    fn input_schema(&self) -> &Value {
        &READ_SCHEMA
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: ReadInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;
        
        reader::read_file(params, context).await
    }
}
```

```rust
// hcode-tools/src/file_read/schema.rs
//! Input schema for FileRead tool.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Maximum file size to read (50MB).
pub const MAX_FILE_SIZE: usize = 50_000_000;

/// Maximum tokens to return (50K).
pub const MAX_TOKENS: usize = 50_000;

/// Read tool input parameters.
#[derive(Debug, Deserialize, Serialize)]
pub struct ReadInput {
    /// Absolute path to the file.
    pub file_path: String,
    
    /// Line number to start reading from (1-indexed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    
    /// Number of lines to read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

/// Read tool output.
#[derive(Debug, Serialize)]
pub struct ReadOutput {
    /// File path that was read.
    pub file_path: String,
    
    /// Content of the file (with line numbers).
    pub content: String,
    
    /// Number of lines returned.
    pub num_lines: usize,
    
    /// Starting line number.
    pub start_line: usize,
    
    /// Total lines in file.
    pub total_lines: usize,
}

/// JSON schema for Read tool input.
pub static READ_SCHEMA: Value = json!({
    "type": "object",
    "properties": {
        "file_path": {
            "type": "string",
            "description": "The absolute path to the file to read"
        },
        "offset": {
            "type": "number",
            "description": "The line number to start reading from (1-indexed)",
            "minimum": 1
        },
        "limit": {
            "type": "number",
            "description": "Number of lines to read",
            "minimum": 1
        }
    },
    "required": ["file_path"]
});
```

```rust
// hcode-tools/src/file_read/reader.rs
//! File reading implementation.

use super::schema::{ReadInput, ReadOutput, MAX_FILE_SIZE};
use crate::{ToolContext, ToolError};
use hcode_types::ToolResult;
use std::path::Path;
use tokio::fs;

/// Read a file.
pub async fn read_file(input: ReadInput, context: ToolContext) -> Result<ToolResult, ToolError> {
    let path = Path::new(&input.file_path);
    
    // Resolve relative paths from working directory
    let full_path = if path.is_relative() {
        context.working_dir.join(path)
    } else {
        path.to_path_buf()
    };

    // Check file exists
    if !full_path.exists() {
        return Err(ToolError::Execution(
            format!("File does not exist: {}", input.file_path)
        ));
    }

    // Check file size
    let metadata = fs::metadata(&full_path).await
        .map_err(|e| ToolError::Execution(format!("Failed to read file metadata: {}", e)))?;
    
    if metadata.len() > MAX_FILE_SIZE as u64 {
        return Err(ToolError::Execution(
            format!("File too large: {} bytes (max {})", metadata.len(), MAX_FILE_SIZE)
        ));
    }

    // Read file content
    let content = fs::read_to_string(&full_path).await
        .map_err(|e| ToolError::Execution(format!("Failed to read file: {}", e)))?;

    // Split into lines
    let all_lines: Vec<&str> = content.lines().collect();
    let total_lines = all_lines.len();

    // Apply offset and limit
    let offset = input.offset.unwrap_or(1);
    let offset_idx = (offset - 1).min(total_lines);
    
    let limit = input.limit.unwrap_or(total_lines - offset_idx);
    let end_idx = (offset_idx + limit).min(total_lines);
    
    let selected_lines = &all_lines[offset_idx..end_idx];
    
    // Add line numbers
    let numbered_content = selected_lines
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{}: {}", offset_idx + i + 1, line))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(ToolResult::success(
        serde_json::to_value(ReadOutput {
            file_path: input.file_path,
            content: numbered_content,
            num_lines: selected_lines.len(),
            start_line: offset,
            total_lines,
        }).unwrap()
    ))
}
```

### Step 2: Write tests

```rust
// hcode-tools/tests/file_read_tool_test.rs
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
    let output: serde_json::Value = result.content;
    
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
    let output: serde_json::Value = result.content;
    
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
```

### Step 3: Update lib.rs

```rust
// Add to hcode-tools/src/lib.rs
pub mod file_read;
pub use file_read::FileReadTool;
```

### Step 4: Run tests

Run: `cargo test --package hcode-tools`
Expected: PASS

### Step 5: Commit

```bash
git add hcode-tools/src/file_read/ hcode-tools/src/lib.rs hcode-tools/tests/file_read_tool_test.rs
git commit -m "feat(tools): implement FileReadTool with offset/limit support"
```

---

## Task 3: FileWriteTool

**Files:**
- Create: `hcode-tools/src/file_write/mod.rs`
- Create: `hcode-tools/src/file_write/schema.rs`
- Create: `hcode-tools/src/file_write/writer.rs`
- Test: `hcode-tools/tests/file_write_tool_test.rs`

### Step 1: Create file_write module

```rust
// hcode-tools/src/file_write/mod.rs
//! File write tool implementation.

mod schema;
mod writer;

use crate::{Tool, ToolContext, ToolError};
use async_trait::async_trait;
use hcode_types::ToolResult;
use serde_json::Value;
pub use schema::*;

pub struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "write"
    }

    fn description(&self) -> &str {
        "Write content to a file"
    }

    fn input_schema(&self) -> &Value {
        &WRITE_SCHEMA
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: WriteInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;
        
        writer::write_file(params, context).await
    }
}
```

```rust
// hcode-tools/src/file_write/schema.rs
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct WriteInput {
    pub file_path: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct WriteOutput {
    pub file_path: String,
    pub bytes_written: usize,
}

pub static WRITE_SCHEMA: Value = json!({
    "type": "object",
    "properties": {
        "file_path": {
            "type": "string",
            "description": "The absolute path to write to"
        },
        "content": {
            "type": "string",
            "description": "The content to write"
        }
    },
    "required": ["file_path", "content"]
});
```

```rust
// hcode-tools/src/file_write/writer.rs
use super::schema::{WriteInput, WriteOutput};
use crate::{ToolContext, ToolError};
use hcode_types::ToolResult;
use std::path::Path;
use tokio::fs;

pub async fn write_file(input: WriteInput, context: ToolContext) -> Result<ToolResult, ToolError> {
    let path = Path::new(&input.file_path);
    
    let full_path = if path.is_relative() {
        context.working_dir.join(path)
    } else {
        path.to_path_buf()
    };

    // Create parent directories if needed
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).await
            .map_err(|e| ToolError::Execution(format!("Failed to create directory: {}", e)))?;
    }

    // Write file
    fs::write(&full_path, &input.content).await
        .map_err(|e| ToolError::Execution(format!("Failed to write file: {}", e)))?;

    Ok(ToolResult::success(
        serde_json::to_value(WriteOutput {
            file_path: input.file_path,
            bytes_written: input.content.len(),
        }).unwrap()
    ))
}
```

### Step 2: Test, update lib.rs, run tests, commit (follow Task 2 pattern)

---

## Task 4: FileEditTool

**Files:**
- Create: `hcode-tools/src/file_edit/mod.rs`
- Create: `hcode-tools/src/file_edit/schema.rs`
- Create: `hcode-tools/src/file_edit/editor.rs`
- Test: `hcode-tools/tests/file_edit_tool_test.rs`

### Step 1: Implement diff-based editing

```rust
// hcode-tools/src/file_edit/mod.rs
mod schema;
mod editor;

use crate::{Tool, ToolContext, ToolError};
use async_trait::async_trait;
use hcode_types::ToolResult;
use serde_json::Value;
pub use schema::*;

pub struct FileEditTool;

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "edit"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing specific text"
    }

    fn input_schema(&self) -> &Value {
        &EDIT_SCHEMA
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: EditInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;
        
        editor::edit_file(params, context).await
    }
}
```

```rust
// hcode-tools/src/file_edit/schema.rs
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct EditInput {
    pub file_path: String,
    pub old_string: String,
    pub new_string: String,
    #[serde(default)]
    pub replace_all: bool,
}

#[derive(Debug, Serialize)]
pub struct EditOutput {
    pub file_path: String,
    pub replacements: usize,
}

pub static EDIT_SCHEMA: Value = json!({
    "type": "object",
    "properties": {
        "file_path": {
            "type": "string",
            "description": "The absolute path to edit"
        },
        "old_string": {
            "type": "string",
            "description": "The text to find and replace"
        },
        "new_string": {
            "type": "string",
            "description": "The text to replace with"
        },
        "replace_all": {
            "type": "boolean",
            "description": "Replace all occurrences"
        }
    },
    "required": ["file_path", "old_string", "new_string"]
});
```

```rust
// hcode-tools/src/file_edit/editor.rs
use super::schema::{EditInput, EditOutput};
use crate::{ToolContext, ToolError};
use hcode_types::ToolResult;
use std::path::Path;
use tokio::fs;

pub async fn edit_file(input: EditInput, context: ToolContext) -> Result<ToolResult, ToolError> {
    let path = Path::new(&input.file_path);
    
    let full_path = if path.is_relative() {
        context.working_dir.join(path)
    } else {
        path.to_path_buf()
    };

    // Read current content
    let content = fs::read_to_string(&full_path).await
        .map_err(|e| ToolError::Execution(format!("Failed to read file: {}", e)))?;

    // Perform replacement
    let new_content = if input.replace_all {
        content.replace(&input.old_string, &input.new_string)
    } else {
        // Replace only first occurrence
        if let Some(pos) = content.find(&input.old_string) {
            let mut result = content.clone();
            result.replace_range(pos..pos + input.old_string.len(), &input.new_string);
            result
        } else {
            return Err(ToolError::Execution("Text not found in file".to_string()));
        }
    };

    // Count replacements
    let count = if input.replace_all {
        content.matches(&input.old_string).count()
    } else {
        if content.contains(&input.old_string) { 1 } else { 0 }
    };

    if count == 0 && !input.replace_all {
        return Err(ToolError::Execution("Text not found in file".to_string()));
    }

    // Write back
    fs::write(&full_path, &new_content).await
        .map_err(|e| ToolError::Execution(format!("Failed to write file: {}", e)))?;

    Ok(ToolResult::success(
        serde_json::to_value(EditOutput {
            file_path: input.file_path,
            replacements: count,
        }).unwrap()
    ))
}
```

### Step 2: Test, update lib.rs, run tests, commit

---

## Task 5: GlobTool

**Files:**
- Create: `hcode-tools/src/glob/mod.rs`
- Create: `hcode-tools/src/glob/schema.rs`
- Create: `hcode-tools/src/glob/searcher.rs`

### Implementation

```rust
// hcode-tools/src/glob/searcher.rs
use super::schema::{GlobInput, GlobOutput};
use crate::{ToolContext, ToolError};
use hcode_types::ToolResult;
use glob::glob;
use std::path::Path;

pub async fn search_files(input: GlobInput, context: ToolContext) -> Result<ToolResult, ToolError> {
    let base_path = Path::new(&input.path.unwrap_or_else(|| ".".to_string()));
    
    let full_base = if base_path.is_relative() {
        context.working_dir.join(base_path)
    } else {
        base_path.to_path_buf()
    };

    let pattern = full_base.join(&input.pattern);
    let pattern_str = pattern.to_str().unwrap();

    let results: Vec<String> = glob(pattern_str)
        .map_err(|e| ToolError::InvalidInput(format!("Invalid pattern: {}", e)))?
        .filter_map(|entry| entry.ok())
        .take(input.limit.unwrap_or(100))
        .map(|path| path.to_str().unwrap().to_string())
        .collect();

    Ok(ToolResult::success(
        serde_json::to_value(GlobOutput {
            files: results,
            count: results.len(),
        }).unwrap()
    ))
}
```

---

## Task 6: GrepTool

**Files:**
- Create: `hcode-tools/src/grep/mod.rs`
- Create: `hcode-tools/src/grep/schema.rs`
- Create: `hcode-tools/src/grep/searcher.rs`

### Implementation (using regex crate)

```rust
// hcode-tools/src/grep/searcher.rs
use super::schema::{GrepInput, GrepOutput};
use crate::{ToolContext, ToolError};
use hcode_types::ToolResult;
use regex::Regex;
use std::path::Path;
use tokio::fs;
use walkdir::WalkDir;

pub async fn search_content(input: GrepInput, context: ToolContext) -> Result<ToolResult, ToolError> {
    let base_path = Path::new(&input.path.unwrap_or_else(|| ".".to_string()));
    
    let full_base = if base_path.is_relative() {
        context.working_dir.join(base_path)
    } else {
        base_path.to_path_buf()
    };

    let pattern = Regex::new(&input.pattern)
        .map_err(|e| ToolError::InvalidInput(format!("Invalid regex: {}", e)))?;

    let mut matches: Vec<GrepMatch> = Vec::new();
    let limit = input.limit.unwrap_or(100);

    for entry in WalkDir::new(&full_base)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if matches.len() >= limit {
            break;
        }

        let path = entry.path();
        
        // Skip binary files
        let content = fs::read_to_string(path).await;
        if content.is_err() {
            continue;
        }

        for (line_num, line) in content.unwrap().lines().enumerate() {
            if pattern.is_match(line) {
                matches.push(GrepMatch {
                    file: path.to_str().unwrap().to_string(),
                    line: line_num + 1,
                    content: line.to_string(),
                });
                
                if matches.len() >= limit {
                    break;
                }
            }
        }
    }

    Ok(ToolResult::success(
        serde_json::to_value(GrepOutput {
            matches,
            count: matches.len(),
        }).unwrap()
    ))
}
```

---

## Task 7: Register Tools in ToolRegistry

**Files:**
- Modify: `hcode-tools/src/registry.rs`

### Step 1: Add default tool registration

```rust
// Add to hcode-tools/src/registry.rs

impl ToolRegistry {
    /// Create registry with default tools.
    pub fn with_default_tools() -> Self {
        let mut registry = Self::new();
        
        // Core tools
        registry.register(Arc::new(BashTool));
        registry.register(Arc::new(FileReadTool));
        registry.register(Arc::new(FileWriteTool));
        registry.register(Arc::new(FileEditTool));
        registry.register(Arc::new(GlobTool));
        registry.register(Arc::new(GrepTool));
        
        registry
    }
}
```

### Step 2: Add imports

```rust
// Add to top of registry.rs
use crate::{BashTool, FileReadTool, FileWriteTool, FileEditTool, GlobTool, GrepTool};
```

### Step 3: Test registry

```rust
// Add to hcode-tools/tests/registry_test.rs
use hcode_tools::ToolRegistry;

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
```

### Step 4: Commit

```bash
git add hcode-tools/src/registry.rs hcode-tools/tests/registry_test.rs
git commit -m "feat(tools): add default tools registration to ToolRegistry"
```

---

## Task 8: Add Dependencies

**Files:**
- Modify: `hcode-tools/Cargo.toml`

### Step 1: Add required dependencies

```toml
# hcode-tools/Cargo.toml
[package]
name = "hcode-tools"
version.workspace = true
edition.workspace = true

[dependencies]
# Internal
hcode-types = { workspace = true }
hcode-permission = { workspace = true }

# Async
tokio = { workspace = true }
async-trait = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Error handling
thiserror = { workspace = true }

# File operations
glob = "0.3"
walkdir = "2"
regex = { workspace = true }

# Testing
tempfile = { workspace = true }
```

### Step 2: Verify build

Run: `cargo build --package hcode-tools`
Expected: SUCCESS

### Step 3: Commit

```bash
git add hcode-tools/Cargo.toml Cargo.lock
git commit -m "feat(tools): add dependencies for file search and regex"
```

---

## Task 9: Integration Test

**Files:**
- Create: `hcode-tools/tests/integration_test.rs`

### Step 1: Write integration test

```rust
// hcode-tools/tests/integration_test.rs
//! End-to-end integration test for tools.

use hcode_tools::{ToolRegistry, ToolContext};
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;
use std::io::Write;

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
    assert!(write_result.success);

    // Read the file
    let read_result = registry.execute(
        "read",
        json!({
            "file_path": "test.txt"
        }),
        context.clone(),
    ).await.unwrap();
    assert!(read_result.content["total_lines"] == 3);

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
    assert!(edit_result.content["replacements"] == 1);

    // Grep for pattern
    let grep_result = registry.execute(
        "grep",
        json!({
            "pattern": "line",
            "path": temp_dir.path().to_str().unwrap()
        }),
        context.clone(),
    ).await.unwrap();
    assert!(grep_result.content["count"].as_u64().unwrap() > 0);

    // Glob for files
    let glob_result = registry.execute(
        "glob",
        json!({
            "pattern": "*.txt",
            "path": temp_dir.path().to_str().unwrap()
        }),
        context.clone(),
    ).await.unwrap();
    assert!(glob_result.content["count"].as_u64().unwrap() >= 1);
}
```

### Step 2: Run test

Run: `cargo test --package hcode-tools integration_test`
Expected: PASS

### Step 3: Commit

```bash
git add hcode-tools/tests/integration_test.rs
git commit -m "test(tools): add integration test for full workflow"
```

---

## Execution Options

**Plan complete and saved to `docs/plans/2026-04-07-tools-migration.md`.**

**Two execution options:**

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration. Use when you want oversight and quick feedback loops.

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints. Use when you want to run implementation independently.

**Which approach?**

---

## Notes for Implementation

### Critical Considerations

1. **Cross-platform compatibility**: BashTool must handle Windows (`cmd.exe`) vs Unix (`sh`)
2. **Security**: Permission checks must be integrated with hcode-permission
3. **Error handling**: All tools should return ToolError with clear messages
4. **Testing**: Each tool needs unit tests + integration tests
5. **Sandboxing**: BashTool sandbox integration is deferred (Task 1 Step 3 placeholder)

### Next After Core Tools

After completing Tasks 1-9, continue with:
- Phase 2: AgentTool, SendMessageTool, TaskOutputTool (requires hcode-engine integration)
- Phase 3: WebFetchTool, WebSearchTool (requires HTTP client)
- Phase 4: TodoWriteTool, AskUserQuestionTool (requires UI integration)

### Dependencies Not Yet Added

These crates are needed but not in workspace Cargo.toml:
- `glob = "0.3"` - for GlobTool
- `walkdir = "2"` - for GrepTool file traversal

Add them during Task 8.