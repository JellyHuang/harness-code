//! Help command - displays available commands.

use super::{Command, CommandContext, CommandResult, CommandError, CommandRegistry};
use async_trait::async_trait;
use std::sync::Arc;

/// Help command.
pub struct HelpCommand {
    registry: Arc<CommandRegistry>,
}

impl HelpCommand {
    pub fn new(registry: Arc<CommandRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl Command for HelpCommand {
    fn name(&self) -> &str {
        "help"
    }
    
    fn description(&self) -> &str {
        "Display available commands"
    }
    
    fn aliases(&self) -> Vec<&str> {
        vec!["h", "?"]
    }
    
    async fn execute(&self, _args: Vec<String>, _context: CommandContext) -> Result<CommandResult, CommandError> {
        let mut output = String::new();
        output.push_str("Available Commands:\n\n");
        
        for name in self.registry.list() {
            if let Some(cmd) = self.registry.get(&name) {
                output.push_str(&format!("/{name} - {desc}\n", name = name, desc = cmd.description()));
            }
        }
        
        output.push_str("\nKeyboard Shortcuts:\n");
        output.push_str("  Ctrl+C - Save session and exit\n");
        output.push_str("  Ctrl+D - Exit without saving\n");
        
        Ok(CommandResult::success(output))
    }
}