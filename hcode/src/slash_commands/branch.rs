//! Branch command - manage git branches.

use super::{Command, CommandContext, CommandError, CommandResult};
use async_trait::async_trait;
use std::process::Command as ProcessCommand;

/// Branch command.
pub struct BranchCommand;

#[async_trait]
impl Command for BranchCommand {
    fn name(&self) -> &str {
        "branch"
    }

    fn description(&self) -> &str {
        "Manage git branches (list, create, switch, delete)"
    }

    fn aliases(&self) -> Vec<&str> {
        vec!["br"]
    }

    async fn execute(
        &self,
        args: Vec<String>,
        context: CommandContext,
    ) -> Result<CommandResult, CommandError> {
        if args.is_empty() {
            // List all branches
            return self.list_branches(&context.working_dir);
        }

        let subcommand = args[0].to_lowercase();

        match subcommand.as_str() {
            "list" | "ls" => self.list_branches(&context.working_dir),
            "create" | "c" => {
                if args.len() < 2 {
                    return Err(CommandError::InvalidArgs(
                        "Usage: /branch create <name> [base]".to_string(),
                    ));
                }
                let branch_name = &args[1];
                let base = args.get(2).map(|s| s.as_str()).unwrap_or("HEAD");
                self.create_branch(&context.working_dir, branch_name, base)
            }
            "switch" | "checkout" | "s" => {
                if args.len() < 2 {
                    return Err(CommandError::InvalidArgs(
                        "Usage: /branch switch <name>".to_string(),
                    ));
                }
                self.switch_branch(&context.working_dir, &args[1])
            }
            "delete" | "d" | "rm" => {
                if args.len() < 2 {
                    return Err(CommandError::InvalidArgs(
                        "Usage: /branch delete <name> [-f]".to_string(),
                    ));
                }
                let force =
                    args.contains(&"-f".to_string()) || args.contains(&"--force".to_string());
                self.delete_branch(&context.working_dir, &args[1], force)
            }
            "current" => self.current_branch(&context.working_dir),
            "help" => Ok(CommandResult::success(self.help_text())),
            _ => Err(CommandError::InvalidArgs(format!(
                "Unknown branch subcommand: {}. Use: list, create, switch, delete, current",
                subcommand
            ))),
        }
    }
}

impl BranchCommand {
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

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CommandError::Failed(format!("Git error: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn list_branches(&self, cwd: &std::path::Path) -> Result<CommandResult, CommandError> {
        let branches = self.run_git_command(cwd, &["branch", "-a"])?;
        let current = self.run_git_command(cwd, &["branch", "--show-current"])?;

        let mut output = String::new();
        output.push_str(&format!("Current branch: {}\n\n", current.trim()));
        output.push_str("All branches:\n");
        output.push_str(&branches);

        Ok(CommandResult::success(output))
    }

    fn current_branch(&self, cwd: &std::path::Path) -> Result<CommandResult, CommandError> {
        let branch = self.run_git_command(cwd, &["branch", "--show-current"])?;
        Ok(CommandResult::success(format!(
            "Current branch: {}",
            branch.trim()
        )))
    }

    fn create_branch(
        &self,
        cwd: &std::path::Path,
        name: &str,
        base: &str,
    ) -> Result<CommandResult, CommandError> {
        self.run_git_command(cwd, &["branch", name, base])?;
        Ok(CommandResult::success(format!(
            "✓ Created branch '{}' from {}",
            name, base
        )))
    }

    fn switch_branch(
        &self,
        cwd: &std::path::Path,
        name: &str,
    ) -> Result<CommandResult, CommandError> {
        self.run_git_command(cwd, &["checkout", name])?;
        Ok(CommandResult::success(format!(
            "✓ Switched to branch '{}'",
            name
        )))
    }

    fn delete_branch(
        &self,
        cwd: &std::path::Path,
        name: &str,
        force: bool,
    ) -> Result<CommandResult, CommandError> {
        let flag = if force { "-D" } else { "-d" };
        self.run_git_command(cwd, &["branch", flag, name])?;
        Ok(CommandResult::success(format!(
            "✓ Deleted branch '{}'",
            name
        )))
    }

    fn help_text(&self) -> String {
        r#"Branch Command Usage:

/branch                  - List all branches
/branch list            - Same as above
/branch current         - Show current branch
/branch create <name> [base] - Create a new branch
/branch switch <name>   - Switch to a branch
/branch delete <name> [-f] - Delete a branch

Examples:
  /branch create feature/login
  /branch create hotfix bug-fix-123 main
  /branch switch main
  /branch delete old-feature -f
"#
        .to_string()
    }
}
