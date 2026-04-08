//! Compact command - triggers conversation compaction.

use super::{Command, CommandContext, CommandResult, CommandError};
use async_trait::async_trait;

/// Compact command.
pub struct CompactCommand;

#[async_trait]
impl Command for CompactCommand {
    fn name(&self) -> &str {
        "compact"
    }
    
    fn description(&self) -> &str {
        "Trigger conversation compaction (coming soon)"
    }
    
    async fn execute(&self, _args: Vec<String>, _context: CommandContext) -> Result<CommandResult, CommandError> {
        Ok(CommandResult::success("Compaction feature coming soon."))
    }
}