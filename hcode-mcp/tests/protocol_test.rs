//! Tests for MCP protocol types.

use hcode_mcp::*;
use serde_json::json;

#[test]
fn test_mcp_tool_serialization() {
    let tool = McpTool {
        name: "test_tool".to_string(),
        description: Some("A test tool".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "input": { "type": "string" }
            }
        }),
    };

    let json = serde_json::to_string(&tool).unwrap();
    assert!(json.contains("test_tool"));
    
    let parsed: McpTool = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, "test_tool");
}

#[test]
fn test_json_rpc_request() {
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "initialize".to_string(),
        params: Some(json!({ "test": "value" })),
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("\"jsonrpc\":\"2.0\""));
    assert!(json.contains("\"method\":\"initialize\""));
}

#[test]
fn test_initialize_params() {
    let params = InitializeParams {
        protocol_version: "2024-11-05".to_string(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "hcode".to_string(),
            version: "0.1.0".to_string(),
        },
    };

    let json = serde_json::to_string(&params).unwrap();
    assert!(json.contains("2024-11-05"));
    assert!(json.contains("hcode"));
}

#[test]
fn test_call_tool_params() {
    let params = CallToolParams {
        name: "bash".to_string(),
        arguments: Some(json!({ "command": "echo hello" })),
    };

    let json = serde_json::to_string(&params).unwrap();
    assert!(json.contains("bash"));
    assert!(json.contains("echo hello"));
}

#[test]
fn test_content_block_text() {
    let content = ContentBlock::Text {
        text: "Hello, World!".to_string(),
    };

    let json = serde_json::to_string(&content).unwrap();
    assert!(json.contains("\"type\":\"text\""));
    assert!(json.contains("Hello, World!"));
}

#[test]
fn test_resource_contents() {
    let text_content = ResourceContents::Text {
        uri: "file:///test.txt".to_string(),
        mime_type: Some("text/plain".to_string()),
        text: "content".to_string(),
    };

    let json = serde_json::to_string(&text_content).unwrap();
    assert!(json.contains("text"));
    
    let blob_content = ResourceContents::Blob {
        uri: "file:///image.png".to_string(),
        mime_type: Some("image/png".to_string()),
        blob: "base64data".to_string(),
    };

    let json = serde_json::to_string(&blob_content).unwrap();
    assert!(json.contains("blob"));
}

#[test]
fn test_server_capabilities() {
    let caps = ServerCapabilities {
        tools: Some(ToolsCapability {
            list_changed: Some(true),
        }),
        resources: Some(ResourcesCapability {
            subscribe: Some(true),
            list_changed: Some(true),
        }),
        prompts: None,
        experimental: None,
    };

    let json = serde_json::to_string(&caps).unwrap();
    assert!(json.contains("tools"));
    assert!(json.contains("resources"));
}