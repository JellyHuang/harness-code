//! MCP client implementation with stdio transport.

use crate::protocol::*;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// MCP client error.
#[derive(Debug, Error)]
pub enum McpError {
    #[error("Failed to start MCP server: {0}")]
    ServerStart(String),
    
    #[error("Failed to communicate with server: {0}")]
    Communication(String),
    
    #[error("JSON parse error: {0}")]
    JsonParse(String),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    
    #[error("Tool execution error: {0}")]
    ToolExecution(String),
    
    #[error("Server not initialized")]
    NotInitialized,
}

/// MCP client configuration.
#[derive(Debug, Clone)]
pub struct McpClientConfig {
    /// Command to start MCP server.
    pub command: String,
    /// Arguments for the command.
    pub args: Vec<String>,
    /// Environment variables.
    pub env: HashMap<String, String>,
    /// Client name for handshake.
    pub client_name: String,
    /// Client version.
    pub client_version: String,
}

impl Default for McpClientConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            env: HashMap::new(),
            client_name: "hcode".to_string(),
            client_version: "0.1.0".to_string(),
        }
    }
}

/// MCP client state.
struct McpClientState {
    /// Server process.
    server: Option<Child>,
    /// Request id counter.
    request_id: AtomicU64,
    /// Server capabilities.
    capabilities: Option<ServerCapabilities>,
    /// Available tools.
    tools: Vec<McpTool>,
    /// Available resources.
    resources: Vec<Resource>,
    /// Available prompts.
    prompts: Vec<Prompt>,
}

/// MCP client.
pub struct McpClient {
    config: McpClientConfig,
    state: Arc<Mutex<McpClientState>>,
}

