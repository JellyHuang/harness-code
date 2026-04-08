//! Skill loader.

use super::schema::SkillDefinition;
use std::path::{Path, PathBuf};
use std::fs;

/// Skill loader error.
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("Skill not found: {0}")]
    NotFound(String),
    
    #[error("Failed to load skill: {0}")]
    LoadError(String),
    
    #[error("Invalid skill format: {0}")]
    InvalidFormat(String),
    
    #[error("Skill execution failed: {0}")]
    ExecutionError(String),
}

/// Skill loader.
pub struct SkillLoader {
    /// Directories to search for skills.
    skill_dirs: Vec<PathBuf>,
    
    /// Cached skill definitions.
    cache: std::collections::HashMap<String, SkillDefinition>,
}

impl SkillLoader {
    /// Create a new skill loader.
    pub fn new(skill_dirs: Vec<PathBuf>) -> Self {
        Self {
            skill_dirs,
            cache: std::collections::HashMap::new(),
        }
    }

    /// Create with default directories.
    pub fn with_defaults() -> Self {
        let dirs = vec![
            PathBuf::from(".hcode/skills"),
            PathBuf::from(".opencode/skills"),
            dirs::config_dir()
                .map(|p| p.join("hcode").join("skills"))
                .unwrap_or_default(),
        ];
        
        Self::new(dirs)
    }

    /// Load all skills from directories.
    pub fn load_all(&mut self) -> Result<Vec<SkillDefinition>, SkillError> {
        let mut skills = Vec::new();
        
        for dir in &self.skill_dirs {
            if !dir.exists() {
                continue;
            }
            
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    
                    if path.extension().map(|e| e == "md" || e == "yaml" || e == "yml").unwrap_or(false) {
                        if let Ok(skill) = self.load_skill(&path) {
                            self.cache.insert(skill.name.clone(), skill.clone());
                            skills.push(skill);
                        }
                    }
                }
            }
        }
        
        Ok(skills)
    }

    /// Load a single skill.
    pub fn load_skill(&self, path: &Path) -> Result<SkillDefinition, SkillError> {
        let content = fs::read_to_string(path)
            .map_err(|e| SkillError::LoadError(e.to_string()))?;
        
        // Check for frontmatter
        if content.starts_with("---") {
            self.parse_frontmatter(&content)
        } else {
            // Simple markdown skill
            Ok(SkillDefinition {
                name: path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                description: "Loaded from file".to_string(),
                version: None,
                author: None,
                trigger: None,
                steps: vec![super::schema::SkillStep {
                    prompt: content,
                    tools: None,
                    condition: None,
                }],
            })
        }
    }

    /// Parse YAML frontmatter.
    fn parse_frontmatter(&self, content: &str) -> Result<SkillDefinition, SkillError> {
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        
        if parts.len() < 3 {
            return Err(SkillError::InvalidFormat("Invalid frontmatter".to_string()));
        }
        
        let frontmatter = parts[1].trim();
        let body = parts[2].trim();
        
        // Parse YAML frontmatter
        let mut skill: SkillDefinition = serde_yaml::from_str(frontmatter)
            .map_err(|e| SkillError::InvalidFormat(e.to_string()))?;
        
        // Add body as a step if not empty
        if !body.is_empty() && skill.steps.is_empty() {
            skill.steps.push(super::schema::SkillStep {
                prompt: body.to_string(),
                tools: None,
                condition: None,
            });
        }
        
        Ok(skill)
    }

    /// Get a skill by name.
    pub fn get(&self, name: &str) -> Option<&SkillDefinition> {
        self.cache.get(name)
    }

    /// List all loaded skills.
    pub fn list(&self) -> Vec<&SkillDefinition> {
        self.cache.values().collect()
    }
}