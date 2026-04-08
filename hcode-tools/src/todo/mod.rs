//! TodoWriteTool implementation.

use async_trait::async_trait;
use hcode_types::ToolResult;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::LazyLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::{Tool, ToolContext, ToolError};

/// Todo item.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TodoItem {
    /// Item content.
    pub content: String,
    
    /// Item status.
    pub status: String,
    
    /// Item priority.
    #[serde(default)]
    pub priority: Option<String>,
}

/// TodoWrite input.
#[derive(Debug, Deserialize)]
pub struct TodoWriteInput {
    /// Todo items.
    pub todos: Vec<TodoItem>,
}

/// TodoWrite output.
#[derive(Debug, Serialize)]
pub struct TodoWriteOutput {
    /// Updated todos.
    pub todos: Vec<TodoItem>,
    
    /// Number updated.
    pub updated: usize,
}

/// Todo manager for tracking todos.
pub struct TodoManager {
    todos: RwLock<Vec<TodoItem>>,
}

impl TodoManager {
    /// Create a new todo manager.
    pub fn new() -> Self {
        Self {
            todos: RwLock::new(Vec::new()),
        }
    }

    /// Update todos.
    pub fn update(&self, todos: Vec<TodoItem>) -> usize {
        let mut current = self.todos.write();
        let count = todos.len();
        *current = todos;
        count
    }

    /// Get todos.
    pub fn list(&self) -> Vec<TodoItem> {
        self.todos.read().clone()
    }
}

impl Default for TodoManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global todo managers by session.
static TODO_MANAGERS: LazyLock<RwLock<HashMap<String, Arc<TodoManager>>>> = 
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Get or create todo manager for a session.
pub fn get_todo_manager(session_id: &str) -> Arc<TodoManager> {
    let mut managers = TODO_MANAGERS.write();
    managers
        .entry(session_id.to_string())
        .or_insert_with(|| Arc::new(TodoManager::new()))
        .clone()
}

/// TodoWrite tool.
pub struct TodoWriteTool;

/// JSON schema for TodoWrite tool.
static TODO_WRITE_SCHEMA: LazyLock<Value> = LazyLock::new(|| json!({
    "type": "object",
    "properties": {
        "todos": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "content": { "type": "string" },
                    "status": { 
                        "type": "string",
                        "enum": ["pending", "in_progress", "completed"]
                    },
                    "priority": {
                        "type": "string",
                        "enum": ["high", "medium", "low"]
                    }
                },
                "required": ["content", "status"]
            }
        }
    },
    "required": ["todos"]
}));

#[async_trait]
impl Tool for TodoWriteTool {
    fn name(&self) -> &str {
        "todo_write"
    }

    fn description(&self) -> &str {
        "Manage a todo list for tracking tasks"
    }

    fn input_schema(&self) -> &Value {
        &TODO_WRITE_SCHEMA
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: TodoWriteInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        let manager = get_todo_manager(&context.session_id);
        let updated = manager.update(params.todos.clone());

        Ok(ToolResult::success(
            serde_json::to_value(TodoWriteOutput {
                todos: params.todos,
                updated,
            }).unwrap()
        ))
    }
}