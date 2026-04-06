//! CLI argument definitions.

use clap::{Args, Subcommand};

#[derive(Args)]
pub struct GlobalArgs {
    /// Path to config file
    #[arg(long, global = true)]
    pub config_path: Option<String>,

    /// Print loaded config and exit (for debugging)
    #[arg(long, global = true)]
    pub debug_config: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run an interactive session
    Run {
        /// Initial prompt
        #[arg(short, long)]
        prompt: Option<String>,

        /// Provider to use (overrides config)
        #[arg(long)]
        provider: Option<String>,

        /// Model to use (overrides config)
        #[arg(short, long)]
        model: Option<String>,
    },

    /// Manage agents
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },

    /// Show version
    Version,
}

#[derive(Subcommand)]
pub enum AgentCommand {
    /// List available agents
    List,

    /// Run a specific agent
    Run {
        /// Agent type
        agent_type: String,
        /// Prompt for the agent
        prompt: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Show current config
    Show,

    /// Validate config file
    Validate,

    /// Set a config value
    Set { key: String, value: String },

    /// Generate a default config file
    Init {
        /// Overwrite existing config
        #[arg(short, long)]
        force: bool,
    },
}
