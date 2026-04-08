//! Interactive REPL session management.
//!
//! Provides the InteractiveSession struct for multi-turn conversations.

use anyhow::Result;
use hcode_config::Config;
use hcode_engine::{QueryEngine, QueryEngineConfig, QueryOutput, TerminalReason};
use hcode_provider::ProviderRegistry;
use hcode_session::{Session, Storage};
use hcode_tools::{Tool, ToolRegistry};
use hcode_types::Message;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::signal;

/// Slash command types.
#[derive(Debug, Clone, PartialEq)]
pub enum SlashCommand {
    Exit,
    Clear,
    Help,
    Compact,
    Cd(String),
}

/// Parse slash command from input.
pub fn parse_slash_command(input: &str) -> Option<SlashCommand> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let cmd = parts.first()?;

    match *cmd {
        "/exit" | "/quit" => Some(SlashCommand::Exit),
        "/clear" => Some(SlashCommand::Clear),
        "/help" => Some(SlashCommand::Help),
        "/compact" => Some(SlashCommand::Compact),
        "/cd" => {
            let path = parts.get(1).unwrap_or(&"").to_string();
            Some(SlashCommand::Cd(path))
        }
        _ => None,
    }
}

/// Interactive session configuration.
pub struct InteractiveConfig {
    /// Working directory
    pub cwd: PathBuf,

    /// Provider name
    pub provider_name: String,

    /// Model name
    pub model: String,

    /// Show thinking blocks
    pub show_thinking: bool,

    /// Verbose output
    pub verbose: bool,

    /// Session storage
    pub storage: Option<Arc<dyn Storage>>,

    /// App config (for provider registry)
    pub app_config: Config,
}

/// Interactive REPL session.
pub struct InteractiveSession {
    /// Session ID
    session_id: String,

    /// Accumulated messages
    messages: Vec<Message>,

    /// Provider registry
    provider_registry: ProviderRegistry,

    /// Configuration
    config: InteractiveConfig,

    /// Cancellation flag
    cancelled: bool,
}

impl InteractiveSession {
    /// Create new interactive session.
    pub fn new(config: InteractiveConfig) -> Self {
        let session_id = uuid::Uuid::new_v4().to_string();
        let provider_registry = ProviderRegistry::from_config(&config.app_config);

        Self {
            session_id,
            messages: Vec::new(),
            provider_registry,
            config,
            cancelled: false,
        }
    }

    /// Get session ID.
    #[allow(dead_code)]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get accumulated messages.
    #[allow(dead_code)]
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Add message to conversation.
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Clear all messages.
    pub fn clear_messages(&mut self) {
        self.messages.clear();
        println!("[Conversation cleared]");
    }

    /// Cancel the session.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// Check if cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    /// Save session to storage.
    pub fn save(&self) -> Result<()> {
        if let Some(storage) = &self.config.storage {
            let session = Session {
                id: self.session_id.clone(),
                messages: self.messages.clone(),
            };
            storage.save(&session)?;
            println!("[Session saved: {}]", self.session_id);
        }
        Ok(())
    }

    /// Run the interactive REPL loop.
    pub async fn run(&mut self) -> Result<()> {
        // Display session header
        println!();
        println!("╔════════════════════════════════════════════════════════════╗");
        println!("║  HCode Interactive Mode                                    ║");
        println!("╠════════════════════════════════════════════════════════════╣");
        println!(
            "║  Session: {}                               ║",
            &self.session_id[..8]
        );
        println!(
            "║  Provider: {:<15}  Model: {:<20}  ║",
            self.config.provider_name, self.config.model
        );
        println!(
            "║  Working Dir: {:<43}║",
            truncate(self.config.cwd.to_str().unwrap_or("?"), 43)
        );
        println!("║  Type /help for commands, /exit to quit                    ║");
        println!("╚════════════════════════════════════════════════════════════╝");
        println!();

        // Setup Ctrl+C handler
        let ctrl_c = signal::ctrl_c();
        tokio::pin!(ctrl_c);

        // Setup stdin reader
        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        // REPL loop
        loop {
            // Check cancellation
            if self.is_cancelled() {
                self.save()?;
                println!("\n[Session ended]");
                return Ok(());
            }

            // Display prompt
            println!();
            print!("> ");
            io::stdout().flush()?;

            // Read input with Ctrl+C handling
            tokio::select! {
                _ = &mut ctrl_c => {
                    println!("\n[Ctrl+C received]");
                    self.save()?;
                    println!("[Session saved and exited]");
                    return Ok(());
                }

                line = lines.next_line() => {
                    match line {
                        Ok(Some(input)) => {
                            self.process_input(input).await?;
                        }
                        Ok(None) => {
                            // EOF (Ctrl+D)
                            println!("\n[EOF received]");
                            self.save()?;
                            return Ok(());
                        }
                        Err(e) => {
                            eprintln!("[Input error: {}]", e);
                            continue;
                        }
                    }
                }
            }
        }
    }

