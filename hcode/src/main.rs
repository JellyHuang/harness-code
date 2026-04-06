//! HCode CLI

use anyhow::Result;
use clap::Parser;

mod cli;
mod commands;

#[derive(Parser)]
#[command(name = "hcode")]
#[command(about = "AI coding agent with coordinator/worker architecture")]
struct App {
    #[command(flatten)]
    global: cli::GlobalArgs,

    #[command(subcommand)]
    command: cli::Command,
}

#[tokio::main]
async fn main() -> Result<()> {
    let app = App::parse();
    let ctx = commands::AppContext::from_args(&app.global)?;
    commands::execute(app.command, &ctx).await
}
