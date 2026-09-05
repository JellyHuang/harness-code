//! Skill command - list and manage skills.

use super::{Command, CommandContext, CommandError, CommandResult};
use async_trait::async_trait;
use hcode_tools::skill::SkillLoader;

/// Skill command.
pub struct SkillCommand;

#[async_trait]
impl Command for SkillCommand {
    fn name(&self) -> &str {
        "skills"
    }

    fn description(&self) -> &str {
        "List available skills"
    }

    fn aliases(&self) -> Vec<&str> {
        vec!["skill"]
    }

    async fn execute(
        &self,
        args: Vec<String>,
        _context: CommandContext,
    ) -> Result<CommandResult, CommandError> {
        if args.is_empty() {
            // List all skills
            return self.list_skills();
        }

        let command = args[0].to_lowercase();

        match command.as_str() {
            "list" | "ls" => self.list_skills(),
            "reload" => self.reload_skills(),
            "help" => Ok(CommandResult::success(Self::help_text())),
            _ => Err(CommandError::InvalidArgs(format!(
                "Unknown skill command: {}. Use: list, reload, help",
                command
            ))),
        }
    }
}

impl SkillCommand {
    fn list_skills(&self) -> Result<CommandResult, CommandError> {
        let mut loader = SkillLoader::with_defaults();
        let skills = loader
            .load_all()
            .map_err(|e| CommandError::Failed(format!("Failed to load skills: {}", e)))?;

        if skills.is_empty() {
            return Ok(CommandResult::success(
                "No skills found.\n\nCreate skills in:\n  - .hcode/skills/\n  - .opencode/skills/\n  - ~/.config/hcode/skills/"
            ));
        }

        let mut output = String::new();
        output.push_str(&format!("Available Skills ({} total):\n\n", skills.len()));

        for skill in &skills {
            output.push_str(&format!(
                "  {name} - {desc}\n",
                name = skill.name,
                desc = skill.description
            ));

            if let Some(ref version) = skill.version {
                output.push_str(&format!("    Version: {}\n", version));
            }

            if let Some(ref author) = skill.author {
                output.push_str(&format!("    Author: {}\n", author));
            }

            output.push_str(&format!("    Steps: {}\n", skill.steps.len()));

            if let Some(ref trigger) = skill.trigger {
                let trigger_desc = match trigger {
                    hcode_tools::skill::SkillTrigger::Manual => "manual".to_string(),
                    hcode_tools::skill::SkillTrigger::Keyword { keywords } => {
                        format!("keywords: {}", keywords.join(", "))
                    }
                    hcode_tools::skill::SkillTrigger::Regex { pattern } => {
                        format!("regex: {}", pattern)
                    }
                };
                output.push_str(&format!("    Trigger: {}\n", trigger_desc));
            }

            output.push('\n');
        }

        output.push_str("Use /skills reload to refresh skills from disk");

        Ok(CommandResult::success(output))
    }

    fn reload_skills(&self) -> Result<CommandResult, CommandError> {
        let mut loader = SkillLoader::with_defaults();
        let skills = loader
            .load_all()
            .map_err(|e| CommandError::Failed(format!("Failed to reload skills: {}", e)))?;

        Ok(CommandResult::success(format!(
            "Successfully reloaded {} skills",
            skills.len()
        )))
    }

    fn help_text() -> String {
        r#"Skill Command Usage:

/skills           - List all available skills
/skills list      - Same as above
/skills reload    - Reload skills from disk
/skills help      - Show this help

Skill Directories:
  - .hcode/skills/       (project-level)
  - .opencode/skills/    (OpenCode project)
  - ~/.config/hcode/skills/ (user-level)

Skill Format:
  Skills are defined in Markdown files with optional YAML frontmatter.
  Example structure:
    .hcode/skills/
      my-skill/
        SKILL.md

SKILL.md frontmatter format:
  ---
  name: my-skill
  description: My custom skill
  version: 1.0.0
  author: Your Name
  ---

  Skill instructions go here...
"#
        .to_string()
    }
}