    /// Process user input.
    async fn process_input(&mut self, input: String) -> Result<()> {
        let trimmed = input.trim();

        // Handle empty input
        if trimmed.is_empty() {
            return Ok(());
        }

        // Handle slash commands
        if let Some(cmd) = parse_slash_command(trimmed) {
            self.handle_slash_command(cmd).await?;
            return Ok(());
        }

        // Process as regular message
        self.process_message(trimmed.to_string()).await?;

        Ok(())
    }

    /// Handle slash command.
    async fn handle_slash_command(&mut self, cmd: SlashCommand) -> Result<()> {
        match cmd {
            SlashCommand::Exit => {
                self.save()?;
                println!("[Session saved. Exiting...]");
                self.cancel();
            }
            SlashCommand::Clear => {
                self.clear_messages();
            }
            SlashCommand::Help => {
                self.show_help();
            }
            SlashCommand::Compact => {
                // Show conversation stats
                let msg_count = self.messages.len();
                let user_count = self
                    .messages
                    .iter()
                    .filter(|m| matches!(m, Message::User(_)))
                    .count();
                let assistant_count = self
                    .messages
                    .iter()
                    .filter(|m| matches!(m, Message::Assistant(_)))
                    .count();

                println!();
                println!("Conversation Stats:");
                println!("  Total messages: {}", msg_count);
                println!("  User messages: {}", user_count);
                println!("  Assistant messages: {}", assistant_count);
                println!();
                println!("[Auto-compaction is triggered when context exceeds threshold]");
                println!("[Manual compaction will be available in a future update]");
            }
            SlashCommand::Cd(path) => {
                self.change_directory(&path)?;
            }
        }
        Ok(())
    }

    /// Change working directory.
    fn change_directory(&mut self, path: &str) -> Result<()> {
        let new_dir = if path.is_empty() || path == "~" {
            dirs::home_dir().unwrap_or_default()
        } else if path.starts_with('~') {
            let home = dirs::home_dir().unwrap_or_default();
            let rest = path.trim_start_matches('~');
            home.join(rest.trim_start_matches('/'))
        } else {
            self.config.cwd.join(path)
        };

        if new_dir.exists() && new_dir.is_dir() {
            self.config.cwd = new_dir.clone();
            println!("[Working directory changed to: {}]", new_dir.display());
        } else {
            println!("[Error: Directory does not exist: {}]", new_dir.display());
        }

        Ok(())
    }

    /// Show help message.
    fn show_help(&self) {
        println!();
        println!("Available commands:");
        println!("  /exit, /quit  - Save session and exit");
        println!("  /clear        - Clear conversation history");
        println!("  /help         - Show this help message");
        println!("  /compact      - Show conversation stats (auto-compaction enabled)");
        println!("  /cd <path>    - Change working directory");
        println!();
        println!("Shortcuts:");
        println!("  Ctrl+C        - Save and exit");
        println!("  Ctrl+D        - Exit without save");
        println!();
    }

