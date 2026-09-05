//! MCP tool integration - bridge MCP tools to HCode Tool trait.

use crate::{ContentBlock, McpClient, McpClientConfig, McpError, McpTool, ResourceContents};
use async_trait::async_trait;
use hcode_tools::{Tool, ToolContext, ToolError, ToolResult};
use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

/// Global MCP registry.
pub struct McpToolRegistry {
    clients: HashMap<String, Arc<McpClient>>,
}

impl McpToolRegistry {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    /// Register an MCP client.
    pub fn register(&mut self, name: String, client: McpClient) {
        self.clients.insert(name, Arc::new(client));
    }

    /// Unregister an MCP client.
    pub fn unregister(&mut self, name: &str) -> Option<Arc<McpClient>> {
        self.clients.remove(name)
    }

    /// Get an MCP client by name.
    pub fn get(&self, name: &str) -> Option<Arc<McpClient>> {
        self.clients.get(name).cloned()
    }

    /// List all registered MCP clients.
    pub fn list(&self) -> Vec<String> {
        self.clients.keys().cloned().collect()
    }

    /// Get all tool names (MCP tool names are prefixed with mcp_<server>_<tool>).
    pub fn get_all_tool_names(&self) -> Vec<String> {
        let mut tool_names = Vec::new();
        for (server_name, client) in &self.clients {
            for tool in client.tools() {
                tool_names.push(format!("mcp_{}_{}", server_name, tool.name));
            }
        }
        tool_names
    }
}

impl Default for McpToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle to an MCP tool.
#[allow(dead_code)]
pub struct McpToolHandle {
    client: Arc<McpClient>,
    tool: McpTool,
}

impl McpToolHandle {
    pub fn name(&self) -> &str {
        &self.tool.name
    }

    pub fn description(&self) -> Option<&str> {
        self.tool.description.as_deref()
    }

    pub fn input_schema(&self) -> &Value {
        &self.tool.input_schema
    }

    pub async fn call(&self, args: Value) -> Result<String, McpError> {
        let result = self.client.call_tool(&self.tool.name, Some(args)).await?;

        // Convert content blocks to string
        let content_str: Vec<String> = result
            .content
            .iter()
            .map(|block| match block {
                ContentBlock::Text { text } => text.clone(),
                ContentBlock::Image { data, mime_type } => {
                    format!("[Image: {} ({} bytes)]", mime_type, data.len())
                }
                ContentBlock::Resource { resource } => match &resource.contents {
                    ResourceContents::Text { text, .. } => text.clone(),
                    ResourceContents::Blob { blob, .. } => {
                        format!("[Blob: {} bytes]", blob.len())
                    }
                },
            })
            .collect();

        Ok(content_str.join("\n"))
    }
}

/// Global MCP tool registry instance.
static GLOBAL_MCP_REGISTRY: LazyLock<RwLock<McpToolRegistry>> =
    LazyLock::new(|| RwLock::new(McpToolRegistry::new()));

/// Get the global MCP registry for reading.
pub fn get_mcp_registry() -> parking_lot::RwLockReadGuard<'static, McpToolRegistry> {
    GLOBAL_MCP_REGISTRY.read()
}

/// Get the global MCP registry for writing.
pub fn get_mcp_registry_mut() -> parking_lot::RwLockWriteGuard<'static, McpToolRegistry> {
    GLOBAL_MCP_REGISTRY.write()
}

/// MCP tool wrapper that implements the HCode Tool trait.
pub struct McpToolWrapper {
    tool_handle: McpToolHandle,
    full_name: String,
}

impl McpToolWrapper {
    pub fn new(full_name: String, tool_handle: McpToolHandle) -> Self {
        Self {
            tool_handle,
            full_name,
        }
    }
}

#[async_trait]
impl Tool for McpToolWrapper {
    fn name(&self) -> &str {
        &self.full_name
    }

    fn description(&self) -> &str {
        self.tool_handle.description().unwrap_or("MCP tool")
    }

    fn input_schema(&self) -> &Value {
        self.tool_handle.input_schema()
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _context: ToolContext) -> Result<ToolResult, ToolError> {
        let result = self
            .tool_handle
            .call(input)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolResult::success(Value::String(result)))
    }
}

/// Register an MCP server from configuration.
pub async fn register_mcp_server(
    name: String,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
) -> Result<(), String> {
    let config = McpClientConfig {
        command,
        args,
        env,
        client_name: "hcode".to_string(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let client = McpClient::new(config);

    match client.connect().await {
        Ok(_) => {
            eprintln!("Connected to MCP server: {}", name);
            let tools = client.tools();
            for tool in &tools {
                eprintln!("  - Tool: {}", tool.name);
            }

            get_mcp_registry_mut().register(name, client);
            Ok(())
        }
        Err(e) => Err(format!("Failed to connect: {}", e)),
    }
}
