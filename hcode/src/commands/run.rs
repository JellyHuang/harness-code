//! Run command.

use crate::commands::AppContext;
use anyhow::Result;
use futures::StreamExt;
use hcode_provider::ProviderRegistry;
use hcode_types::Message;

pub async fn execute(
    prompt: Option<String>,
    provider_override: Option<String>,
    model_override: Option<String>,
    ctx: &AppContext,
) -> Result<()> {
    // Create provider registry from config
    let registry = ProviderRegistry::from_config(&ctx.config);

    // Determine provider and model
    let provider_name = provider_override
        .or_else(|| {
            ctx.config.model.as_ref().and_then(|m| {
                let (p, _) = hcode_config::Config::parse_model_string(m);
                p.map(|s| s.to_string())
            })
        })
        .unwrap_or_else(|| {
            registry
                .get_default()
                .map(|p| p.name().to_string())
                .unwrap_or_else(|| "anthropic".to_string())
        });

    let model = model_override
        .or_else(|| {
            ctx.config.model.as_ref().and_then(|m| {
                let (p, model) = hcode_config::Config::parse_model_string(m);
                if p.is_none() {
                    Some(model.to_string())
                } else {
                    None
                }
            })
        })
        .or_else(|| {
            ctx.config
                .provider
                .get(&provider_name)
                .and_then(|p| p.models.as_ref().and_then(|m| m.keys().next().cloned()))
        })
        .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());

    // Get provider
    let provider = registry.get(&provider_name).ok_or_else(|| {
        anyhow::anyhow!("Provider '{}' not found or not configured", provider_name)
    })?;

    if let Some(p) = prompt {
        println!("Provider: {}", provider_name);
        println!("Model: {}", model);
        println!();

        // Create message from prompt
        let messages = vec![Message::user_text(&p)];

        // Make streaming API call
        println!("Sending request...");

        match provider.stream(messages, vec![]).await {
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
                                    // Skip thinking output
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
    } else {
        println!("Interactive mode");
        println!("Provider: {}", provider_name);
        println!("Model: {}", model);
        println!("\nNote: Interactive mode not yet fully implemented.");
    }
    Ok(())
}
