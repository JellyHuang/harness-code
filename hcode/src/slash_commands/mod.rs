//! Slash commands for interactive mode.

mod branch;
mod clear;
mod commit;
mod compact;
mod config;
mod diff;
mod doctor;
mod exit;
mod help;
mod mcp;
mod registry;
mod skill;
mod status;

#[allow(unused_imports)]
pub use branch::BranchCommand;
pub use clear::ClearCommand;
pub use commit::CommitCommand;
pub use compact::CompactCommand;
pub use config::ConfigCommand;
pub use diff::DiffCommand;
pub use doctor::DoctorCommand;
pub use exit::ExitCommand;
pub use help::HelpCommand;
#[allow(unused_imports)]
pub use mcp::McpCommand;
pub use registry::{CommandContext, CommandError, CommandRegistry, CommandResult};
pub use skill::SkillCommand;
#[allow(unused_imports)]
pub use status::StatusCommand;

use async_trait::async_trait;

/// Command trait for slash commands.
#[async_trait]
pub trait Command: Send + Sync {
    /// Get command name (without leading slash).
    fn name(&self) -> &str;

    /// Get command description.
    fn description(&self) -> &str;

    /// Get command aliases.
    fn aliases(&self) -> Vec<&str> {
        vec![]
    }

    /// Execute the command.
    async fn execute(
        &self,
        args: Vec<String>,
        context: CommandContext,
    ) -> Result<CommandResult, CommandError>;
}
