//! Exit command - exits the application.

use super::{Command, CommandContext, CommandResult, CommandError};
use async_trait::async_trait;

/// Exit command.
pub struct ExitCommand;

#[async_trait]
impl Command for ExitCommand {
    fn name(&self) -> &str {
        "exit"
    }
    
    fn description(&self) -> &str {
        "Exit the application"
    }
    
    fn aliases(&self) -> Vec<&str> {
        vec!["quit", "q"]
    }
    
    async fn execute(&self, _args: Vec<String>, _context: CommandContext) -> Result<CommandResult, CommandError> {
        Ok(CommandResult::exit("Goodbye! Session saved."))
    }
}