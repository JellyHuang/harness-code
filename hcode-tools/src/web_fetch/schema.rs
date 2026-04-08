//! WebFetchTool schema.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::LazyLock;
use std::collections::HashMap;

/// WebFetch input parameters.
#[derive(Debug, Deserialize)]
pub struct WebFetchInput {
    /// URL to fetch.
    pub url: String,
    
    /// HTTP method.
    #[serde(default = "default_method")]
    pub method: String,
    
    /// Request headers.
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    
    /// Request body.
    #[serde(default)]
    pub body: Option<String>,
    
    /// Timeout in milliseconds.
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    
    /// Follow redirects.
    #[serde(default = "default_follow_redirects")]
    pub follow_redirects: bool,
}

fn default_method() -> String { "GET".to_string() }
fn default_timeout() -> u64 { 30000 }
fn default_follow_redirects() -> bool { true }

/// WebFetch output.
#[derive(Debug, Serialize)]
pub struct WebFetchOutput {
    /// HTTP status code.
    pub status: u16,
    
    /// Response headers.
    pub headers: HashMap<String, String>,
    
    /// Response body.
    pub body: String,
    
    /// Final URL (after redirects).
    pub final_url: String,
    
    /// Whether the request succeeded.
    pub success: bool,
}

/// JSON schema for WebFetch tool.
pub static WEB_FETCH_SCHEMA: LazyLock<Value> = LazyLock::new(|| json!({
    "type": "object",
    "properties": {
        "url": {
            "type": "string",
            "description": "The URL to fetch"
        },
        "method": {
            "type": "string",
            "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD"],
            "default": "GET"
        },
        "headers": {
            "type": "object",
            "description": "Request headers"
        },
        "body": {
            "type": "string",
            "description": "Request body"
        },
        "timeout": {
            "type": "number",
            "description": "Timeout in milliseconds",
            "default": 30000
        },
        "follow_redirects": {
            "type": "boolean",
            "default": true
        }
    },
    "required": ["url"]
}));