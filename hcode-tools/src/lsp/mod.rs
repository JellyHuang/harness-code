//! LSPTool for code intelligence.

use async_trait::async_trait;
use hcode_types::ToolResult;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::LazyLock;

use crate::{Tool, ToolContext, ToolError};

/// LSP action types.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LspAction {
    Definition,
    References,
    Hover,
    Completion,
    Rename,
    Symbols,
}

/// LSPTool input.
#[derive(Debug, Deserialize)]
pub struct LspInput {
    /// Action to perform.
    pub action: LspAction,
    
    /// File path.
    pub file_path: String,
    
    /// Line number (1-indexed).
    pub line: usize,
    
    /// Column number (0-indexed).
    pub column: usize,
    
    /// New name for rename action.
    #[serde(default)]
    pub new_name: Option<String>,
}

/// LSP location result.
#[derive(Debug, Serialize)]
pub struct LspLocation {
    /// File path.
    pub file_path: String,
    
    /// Line number.
    pub line: usize,
    
    /// Column number.
    pub column: usize,
    
    /// Optional text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// LSPTool output.
#[derive(Debug, Serialize)]
pub struct LspOutput {
    /// Action performed.
    pub action: String,
    
    /// Results.
    pub results: Vec<LspLocation>,
    
    /// Total count.
    pub total: usize,
}

/// LSPTool for code intelligence.
pub struct LspTool;

/// JSON schema for LSP tool.
static LSP_SCHEMA: LazyLock<Value> = LazyLock::new(|| json!({
    "type": "object",
    "properties": {
        "action": {
            "type": "string",
            "enum": ["definition", "references", "hover", "completion", "rename", "symbols"],
            "description": "LSP action to perform"
        },
        "file_path": {
            "type": "string",
            "description": "File path"
        },
        "line": {
            "type": "number",
            "description": "Line number (1-indexed)",
            "minimum": 1
        },
        "column": {
            "type": "number",
            "description": "Column number (0-indexed)",
            "minimum": 0
        },
        "new_name": {
            "type": "string",
            "description": "New name for rename action"
        }
    },
    "required": ["action", "file_path", "line", "column"]
}));

#[async_trait]
impl Tool for LspTool {
    fn name(&self) -> &str {
        "lsp"
    }

    fn description(&self) -> &str {
        "Get code intelligence via LSP (definitions, references, etc.)"
    }

    fn input_schema(&self) -> &Value {
        &LSP_SCHEMA
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: LspInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        // Note: Full LSP integration requires language server setup
        // This placeholder returns simulated results
        
        let results = match params.action {
            LspAction::Definition => {
                vec![LspLocation {
                    file_path: params.file_path.clone(),
                    line: params.line + 5,
                    column: params.column,
                    text: Some("fn example() {".to_string()),
                }]
            }
            LspAction::References => {
                vec![
                    LspLocation {
                        file_path: params.file_path.clone(),
                        line: params.line,
                        column: params.column,
                        text: None,
                    }
                ]
            }
            _ => vec![]
        };

        let total = results.len();

        Ok(ToolResult::success(
            serde_json::to_value(LspOutput {
                action: match params.action {
                    LspAction::Definition => "definition",
                    LspAction::References => "references",
                    LspAction::Hover => "hover",
                    LspAction::Completion => "completion",
                    LspAction::Rename => "rename",
                    LspAction::Symbols => "symbols",
                }.to_string(),
                results,
                total,
            }).unwrap()
        ))
    }
}