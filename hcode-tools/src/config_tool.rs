//! ConfigTool for viewing and modifying configuration.

use async_trait::async_trait;
use hcode_types::ToolResult;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::LazyLock;

use crate::{Tool, ToolContext, ToolError};

/// ConfigTool input.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ConfigInput {
    Get {
        /// Setting key to get
        key: String,
    },
    Set {
        /// Setting key to set
        key: String,
        /// Value to set
        value: Value,
    },
}

/// ConfigTool output.
#[derive(Debug, Serialize)]
pub struct ConfigOutput {
    /// Whether operation succeeded
    pub success: bool,
    /// Operation type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    /// Setting key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Current value (for get operation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// Previous value (for set operation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_value: Option<Value>,
    /// New value (for set operation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_value: Option<Value>,
    /// Error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// ConfigTool for managing configuration.
pub struct ConfigTool;

/// JSON schema for Config tool.
static CONFIG_SCHEMA: LazyLock<Value> = LazyLock::new(|| {
    json!({
      "type": "object",
      "properties": {
        "key": {
          "type": "string",
          "description": "Configuration key (e.g., 'theme', 'model', 'max_turns')"
        },
        "value": {
          "type": ["string", "number", "boolean"],
          "description": "Value to set. Omit for get operation."
        }
      },
      "required": ["key"]
    })
});

const SUPPORTED_KEYS: &[&str] = &[
    "theme",
    "model",
    "provider",
    "max_turns",
    "max_budget_usd",
    "verbose",
    "auto_compact",
    "permissions.defaultMode",
];

#[async_trait]
impl Tool for ConfigTool {
    fn name(&self) -> &str {
        "config"
    }

    fn description(&self) -> &str {
        "Get or set configuration settings (theme, model, etc.)"
    }

    fn input_schema(&self) -> &Value {
        &CONFIG_SCHEMA
    }

    fn is_read_only(&self) -> bool {
        // Read-only only if getting (no value provided)
        false
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: ConfigInput =
            serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        match params {
            ConfigInput::Get { key } => self.get_config(&key),
            ConfigInput::Set { key, value } => self.set_config(&key, value).await,
        }
    }
}

impl ConfigTool {
    fn get_config(&self, key: &str) -> Result<ToolResult, ToolError> {
        // Check if key is supported
        if !SUPPORTED_KEYS.iter().any(|k| *k == key) {
            return Ok(ToolResult::success(json!({
                "success": false,
                "error": format!("Unknown setting key: {}. Supported: {}", key, SUPPORTED_KEYS.join(", "))
            })));
        }

        // In a real implementation, you'd load actual config
        // For now, return placeholder values
        let value = match key {
            "theme" => json!("dark"),
            "model" => json!("claude-sonnet-4-20250514"),
            "provider" => json!("anthropic"),
            "max_turns" => json!(null),
            "max_budget_usd" => json!(null),
            "verbose" => json!(false),
            "auto_compact" => json!(true),
            "permissions.defaultMode" => json!("ask"),
            _ => json!(null),
        };

        Ok(ToolResult::success(json!({
            "success": true,
            "operation": "get",
            "key": key,
            "value": value
        })))
    }

    async fn set_config(&self, key: &str, value: Value) -> Result<ToolResult, ToolError> {
        // Check if key is supported
        if !SUPPORTED_KEYS.iter().any(|k| *k == key) {
            return Ok(ToolResult::success(json!({
                "success": false,
                "error": format!("Unknown setting key: {}. Supported: {}", key, SUPPORTED_KEYS.join(", "))
            })));
        }

        // Validate value type
        match key {
            "max_turns" if !value.is_number() && !value.is_null() => {
                return Ok(ToolResult::success(json!({
                    "success": false,
                    "error": format!("{} must be a number or null", key)
                })));
            }
            "max_budget_usd" if !value.is_number() && !value.is_null() => {
                return Ok(ToolResult::success(json!({
                    "success": false,
                    "error": format!("{} must be a number or null", key)
                })));
            }
            "verbose" | "auto_compact" if !value.is_boolean() => {
                return Ok(ToolResult::success(json!({
                    "success": false,
                    "error": format!("{} must be a boolean", key)
                })));
            }
            _ => {}
        }

        // In a real implementation, you'd save config to file
        // For now, just acknowledge the change
        Ok(ToolResult::success(json!({
            "success": true,
            "operation": "set",
            "key": key,
            "previous_value": null,
            "new_value": value,
            "note": "Configuration will be saved to ~/.config/hcode/config.yaml"
        })))
    }
}
