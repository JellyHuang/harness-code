//! AskUserQuestionTool implementation.

use async_trait::async_trait;
use hcode_types::ToolResult;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{Tool, ToolContext, ToolError};

/// AskUserQuestion input.
#[derive(Debug, Deserialize)]
pub struct AskUserQuestionInput {
    /// Question to ask.
    pub question: String,
    
    /// Options for the user to choose from.
    #[serde(default)]
    pub options: Option<Vec<String>>,
    
    /// Allow custom answer.
    #[serde(default = "default_allow_custom")]
    pub allow_custom: bool,
}

fn default_allow_custom() -> bool { true }

/// AskUserQuestion output.
#[derive(Debug, Serialize)]
pub struct AskUserQuestionOutput {
    /// User's answer.
    pub answer: String,
    
    /// Selected option index (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_option: Option<usize>,
}

/// AskUserQuestion tool for interactive prompts.
pub struct AskUserQuestionTool;

#[async_trait]
impl Tool for AskUserQuestionTool {
    fn name(&self) -> &str {
        "ask_user"
    }

    fn description(&self) -> &str {
        "Ask the user a question and get their response"
    }

    fn input_schema(&self) -> &Value {
        &json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "The question to ask"
                },
                "options": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Options for user to choose from"
                },
                "allow_custom": {
                    "type": "boolean",
                    "default": true,
                    "description": "Allow custom answer"
                }
            },
            "required": ["question"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: AskUserQuestionInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        // Note: Full implementation requires UI integration
        // This placeholder returns a simulated response
        let answer = if let Some(options) = &params.options {
            if !options.is_empty() {
                format!("Please select from: {}", options.join(", "))
            } else {
                "Please provide your answer".to_string()
            }
        } else {
            "Please provide your answer".to_string()
        };

        Ok(ToolResult::success(
            serde_json::to_value(AskUserQuestionOutput {
                answer,
                selected_option: None,
            }).unwrap()
        ))
    }
}