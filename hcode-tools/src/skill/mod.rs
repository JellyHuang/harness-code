//! Skill system for user-defined capabilities.

mod schema;
mod loader;

use crate::{Tool, ToolContext, ToolError};
use async_trait::async_trait;
use hcode_types::ToolResult;
use parking_lot::RwLock;
use serde_json::{json, Value};
use std::sync::LazyLock;
use std::sync::Arc;

pub use schema::*;
pub use loader::*;

/// Skill executor.
pub struct SkillExecutor {
    loader: Arc<RwLock<SkillLoader>>,
}

impl SkillExecutor {
    /// Create a new skill executor.
    pub fn new(loader: SkillLoader) -> Self {
        Self {
            loader: Arc::new(RwLock::new(loader)),
        }
    }

    /// Execute a skill.
    pub async fn execute(&self, input: SkillInput) -> Result<SkillOutput, SkillError> {
        let loader = self.loader.read();
        let skill = loader.get(&input.skill_name)
            .ok_or_else(|| SkillError::NotFound(input.skill_name.clone()))?
            .clone();
        drop(loader);
        
        let mut steps_completed = 0;
        let mut results = Vec::new();
        
        for step in &skill.steps {
            // Interpolate parameters into prompt
            let prompt = self.interpolate_prompt(&step.prompt, &input.parameters);
            
            // Execute step (placeholder - would integrate with QueryEngine)
            let result = format!("Step {} completed: {}", steps_completed + 1, prompt);
            
            results.push(result);
            steps_completed += 1;
        }
        
        Ok(SkillOutput {
            skill_name: input.skill_name,
            result: results.join("\n\n"),
            steps_completed,
            success: true,
        })
    }

    /// Interpolate parameters into prompt.
    fn interpolate_prompt(&self, prompt: &str, params: &Option<Value>) -> String {
        let mut result = prompt.to_string();
        
        if let Some(params) = params {
            if let Value::Object(map) = params {
                for (key, value) in map {
                    let placeholder = format!("{{{}}}", key);
                    if let Some(s) = value.as_str() {
                        result = result.replace(&placeholder, s);
                    }
                }
            }
        }
        
        result
    }

    /// Reload skills.
    pub fn reload(&self) -> Result<Vec<SkillDefinition>, SkillError> {
        let mut loader = self.loader.write();
        loader.load_all()
    }
}

/// Skill tool for executing skills.
pub struct SkillTool {
    executor: Option<Arc<SkillExecutor>>,
}

impl Default for SkillTool {
    fn default() -> Self {
        Self { executor: None }
    }
}

impl SkillTool {
    /// Create a new skill tool.
    pub fn new(executor: Arc<SkillExecutor>) -> Self {
        Self { executor: Some(executor) }
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str {
        "skill"
    }

    fn description(&self) -> &str {
        "Execute a user-defined skill"
    }

    fn input_schema(&self) -> &Value {
        static SKILL_SCHEMA: LazyLock<Value> = LazyLock::new(|| json!({
            "type": "object",
            "properties": {
                "skill_name": {
                    "type": "string",
                    "description": "Name of the skill to execute"
                },
                "parameters": {
                    "type": "object",
                    "description": "Parameters for the skill"
                }
            },
            "required": ["skill_name"]
        }));
        &SKILL_SCHEMA
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: SkillInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;
        
        // If no executor, return placeholder response
        let output = if let Some(executor) = &self.executor {
            executor.execute(params).await
                .map_err(|e| ToolError::Execution(e.to_string()))?
        } else {
            SkillOutput {
                skill_name: params.skill_name,
                result: "Skill system not initialized".to_string(),
                steps_completed: 0,
                success: false,
            }
        };
        
        Ok(ToolResult::success(
            serde_json::to_value(output).unwrap()
        ))
    }
}