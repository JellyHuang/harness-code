//! WebSearchTool schema.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::LazyLock;

/// WebSearch input parameters.
#[derive(Debug, Deserialize)]
pub struct WebSearchInput {
    /// Search query.
    pub query: String,
    
    /// Search engine to use.
    #[serde(default)]
    pub engine: Option<String>,
    
    /// Maximum results.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize { 10 }

/// Search result.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    /// Result title.
    pub title: String,
    
    /// Result URL.
    pub url: String,
    
    /// Result snippet.
    pub snippet: String,
}

/// WebSearch output.
#[derive(Debug, Serialize)]
pub struct WebSearchOutput {
    /// Search results.
    pub results: Vec<SearchResult>,
    
    /// Original query.
    pub query: String,
    
    /// Total results.
    pub total: usize,
}

/// JSON schema for WebSearch tool.
pub static WEB_SEARCH_SCHEMA: LazyLock<Value> = LazyLock::new(|| json!({
    "type": "object",
    "properties": {
        "query": {
            "type": "string",
            "description": "The search query"
        },
        "engine": {
            "type": "string",
            "description": "Search engine (default: duckduckgo)",
            "enum": ["duckduckgo", "google"]
        },
        "limit": {
            "type": "number",
            "description": "Maximum number of results",
            "default": 10
        }
    },
    "required": ["query"]
}));