impl McpClient {
    /// Create a new MCP client with config.
    pub fn new(config: McpClientConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(McpClientState {
                server: None,
                request_id: AtomicU64::new(1),
                capabilities: None,
                tools: Vec::new(),
                resources: Vec::new(),
                prompts: Vec::new(),
            })),
        }
    }

    /// Connect to MCP server (start process and handshake).
    pub async fn connect(&self) -> Result<(), McpError> {
        // Start server process
        let mut cmd = Command::new(&self.config.command);
        cmd.args(&self.config.args);
        
        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }
        
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        let mut server = cmd.spawn()
            .map_err(|e| McpError::ServerStart(format!("Failed to spawn: {}", e)))?;
        
        let stdin = server.stdin.take()
            .ok_or_else(|| McpError::ServerStart("No stdin".to_string()))?;
        let stdout = server.stdout.take()
            .ok_or_else(|| McpError::ServerStart("No stdout".to_string()))?;
        
        // Store process
        {
            let mut state = self.state.lock().unwrap();
            state.server = Some(server);
        }
        
        // Initialize handshake
        let result = self.send_request::<InitializeResult>(
            "initialize",
            InitializeParams {
                protocol_version: "2024-11-05".to_string(),
                capabilities: ClientCapabilities::default(),
                client_info: Implementation {
                    name: self.config.client_name.clone(),
                    version: self.config.client_version.clone(),
                },
            },
            stdin,
            BufReader::new(stdout),
        ).await?;
        
        // Store capabilities
        {
            let mut state = self.state.lock().unwrap();
            state.capabilities = Some(result.capabilities);
        }
        
        // Send initialized notification
        self.send_notification("notifications/initialized", None)?;
        
        // Fetch tools, resources, prompts
        self.refresh_tools()?;
        self.refresh_resources()?;
        self.refresh_prompts()?;
        
        Ok(())
    }

    /// Disconnect from MCP server.
    pub fn disconnect(&mut self) -> Result<(), McpError> {
        let mut state = self.state.lock().unwrap();
        if let Some(mut server) = state.server.take() {
            server.kill()
                .map_err(|e| McpError::ServerStart(format!("Failed to kill server: {}", e)))?;
        }
        state.capabilities = None;
        state.tools.clear();
        state.resources.clear();
        state.prompts.clear();
        Ok(())
    }

    /// Get available tools.
    pub fn tools(&self) -> Vec<McpTool> {
        self.state.lock().unwrap().tools.clone()
    }

    /// Get available resources.
    pub fn resources(&self) -> Vec<Resource> {
        self.state.lock().unwrap().resources.clone()
    }

    /// Get available prompts.
    pub fn prompts(&self) -> Vec<Prompt> {
        self.state.lock().unwrap().prompts.clone()
    }

    /// Call a tool.
    pub async fn call_tool(&self, name: &str, arguments: Option<Value>) -> Result<CallToolResult, McpError> {
        // Check tool exists
        {
            let state = self.state.lock().unwrap();
            if !state.tools.iter().any(|t| t.name == name) {
                return Err(McpError::ToolNotFound(name.to_string()));
            }
        }
        
        self.send_request::<CallToolResult>(
            "tools/call",
            CallToolParams {
                name: name.to_string(),
                arguments,
            },
        ).await
    }

    /// Read a resource.
    pub async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult, McpError> {
        self.send_request::<ReadResourceResult>(
            "resources/read",
            ReadResourceParams {
                uri: uri.to_string(),
            },
        ).await
    }

    /// Get a prompt.
    pub async fn get_prompt(&self, name: &str, arguments: Option<Value>) -> Result<GetPromptResult, McpError> {
        self.send_request::<GetPromptResult>(
            "prompts/get",
            GetPromptParams {
                name: name.to_string(),
                arguments,
            },
        ).await
    }

    /// Refresh tools list.
    fn refresh_tools(&self) -> Result<(), McpError> {
        let result = self.send_request_blocking::<ListToolsResult>("tools/list", None)?;
        let mut state = self.state.lock().unwrap();
        state.tools = result.tools;
        Ok(())
    }

    /// Refresh resources list.
    fn refresh_resources(&self) -> Result<(), McpError> {
        let result = self.send_request_blocking::<ListResourcesResult>("resources/list", None)?;
        let mut state = self.state.lock().unwrap();
        state.resources = result.resources;
        Ok(())
    }

    /// Refresh prompts list.
    fn refresh_prompts(&self) -> Result<(), McpError> {
        let result = self.send_request_blocking::<ListPromptsResult>("prompts/list", None)?;
        let mut state = self.state.lock().unwrap();
        state.prompts = result.prompts;
        Ok(())
    }

    /// Generate next request id.
    fn next_request_id(&self) -> RequestId {
        let id = self.state.lock().unwrap().request_id.fetch_add(1, Ordering::SeqCst);
        RequestId::Number(id)
    }

    /// Send a JSON-RPC request and wait for response.
    async fn send_request<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: impl Serialize,
    ) -> Result<T, McpError> {
        self.send_request_blocking(method, Some(params))
    }

    /// Send a JSON-RPC request (blocking version for simplicity).
    fn send_request_blocking<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: Option<impl Serialize>,
    ) -> Result<T, McpError> {
        let id = self.next_request_id();
        
        let request = JsonRpcRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: id.clone(),
            method: method.to_string(),
            params: params.map(|p| serde_json::to_value(p).unwrap()),
        };
        
        let request_str = serde_json::to_string(&request)
            .map_err(|e| McpError::JsonParse(e.to_string()))?;
        
        // Get stdin/stdout from process
        let (stdin, stdout) = {
            let mut state = self.state.lock().unwrap();
            let server = state.server.as_mut()
                .ok_or(McpError::NotInitialized)?;
            
            let stdin = server.stdin.as_mut()
                .ok_or_else(|| McpError::Communication("No stdin".to_string()))?;
            let stdout = server.stdout.as_mut()
                .ok_or_else(|| McpError::Communication("No stdout".to_string()))?;
            
            (stdin, BufReader::new(stdout))
        };
        
        // Send request
        stdin.write_all(request_str.as_bytes())
            .map_err(|e| McpError::Communication(e.to_string()))?;
        stdin.write_all(b"\n")
            .map_err(|e| McpError::Communication(e.to_string()))?;
        stdin.flush()
            .map_err(|e| McpError::Communication(e.to_string()))?;
        
        // Read response
        let mut response_line = String::new();
        stdout.read_line(&mut response_line)
            .map_err(|e| McpError::Communication(e.to_string()))?;
        
        // Parse response
        let response: Value = serde_json::from_str(&response_line)
            .map_err(|e| McpError::JsonParse(e.to_string()))?;
        
        // Check for error
        if let Some(error) = response.get("error") {
            let error_obj: JsonRpcError = serde_json::from_value(error.clone())
                .map_err(|e| McpError::JsonParse(e.to_string()))?;
            return Err(McpError::Protocol(error_obj.message));
        }
        
        // Extract result
        let result = response.get("result")
            .ok_or_else(|| McpError::Protocol("No result in response".to_string()))?;
        
        let parsed: T = serde_json::from_value(result.clone())
            .map_err(|e| McpError::JsonParse(e.to_string()))?;
        
        Ok(parsed)
    }

    /// Send a JSON-RPC notification (no response expected).
    fn send_notification(&self, method: &str, params: Option<Value>) -> Result<(), McpError> {
        let request = JsonRpcRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: RequestId::String("notification".to_string()),
            method: method.to_string(),
            params,
        };
        
        let request_str = serde_json::to_string(&request)
            .map_err(|e| McpError::JsonParse(e.to_string()))?;
        
        let mut state = self.state.lock().unwrap();
        let server = state.server.as_mut()
            .ok_or(McpError::NotInitialized)?;
        
        let stdin = server.stdin.as_mut()
            .ok_or_else(|| McpError::Communication("No stdin".to_string()))?;
        
        stdin.write_all(request_str.as_bytes())
            .map_err(|e| McpError::Communication(e.to_string()))?;
        stdin.write_all(b"\n")
            .map_err(|e| McpError::Communication(e.to_string()))?;
        stdin.flush()
            .map_err(|e| McpError::Communication(e.to_string()))?;
        
        Ok(())
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}

/// Adapter to use MCP tools as hcode Tool trait.
pub struct McpToolAdapter {
    client: Arc<McpClient>,
    tool: McpTool,
}

impl McpToolAdapter {
    pub fn new(client: Arc<McpClient>, tool: McpTool) -> Self {
        Self { client, tool }
    }
    
    pub fn name(&self) -> &str {
        &self.tool.name
    }
    
    pub fn description(&self) -> Option<&str> {
        self.tool.description.as_deref()
    }
    
    pub fn input_schema(&self) -> &Value {
        &self.tool.input_schema
    }
    
    /// Call the MCP tool.
    pub async fn call(&self, input: Value) -> Result<String, McpError> {
        let result = self.client.call_tool(&self.tool.name, Some(input)).await?;
        
        // Convert content blocks to string
        let content_str = result.content
            .iter()
            .map(|block| {
                match block {
                    ContentBlock::Text { text } => text.clone(),
                    ContentBlock::Image { data, mime_type } => {
                        format!("[Image: {} ({} bytes)]", mime_type, data.len())
                    }
                    ContentBlock::Resource { resource } => {
                        match &resource.contents {
                            ResourceContents::Text { text, .. } => text.clone(),
                            ResourceContents::Blob { blob, .. } => {
                                format!("[Blob: {} bytes]", blob.len())
                            }
                        }
                    }
                }
            })
            .join("\n");
        
        Ok(content_str)
    }
}