//! Clear command - clears conversation history.

use super::{Command, CommandContext, CommandResult, CommandError};
use async_trait::async_trait;

/// Clear command.
pub struct ClearCommand;

#[async_trait]
impl Command for ClearCommand {
    fn name(&self) -> &str {
        "clear"
    }
    
    fn description(&self) -> &str {
        "Clear conversation history"
    }
    
    fn aliases(&self) -> Vec<&str> {
        vec!["cls"]
    }
    
    async fn execute(&self, _args: Vec<String>, _context: CommandContext) -> Result<CommandResult, CommandError> {
        Ok(CommandResult::success("Conversation history cleared."))
    }
}