//! Status command - show git status.

#![allow(dead_code)]

use super::{Command, CommandContext, CommandError, CommandResult};
use async_trait::async_trait;
use std::process::Command as ProcessCommand;

/// Status command.
pub struct StatusCommand;

#[async_trait]
impl Command for StatusCommand {
    fn name(&self) -> &str {
        "status"
    }

    fn description(&self) -> &str {
        "Show the working-tree status (git status)"
    }

    fn aliases(&self) -> Vec<&str> {
        vec!["st"]
    }

    async fn execute(
        &self,
        args: Vec<String>,
        context: CommandContext,
    ) -> Result<CommandResult, CommandError> {
        // Get current branch
        let branch = self.run_git_command(&context.working_dir, &["branch", "--show-current"])?;

        // Get remote info
        let remote_branch = self
            .run_git_command(
                &context.working_dir,
                &[
                    "rev-parse",
                    "--abbrev-ref",
                    "--symbolic-full-name",
                    "@{upstream",
                ],
            )
            .unwrap_or_default();

        // Get status (short format)
        let status_short =
            self.run_git_command(&context.working_dir, &["status", "--porcelain"])?;

        // Get status (long format)
        let status_long = self.run_git_command(&context.working_dir, &["status"])?;

        let mut output = String::new();
        output.push_str("## Git Status\n\n");
        output.push_str(&format!("Current Branch: **{}**\n\n", branch.trim()));

        if !remote_branch.is_empty() && remote_branch.trim() != branch.trim() {
            output.push_str(&format!("Upstream: {}\n\n", remote_branch.trim()));
        }

        // Check ahead/behind
        if !remote_branch.is_empty() {
            let ahead_behind = self
                .run_git_command(
                    &context.working_dir,
                    &[
                        "rev-list",
                        "--left-right",
                        "--count",
                        &format!("{}...{}", branch.trim(), remote_branch.trim()),
                    ],
                )
                .unwrap_or_default();

            if !ahead_behind.trim().is_empty() {
                let parts: Vec<&str> = ahead_behind.trim().split_whitespace().collect();
                if parts.len() >= 2 {
                    output.push_str(&format!(
                        "Ahead: {} commits, Behind: {} commits\n\n",
                        parts[0], parts[1]
                    ));
                }
            }
        }

        if status_short.is_empty() {
            output.push_str("✓ Working tree is clean - no changes to commit.\n");
        } else {
            output.push_str("### Changes:\n");
            output.push_str(&status_short);
            output.push_str("\n\n");

            output.push_str("### Detailed Status:\n");
            output.push_str(&status_long);
        }

        if args.iter().any(|a| a == "--help") {
            output.push_str("\n\n");
            output.push_str(&self.help_text());
        }

        Ok(CommandResult::success(output))
    }
}

impl StatusCommand {
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

        // Some git commands return non-zero but are not errors
        if !output.status.success() && output.status.code() != Some(1) {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CommandError::Failed(format!("Git error: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn help_text(&self) -> String {
        r#"Status Command Usage:

/status                - Show working tree status
/status --help        - Show this help

The status command shows:
- Current branch name
- Remote tracking branch
- Ahead/behind commit count vs remote
- Staged changes (ready to commit)
- Unstaged changes (not staged)
- Untracked files

Tip: Use /commit to create a commit with your changes.
"#
        .to_string()
    }
}
