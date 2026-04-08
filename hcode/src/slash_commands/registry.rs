//! Command registry.

use super::Command;
use std::collections::HashMap;
use std::sync::Arc;

/// Command execution context.
#[derive(Debug, Clone)]
pub struct CommandContext {
    /// Working directory.
    pub working_dir: std::path::PathBuf,
    
    /// Session ID.
    pub session_id: String,
}

impl CommandContext {
    pub fn new(working_dir: std::path::PathBuf, session_id: String) -> Self {
        Self { working_dir, session_id }
    }
}

/// Command result.
#[derive(Debug)]
pub struct CommandResult {
    /// Output text.
    pub output: String,
    
    /// Whether the command succeeded.
    pub success: bool,
    
    /// Whether to exit the application.
    pub should_exit: bool,
}

impl CommandResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            success: true,
            should_exit: false,
        }
    }
    
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            output: message.into(),
            success: false,
            should_exit: false,
        }
    }
    
    pub fn exit(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            success: true,
            should_exit: true,
        }
    }
}

/// Command error.
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Command failed: {0}")]
    Failed(String),
    
    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),
    
    #[error("Not available: {0}")]
    NotAvailable(String),
    
    #[error("Command not found: {0}")]
    NotFound(String),
}

/// Registry for managing slash commands.
pub struct CommandRegistry {
    commands: HashMap<String, Arc<dyn Command>>,
}

impl CommandRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }
    
    /// Create registry with default commands.
    pub fn with_default_commands() -> Self {
        let mut registry = Self::new();
        
        registry.register(Arc::new(super::ClearCommand));
        registry.register(Arc::new(super::CompactCommand));
        registry.register(Arc::new(super::DoctorCommand));
        registry.register(Arc::new(super::ExitCommand));
        registry.register(Arc::new(super::HelpCommand::new(Arc::new(Self::new()))));
        
        registry
    }
    
    /// Register a command.
    pub fn register(&mut self, command: Arc<dyn Command>) {
        self.commands.insert(command.name().to_string(), command.clone());
        
        // Register aliases
        for alias in command.aliases() {
            self.commands.insert(alias.to_string(), command.clone());
        }
    }
    
    /// Get a command by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Command>> {
        self.commands.get(name).cloned()
    }
    
    /// List all command names.
    pub fn list(&self) -> Vec<String> {
        self.commands.keys()
            .filter(|k| {
                // Filter out aliases (only show primary names)
                self.commands.values().any(|c| c.name() == *k)
            })
            .cloned()
            .collect()
    }
    
    /// Execute a command by name.
    pub async fn execute(&self, name: &str, args: Vec<String>, context: CommandContext) -> Result<CommandResult, CommandError> {
        let command = self.get(name)
            .ok_or_else(|| CommandError::NotFound(name.to_string()))?;
        
        command.execute(args, context).await
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}