//! Skill schema definitions.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Skill definition.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillDefinition {
    /// Skill name.
    pub name: String,
    
    /// Skill description.
    pub description: String,
    
    /// Skill version.
    #[serde(default)]
    pub version: Option<String>,
    
    /// Skill author.
    #[serde(default)]
    pub author: Option<String>,
    
    /// Trigger for the skill.
    #[serde(default)]
    pub trigger: Option<SkillTrigger>,
    
    /// Steps to execute.
    pub steps: Vec<SkillStep>,
}

/// Skill trigger.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SkillTrigger {
    /// Manually invoked.
    Manual,
    
    /// Triggered by keywords.
    Keyword { keywords: Vec<String> },
    
    /// Triggered by regex pattern.
    Regex { pattern: String },
}

/// Skill step.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillStep {
    /// Prompt for this step.
    pub prompt: String,
    
    /// Tools available for this step.
    #[serde(default)]
    pub tools: Option<Vec<String>>,
    
    /// Condition to execute this step.
    #[serde(default)]
    pub condition: Option<String>,
}

/// Skill input for SkillTool.
#[derive(Debug, Clone, Deserialize)]
pub struct SkillInput {
    /// Skill name to execute.
    pub skill_name: String,
    
    /// Parameters for the skill.
    #[serde(default)]
    pub parameters: Option<Value>,
}

/// Skill output.
#[derive(Debug, Clone, Serialize)]
pub struct SkillOutput {
    /// Skill name.
    pub skill_name: String,
    
    /// Final result.
    pub result: String,
    
    /// Number of steps completed.
    pub steps_completed: usize,
    
    /// Whether the skill succeeded.
    pub success: bool,
}