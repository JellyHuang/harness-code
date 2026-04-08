//! Run command.
//!
//! Handles both single-shot mode (`-p "prompt"`) and interactive REPL mode.

use crate::commands::AppContext;
use crate::interactive::{InteractiveConfig, InteractiveSession};
use anyhow::Result;
use futures::StreamExt;
use hcode_provider::ProviderRegistry;
use hcode_session::JsonStorage;
use hcode_types::Message;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

/// Execute the run command.
pub async fn execute(
    prompt: Option<String>,
    provider_override: Option<String>,
    model_override: Option<String>,
    ctx: &AppContext,
) -> Result<()> {
    // Create provider registry from config
    let registry = ProviderRegistry::from_config(&ctx.config);

    // Determine provider and model
    let provider_name = if let Some(name) = provider_override {
        // --provider flag explicitly given
        name
    } else if let Some(p) = ctx.config.model.as_ref().and_then(|m| {
        let (p, _) = hcode_config::Config::parse_model_string(m);
        p.map(|s| s.to_string())
    }) {
        // config.model is "provider/model" format, extract the provider part
        p
    } else if let Some(default_provider) = registry.get_default().map(|p| p.name().to_string()) {
        // use the first registered provider
        default_provider
    } else {
        // nothing configured at all
        return Err(anyhow::anyhow!(
            "No provider configured. Add a provider to your config file (~/.config/hcode/config.json).\n\
             Example:\n\
             {{\n  \"provider\": {{\n    \"anthropic\": {{ \"options\": {{ \"apiKey\": \"sk-ant-...\" }} }}\n  }}\n}}"
        ));
    };

    let model = if let Some(m) = model_override {
        // --model flag explicitly given
        m
    } else if let Some(m) = ctx.config.model.as_ref().and_then(|m| {
        let (p, model) = hcode_config::Config::parse_model_string(m);
        // only use it if it's a bare model name (no provider prefix)
        if p.is_none() { Some(model.to_string()) } else { None }
    }) {
        m
    } else if let Some(m) = ctx.config
        .provider
        .get(&provider_name)
        .and_then(|p| p.models.as_ref().and_then(|m| m.keys().next().cloned()))
    {
        // first model listed under this provider in config
        m
    } else {
        return Err(anyhow::anyhow!(
            "No model configured for provider '{}'. Add a model to your config file.\n\
             Example:\n\
             {{\n  \"provider\": {{\n    \"{}\": {{ \"models\": {{ \"your-model-id\": {{}} }} }}\n  }}\n}}",
            provider_name,
            provider_name
        ));
    };

    // Get provider
    let provider = registry.get(&provider_name).ok_or_else(|| {
        anyhow::anyhow!(
            "Provider '{}' is configured but failed to initialize (check API key and baseURL).",
            provider_name
        )
    })?;

    if let Some(p) = prompt {
        // Single-shot mode: execute prompt and exit
        run_single_shot(&p, provider_name, model, provider).await?;
    } else {
        // Interactive mode: start REPL
        run_interactive(provider_name, model, ctx).await?;
    }

    Ok(())
}

