//! Agent command.

use crate::commands::AppContext;
use anyhow::Result;

pub async fn list(ctx: &AppContext) -> Result<()> {
    println!("Available agents:");

    // List agents from config
    for (name, agent) in &ctx.config.agents {
        let desc = agent.description.as_deref().unwrap_or("No description");
        println!("  - {} ({})", name, desc);
    }

    // List built-in agents if not overridden
    let built_in = ["coordinator", "researcher", "implementer"];
    for name in built_in {
        if !ctx.config.agents.contains_key(name) {
            println!("  - {} (built-in)", name);
        }
    }

    Ok(())
}

pub async fn run(agent_type: String, prompt: String, ctx: &AppContext) -> Result<()> {
    println!("Running {} agent with: {}", agent_type, prompt);

    // Check if agent is configured
    if let Some(agent) = ctx.config.agents.get(&agent_type) {
        if let Some(model) = &agent.model {
            println!("Using model: {}", model);
        }
    }

    Ok(())
}
