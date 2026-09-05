//! Diff command - show git differences.

use super::{Command, CommandContext, CommandError, CommandResult};
use async_trait::async_trait;
use std::process::Command as ProcessCommand;

/// Diff command.
pub struct DiffCommand;

#[async_trait]
impl Command for DiffCommand {
    fn name(&self) -> &str {
        "diff"
    }

    fn description(&self) -> &str {
        "Show changes between commits, commit and working tree, etc."
    }

    fn aliases(&self) -> Vec<&str> {
        vec!["d"]
    }

    async fn execute(
        &self,
        args: Vec<String>,
        context: CommandContext,
    ) -> Result<CommandResult, CommandError> {
        if args.is_empty() {
            // Show working tree diff
            return self.show_diff(&context.working_dir, &["HEAD"]);
        }

        let arg = &args[0];

        match arg.as_str() {
            "--cached" | "--staged" => {
                // Show staged changes
                self.show_diff(&context.working_dir, &["--cached"])
            }
            "help" => Ok(CommandResult::success(self.help_text())),
            _ => {
                // Show diff against specific commit/ref
                self.show_diff(&context.working_dir, &[arg])
            }
        }
    }
}

impl DiffCommand {
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

        // Git diff returns 1 when there are differences (not an error)
        if !output.status.success() && output.status.code() != Some(1) {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CommandError::Failed(format!("Git error: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn show_diff(
        &self,
        cwd: &std::path::Path,
        args: &[&str],
    ) -> Result<CommandResult, CommandError> {
        let mut git_args = vec!["diff"];
        git_args.extend_from_slice(args);

        let diff = self.run_git_command(cwd, &git_args)?;

        if diff.is_empty() {
            if args.is_empty() || args[0] == "--cached" {
                return Ok(CommandResult::success("No changes detected.".to_string()));
            }
            return Ok(CommandResult::success(format!(
                "No differences found against {:?}",
                args.first()
            )));
        }

        // Get diff stat summary
        let mut stat_args = vec!["diff", "--stat"];
        stat_args.extend_from_slice(args);
        let stat = self.run_git_command(cwd, &stat_args).unwrap_or_default();

        let mut output = String::new();
        output.push_str("## Git Diff\n\n");
        if !stat.is_empty() {
            output.push_str("### Summary:\n");
            output.push_str(&stat);
            output.push_str("\n\n");
        }
        output.push_str("### Changes:\n");
        output.push_str(&diff);

        Ok(CommandResult::success(output))
    }

    fn help_text(&self) -> String {
        r#"Diff Command Usage:

/diff                  - Show changes in working tree vs HEAD
/diff --cached        - Show staged changes
/diff <commit>        - Show changes against a specific commit
/diff --help          - Show this help

Examples:
  /diff HEAD~1        - Show changes since last commit
  /diff main          - Show changes against main branch
  /diff --cached      - Show staged changes only
"#
        .to_string()
    }
}
