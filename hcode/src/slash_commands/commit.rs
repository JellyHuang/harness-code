//! Commit command - create a git commit.

use super::{Command, CommandContext, CommandError, CommandResult};
use async_trait::async_trait;
use std::process::Command as ProcessCommand;

/// Commit command.
pub struct CommitCommand;

#[async_trait]
impl Command for CommitCommand {
    fn name(&self) -> &str {
        "commit"
    }

    fn description(&self) -> &str {
        "Create a git commit with staged changes"
    }

    fn aliases(&self) -> Vec<&str> {
        vec!["ci"]
    }

    async fn execute(
        &self,
        args: Vec<String>,
        context: CommandContext,
    ) -> Result<CommandResult, CommandError> {
        // Get git status
        let status = self.run_git_command(&context.working_dir, &["status", "--porcelain"])?;

        if status.is_empty() {
            return Ok(CommandResult::success(
                "No changes to commit. Working tree is clean.".to_string(),
            ));
        }

        // Get git diff
        let diff = self.run_git_command(&context.working_dir, &["diff", "--stat", "HEAD"])?;

        // Get current branch
        let branch = self.run_git_command(&context.working_dir, &["branch", "--show-current"])?;

        // Get recent commits
        let recent = self.run_git_command(&context.working_dir, &["log", "--oneline", "-5"])?;

        let mut output = String::new();
        output.push_str("## Git Status\n\n");
        output.push_str(&format!("Branch: {}\n\n", branch.trim()));

        output.push_str("## Changes to Commit:\n");
        output.push_str(&diff);
        output.push_str("\n\n");

        output.push_str("## Detailed Status:\n");
        output.push_str(&status);
        output.push_str("\n\n");

        output.push_str("## Recent Commits:\n");
        output.push_str(&recent);
        output.push_str("\n\n");

        if args.is_empty() {
            output.push_str("## Usage:\n");
            output.push_str("  /commit <message> - Create commit with the given message\n");
            output.push_str("  /commit -a <message> - Stage all changes and commit\n\n");
            output.push_str("Example: /commit feat: add new feature\n");
        } else {
            // Parse arguments
            let (stage_all, message) = if args[0] == "-a" {
                (true, args[1..].join(" "))
            } else {
                (false, args.join(" "))
            };

            if message.is_empty() {
                return Err(CommandError::InvalidArgs(
                    "Commit message cannot be empty".to_string(),
                ));
            }

            // Stage files if -a flag
            if stage_all {
                let add_output = self.run_git_command(&context.working_dir, &["add", "-A"])?;
                if !add_output.is_empty() {
                    output.push_str(&format!("Staged:\n{}\n\n", add_output));
                }
            }

            // Create commit with HEREDOC-style message
            let commit_output = self.create_commit(&context.working_dir, &message)?;
            output.push_str(&commit_output);
        }

        Ok(CommandResult::success(output))
    }
}

impl CommitCommand {
    fn run_git_command(
        &self,
        cwd: &std::path::Path,
        args: &[&str],
    ) -> Result<String, CommandError> {
        let output = ProcessCommand::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .map_err(|e| CommandError::Failed(format!("Failed to run git: {}", e)))?;

        if !output.status.success() && !args.contains(&"diff") {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CommandError::Failed(format!("Git error: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn create_commit(&self, cwd: &std::path::Path, message: &str) -> Result<String, CommandError> {
        // Check if there are changes to commit
        let status = self.run_git_command(cwd, &["status", "--porcelain"])?;
        if status.is_empty() {
            return Ok("No changes to commit.".to_string());
        }

        // Create commit using -m flag
        let output = ProcessCommand::new("git")
            .args(["commit", "-m", message])
            .current_dir(cwd)
            .output()
            .map_err(|e| CommandError::Failed(format!("Failed to create commit: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CommandError::Failed(format!("Commit failed: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(format!("✓ Commit created successfully:\n{}", stdout))
    }
}
