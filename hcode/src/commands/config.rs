//! Config command.

use crate::commands::AppContext;
use anyhow::Result;
use hcode_config::save_config;
use std::path::PathBuf;

pub async fn show(ctx: &AppContext) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&ctx.config)?);
    if let Some(path) = &ctx.config_path {
        println!("\nConfig loaded from: {}", path.display());
    }
    Ok(())
}

pub async fn validate(_ctx: &AppContext) -> Result<()> {
    // Validation is already done during loading
    println!("Config is valid!");
    Ok(())
}

pub async fn set(_key: String, _value: String, _ctx: &AppContext) -> Result<()> {
    anyhow::bail!("Config set is not yet implemented. Please edit the config file directly.");
}

pub async fn init(force: bool, _ctx: &AppContext) -> Result<()> {
    let path = dirs::home_dir()
        .map(|h: PathBuf| h.join(".config").join("hcode").join("config.json"))
        .unwrap_or_else(|| PathBuf::from("hcode.json"));

    if path.exists() && !force {
        anyhow::bail!(
            "Config file already exists at {}. Use --force to overwrite.",
            path.display()
        );
    }

    // Create default config
    let default_config = hcode_config::Config::default();
    save_config(&default_config, &path)?;

    println!("Created config file at: {}", path.display());
    println!("Edit this file to configure your providers.");
    Ok(())
}
