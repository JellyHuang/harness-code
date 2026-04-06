//! Command implementations.

use crate::cli::{AgentCommand, Command, ConfigCommand, GlobalArgs};
use anyhow::Result;
use hcode_config::{load_config, load_config_from_path, Config};
use std::path::PathBuf;

/// Application context with global options.
pub struct AppContext {
    pub config: Config,
    pub config_path: Option<PathBuf>,
}

impl AppContext {
    /// Create context from global args.
    pub fn from_args(args: &GlobalArgs) -> Result<Self> {
        let (config, config_path) = if let Some(path) = &args.config_path {
            let path = PathBuf::from(path);
            (load_config_from_path(&path)?, Some(path))
        } else {
            let config = load_config()?;
            let path = hcode_config::default_config_path();
            (config, path)
        };

        // Debug output
        if args.debug_config {
            println!("Loaded config:");
            println!("{}", serde_json::to_string_pretty(&config)?);
            std::process::exit(0);
        }

        Ok(Self {
            config,
            config_path,
        })
    }
}

pub async fn execute(command: Command, ctx: &AppContext) -> Result<()> {
    match command {
        Command::Run {
            prompt,
            provider,
            model,
        } => run::execute(prompt, provider, model, ctx).await,
        Command::Agent { command } => match command {
            AgentCommand::List => agent::list(ctx).await,
            AgentCommand::Run { agent_type, prompt } => agent::run(agent_type, prompt, ctx).await,
        },
        Command::Config { command } => match command {
            ConfigCommand::Show => config::show(ctx).await,
            ConfigCommand::Validate => config::validate(ctx).await,
            ConfigCommand::Set { key, value } => config::set(key, value, ctx).await,
            ConfigCommand::Init { force } => config::init(force, ctx).await,
        },
        Command::Version => {
            println!("hcode {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}

mod agent;
mod config;
mod run;