/// Run single-shot mode with a prompt.
async fn run_single_shot(
    prompt: &str,
    provider_name: String,
    model: String,
    provider: Arc<dyn hcode_provider::Provider>,
) -> Result<()> {
    println!("Provider: {}", provider_name);
    println!("Model: {}", model);
    println!();

    // Create message from prompt
    let messages = vec![Message::user_text(prompt)];

    // Build system prompt
    let system_prompt = Some(hcode_engine::QueryEngineConfig::default_system_prompt());

    // Make streaming API call
    println!("Sending request...");

    match provider.stream(messages, vec![], system_prompt).await {
        Ok(mut stream) => {
            while let Some(event) = stream.next().await {
                match event {
                    hcode_protocol::StreamEvent::MessageStart { .. } => {}
                    hcode_protocol::StreamEvent::ContentBlockStart { .. } => {}
                    hcode_protocol::StreamEvent::ContentBlockDelta { delta, .. } => {
                        match delta {
                            hcode_protocol::ContentDelta::Text { text } => {
                                print!("{}", text);
                                use std::io::Write;
                                std::io::stdout().flush().ok();
                            }
                            hcode_protocol::ContentDelta::Thinking { .. } => {
                                // Skip thinking output by default
                            }
                            hcode_protocol::ContentDelta::InputJsonDelta { .. } => {}
                        }
                    }
                    hcode_protocol::StreamEvent::ContentBlockStop { .. } => {}
                    hcode_protocol::StreamEvent::MessageDelta { stop_reason, usage } => {
                        if let Some(reason) = stop_reason {
                            eprintln!("\n[Stop: {}]", reason);
                        }
                        eprintln!(
                            "[Tokens: {} in, {} out]",
                            usage.input_tokens, usage.output_tokens
                        );
                    }
                    hcode_protocol::StreamEvent::MessageStop => {}
                    hcode_protocol::StreamEvent::Error { message } => {
                        eprintln!("\nError: {}", message);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("API Error: {}", e);
        }
    }

    Ok(())
}

/// Run interactive REPL mode.
async fn run_interactive(provider_name: String, model: String, ctx: &AppContext) -> Result<()> {
    // Create session storage
    let storage = Arc::new(JsonStorage::new()?);

    // Determine working directory
    let cwd = determine_working_directory()?;

    // Create interactive config
    let config = InteractiveConfig {
        cwd,
        provider_name,
        model,
        show_thinking: false,
        verbose: false,
        storage: Some(storage),
        app_config: ctx.config.clone(),
    };

    // Create and run interactive session
    let mut session = InteractiveSession::new(config);
    session.run().await?;

    Ok(())
}

/// Determine the working directory for the session.
///
/// Priority:
/// 1. Current directory if it's a git repository root
/// 2. Prompt user to enter a directory or use current
fn determine_working_directory() -> Result<PathBuf> {
    let current_dir = std::env::current_dir().unwrap_or_default();

    // Check if current directory is a git repository
    let is_git_repo = current_dir.join(".git").exists();

    if is_git_repo {
        return Ok(current_dir);
    }

    // Try to find parent git repository
    if let Some(git_root) = find_git_root(&current_dir) {
        println!();
        println!("Found git repository at: {}", git_root.display());
        println!("Current directory: {}", current_dir.display());
        println!();
        print!("Use this directory as working directory? [Y/n]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let answer = input.trim().to_lowercase();

        if answer == "n" || answer == "no" {
            return prompt_for_directory();
        }
        return Ok(current_dir);
    }

    // No git repository found, prompt user
    println!();
    println!("No git repository detected in current directory.");
    println!("Current directory: {}", current_dir.display());
    println!();
    print!("Enter working directory (or press Enter to use current): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let path = input.trim();

    if path.is_empty() {
        return Ok(current_dir);
    }

    let dir = PathBuf::from(path);
    if dir.exists() && dir.is_dir() {
        Ok(dir)
    } else {
        println!("Directory does not exist, using current directory.");
        Ok(current_dir)
    }
}

/// Find the root of a git repository by walking up the directory tree.
fn find_git_root(start: &PathBuf) -> Option<PathBuf> {
    let mut current = start.clone();

    while let Some(parent) = current.parent() {
        if parent.join(".git").exists() {
            return Some(parent.to_path_buf());
        }
        current = parent.to_path_buf();
    }

    None
}

/// Prompt user for a custom directory.
fn prompt_for_directory() -> Result<PathBuf> {
    println!();
    print!("Enter working directory path: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let path = input.trim();

    if path.is_empty() {
        return Ok(std::env::current_dir().unwrap_or_default());
    }

    let dir = PathBuf::from(path);
    if dir.exists() && dir.is_dir() {
        Ok(dir)
    } else {
        println!("Directory does not exist: {}", dir.display());
        println!("Using current directory instead.");
        Ok(std::env::current_dir().unwrap_or_default())
    }
}
