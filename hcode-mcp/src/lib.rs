//! HCode MCP - Model Context Protocol client.
//!
//! This crate provides a complete MCP (Model Context Protocol) client implementation
//! that can connect to MCP servers, discover tools/resources/prompts, and execute them.
//!
//! # Example
//!
//! ```no_run
//! use hcode_mcp::{McpClient, McpClientConfig};
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = McpClientConfig {
//!         command: "mcp-server".to_string(),
//!         args: vec![],
//!         env: HashMap::new(),
//!         client_name: "hcode".to_string(),
//!         client_version: "0.1.0".to_string(),
//!     };
//!
//!     let client = McpClient::new(config);
//!     client.connect().await?;
//!
//!     // List available tools
//!     for tool in client.tools() {
//!         println!("Tool: {}", tool.name);
//!     }
//!
//!     // Call a tool
//!     let result = client.call_tool("my_tool", None).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod protocol;

pub use client::*;
pub use protocol::*;
