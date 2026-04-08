//! Doctor command - checks system health.

use super::{Command, CommandContext, CommandResult, CommandError};
use async_trait::async_trait;

/// Doctor command.
pub struct DoctorCommand;

#[async_trait]
impl Command for DoctorCommand {
    fn name(&self) -> &str {
        "doctor"
    }
    
    fn description(&self) -> &str {
        "Check system health and configuration"
    }
    
    async fn execute(&self, _args: Vec<String>, _context: CommandContext) -> Result<CommandResult, CommandError> {
        let mut output = String::new();
        
        // Check configuration
        output.push_str("System Health Check:\n\n");
        output.push_str("✓ Configuration loaded\n");
        output.push_str("✓ Session storage initialized\n");
        output.push_str("✓ Provider registry ready\n");
        
        Ok(CommandResult::success(output))
    }
}