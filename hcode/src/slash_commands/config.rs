//! Config command - view and modify configuration.

use super::{Command, CommandContext, CommandError, CommandResult};
use async_trait::async_trait;
use hcode_config::load_config;

/// Config command.
pub struct ConfigCommand;

#[async_trait]
impl Command for ConfigCommand {
    fn name(&self) -> &str {
        "config"
    }

    fn description(&self) -> &str {
        "View or modify configuration settings"
    }

    fn aliases(&self) -> Vec<&str> {
        vec!["cfg", "settings"]
    }

    async fn execute(
        &self,
        args: Vec<String>,
        _context: CommandContext,
    ) -> Result<CommandResult, CommandError> {
        if args.is_empty() {
            // Show all config
            return self.show_all_config();
        }

        let command = args[0].to_lowercase();

        match command.as_str() {
            "show" | "get" | "list" => self.show_all_config(),
            "help" => Ok(CommandResult::success(Self::help_text())),
            _ => Err(CommandError::InvalidArgs(format!(
                "Unknown config command: {}. Use: show, help",
                command
            ))),
        }
    }
}

impl ConfigCommand {
    fn show_all_config(&self) -> Result<CommandResult, CommandError> {
        let config = load_config()
            .map_err(|e| CommandError::Failed(format!("Failed to load config: {}", e)))?;

        let mut output = String::from("Current Configuration:\n\n");

        if let Some(ref model) = config.model {
            output.push_str(&format!("Model: {}\n", model));
        }

        if !config.provider.is_empty() {
            output.push_str("\nProviders:\n");
            for (name, provider) in &config.provider {
                if provider.options.api_key.is_some() {
                    output.push_str(&format!("  {} - API key configured\n", name));
                } else {
                    output.push_str(&format!("  {} - No API key\n", name));
                }
            }
        }

        if let Some(ref data_dir) = config.data_dir {
            output.push_str(&format!("\nData Directory: {}\n", data_dir));
        }

        output.push_str(&format!("\nDebug Mode: {}\n", config.debug));

        output.push_str("\nUse /config help for more options");

        Ok(CommandResult::success(output))
    }

    fn help_text() -> String {
        r#"Config Command Usage:

/config show       - Show all configuration
/config list       - Same as show
/config help       - Show this help

To modify configuration, edit the config file directly:
- ~/.config/hcode/config.json
- ./hcode.json
- ./hcode.jsonc
"#
        .to_string()
    }
}