    /// Process user message through QueryEngine.
    async fn process_message(&mut self, prompt: String) -> Result<()> {
        // Add user message
        let user_message = Message::user_text(&prompt);
        self.add_message(user_message.clone());

        // Get provider
        let provider = self
            .provider_registry
            .get(&self.config.provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found", self.config.provider_name))?;

        // Create QueryEngine with accumulated messages
        let engine_config = QueryEngineConfig {
            cwd: self.config.cwd.clone(),
            initial_messages: self.messages.clone(),
            verbose: self.config.verbose,
            ..Default::default()
        };

        // Create tool registry with all default tools
        let tools = Arc::new(ToolRegistryAdapter(Arc::new(ToolRegistry::with_default_tools())));

        let engine = QueryEngine::new(engine_config, tools, provider.clone());

        // Stream response
        println!();
        let stream = engine.submit_message(prompt);
        futures::pin_mut!(stream);

        while let Some(output) = futures::StreamExt::next(&mut stream).await {
            match output {
                QueryOutput::StreamEvent(event) => {
                    self.handle_stream_event(event)?;
                }
                QueryOutput::ToolResult {
                    tool_use_id: _,
                    tool_name,
                    result,
                    is_error,
                } => {
                    if is_error {
                        eprintln!("[Tool error: {}] {}", tool_name, truncate(&result, 200));
                    } else {
                        println!("[Tool: {}] {}", tool_name, truncate(&result, 200));
                    }
                }
                QueryOutput::Progress { tool_use_id, data } => {
                    println!("[Progress: {}] {}", tool_use_id, truncate(&data, 50));
                }
                QueryOutput::Complete {
                    reason,
                    duration_ms,
                    total_cost_usd,
                    usage,
                } => {
                    println!();
                    println!("────────────────────────────────────────");
                    match reason {
                        TerminalReason::Completed => {
                            println!(
                                "[Completed in {}ms, cost: ${:.4}]",
                                duration_ms, total_cost_usd
                            );
                        }
                        TerminalReason::MaxTurns {
                            turn_count,
                            max_turns,
                        } => {
                            println!("[Max turns reached: {} / {}]", turn_count, max_turns);
                        }
                        TerminalReason::BudgetExceeded {
                            cost_usd,
                            max_budget_usd,
                        } => {
                            println!(
                                "[Budget exceeded: ${:.2} / ${:.2}]",
                                cost_usd, max_budget_usd
                            );
                        }
                        TerminalReason::Cancelled => {
                            println!("[Cancelled]");
                        }
                        TerminalReason::Error { message } => {
                            eprintln!("[Error: {}]", message);
                        }
                        _ => {
                            println!("[Ended: {:?}]", reason);
                        }
                    }
                    println!(
                        "[Tokens: {} in, {} out]",
                        usage.input_tokens, usage.output_tokens
                    );
                }
                QueryOutput::Error { message } => {
                    eprintln!("[Error: {}]", message);
                }
            }
        }

        // Update accumulated messages from engine
        self.messages = engine.get_messages();

        Ok(())
    }

    /// Handle stream event.
    fn handle_stream_event(&self, event: hcode_engine::StreamEventOutput) -> Result<()> {
        match event.event_type.as_str() {
            "content_block_delta" => {
                if let Some(text) = &event.content {
                    // Skip "index=" prefix for actual content
                    if !text.starts_with("index=") {
                        print!("{}", text);
                        io::stdout().flush()?;
                    }
                }
            }
            "thinking_delta" => {
                if self.config.show_thinking {
                    if let Some(thinking) = &event.content {
                        print!("\x1b[36m{}\x1b[0m", thinking);
                        io::stdout().flush()?;
                    }
                }
            }
            "message_start" | "message_stop" | "message_delta" => {
                // Skip these for cleaner output
            }
            "content_block_start" | "content_block_stop" => {
                if event.tool_use.is_some() {
                    if let Some(tool) = &event.tool_use {
                        println!("\n[Tool: {}]", tool.name);
                    }
                }
            }
            _ => {
                if self.config.verbose {
                    println!("[Event: {}]", event.event_type);
                }
            }
        }
        Ok(())
    }
}

/// Truncate string to max length (UTF-8 safe).
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        // Use chars() to handle UTF-8 correctly
        let truncated: String = s.chars().take(max_len).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_generation() {
        let config = InteractiveConfig {
            cwd: PathBuf::from("/test"),
            provider_name: "anthropic".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            show_thinking: false,
            verbose: false,
            storage: None,
            app_config: Config::default(),
        };
        let session = InteractiveSession::new(config);
        assert!(!session.session_id().is_empty());
        assert_eq!(session.session_id().len(), 36); // UUID format
    }

    #[test]
    fn test_slash_command_parsing() {
        assert_eq!(parse_slash_command("/exit"), Some(SlashCommand::Exit));
        assert_eq!(parse_slash_command("/quit"), Some(SlashCommand::Exit));
        assert_eq!(parse_slash_command("/clear"), Some(SlashCommand::Clear));
        assert_eq!(parse_slash_command("/help"), Some(SlashCommand::Help));
        assert_eq!(parse_slash_command("/compact"), Some(SlashCommand::Compact));
        assert_eq!(parse_slash_command("/unknown"), None);
        assert_eq!(parse_slash_command("regular text"), None);
        assert_eq!(parse_slash_command(""), None);
    }

    #[test]
    fn test_message_accumulation() {
        let config = InteractiveConfig {
            cwd: PathBuf::from("/test"),
            provider_name: "anthropic".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            show_thinking: false,
            verbose: false,
            storage: None,
            app_config: Config::default(),
        };
        let mut session = InteractiveSession::new(config);
        assert!(session.messages().is_empty());

        session.add_message(Message::user_text("Hello"));
        assert_eq!(session.messages().len(), 1);

        session.add_message(Message::user_text("World"));
        assert_eq!(session.messages().len(), 2);

        session.clear_messages();
        assert!(session.messages().is_empty());
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 100), "short");
        assert_eq!(truncate("very long string here", 10), "very long ...");
    }
}

/// Adapter to wrap hcode_tools::ToolRegistry as hcode_engine::ToolRegistry
struct ToolRegistryAdapter(Arc<ToolRegistry>);

#[async_trait::async_trait]
impl hcode_engine::ToolRegistry for ToolRegistryAdapter {
    fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.0.get(name)
    }

    fn list(&self) -> Vec<Arc<dyn Tool>> {
        self.0.tools()
    }
}
