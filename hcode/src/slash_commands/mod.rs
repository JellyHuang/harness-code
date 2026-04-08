//! Slash commands for interactive mode.

mod clear;
mod compact;
mod doctor;
mod exit;
mod help;
mod registry;

pub use clear::ClearCommand;
pub use compact::CompactCommand;
pub use doctor::DoctorCommand;
pub use exit::ExitCommand;
pub use help::HelpCommand;
pub use registry::{CommandRegistry, CommandContext, CommandResult, CommandError};

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
    async fn execute(&self, args: Vec<String>, context: CommandContext) -> Result<CommandResult, CommandError>;
}