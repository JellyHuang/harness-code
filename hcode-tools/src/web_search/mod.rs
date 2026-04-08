//! WebSearchTool implementation.

mod schema;

use crate::{Tool, ToolContext, ToolError};
use async_trait::async_trait;
use hcode_types::ToolResult;
use serde_json::Value;
pub use schema::*;

/// WebSearch tool for searching the web.
pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "websearch"
    }

    fn description(&self) -> &str {
        "Search the web for information"
    }

    fn input_schema(&self) -> &Value {
        &WEB_SEARCH_SCHEMA
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: WebSearchInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        // Perform search based on engine
        let results = match params.engine.as_deref() {
            Some("google") | None => search_duckduckgo(&params.query, params.limit).await?,
            Some("duckduckgo") => search_duckduckgo(&params.query, params.limit).await?,
            Some(other) => return Err(ToolError::InvalidInput(
                format!("Unknown search engine: {}", other)
            )),
        };

        let total = results.len();
        
        Ok(ToolResult::success(
            serde_json::to_value(WebSearchOutput {
                results,
                query: params.query,
                total,
            }).unwrap()
        ))
    }
}

/// Search using DuckDuckGo.
async fn search_duckduckgo(query: &str, limit: usize) -> Result<Vec<SearchResult>, ToolError> {
    // Use DuckDuckGo HTML search (no API key required)
    let url = format!("https://html.duckduckgo.com/html/?q={}", 
        urlencoding::encode(query));
    
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; HCode/0.1.0)")
        .build()
        .map_err(|e| ToolError::Execution(e.to_string()))?;
    
    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| ToolError::Execution(format!("Search failed: {}", e)))?;
    
    let html = response.text()
        .await
        .map_err(|e| ToolError::Execution(format!("Failed to read response: {}", e)))?;
    
    // Parse results from HTML (simplified parsing)
    let results = parse_ddg_results(&html, limit);
    
    Ok(results)
}

/// Parse DuckDuckGo HTML results.
fn parse_ddg_results(html: &str, limit: usize) -> Vec<SearchResult> {
    let mut results = Vec::new();
    
    // Simple regex-like parsing for DDG HTML
    // Look for result class patterns
    let lines: Vec<&str> = html.lines().collect();
    let mut i = 0;
    
    while i < lines.len() && results.len() < limit {
        let line = lines[i];
        
        // Look for result URLs
        if line.contains("result__a") || line.contains("class=\"result__title\"") {
            // Extract URL
            if let Some(url_start) = line.find("href=\"") {
                let url_start = url_start + 6;
                if let Some(url_end) = line[url_start..].find('"') {
                    let url = &line[url_start..url_start + url_end];
                    
                    // Extract title (next text content)
                    let mut title = String::new();
                    for j in i..i+5.min(lines.len()) {
                        let l = lines[j];
                        if l.contains("</a>") {
                            // Extract text between > and </a>
                            if let Some(gt) = l.rfind('>') {
                                let after_gt = &l[gt+1..];
                                if let Some(end) = after_gt.find('<') {
                                    title = after_gt[..end].to_string();
                                    break;
                                }
                            }
                        }
                    }
                    
                    if title.is_empty() {
                        title = "No title".to_string();
                    }
                    
                    // Decode HTML entities
                    let title = decode_html_entities(&title);
                    
                    results.push(SearchResult {
                        title,
                        url: url.to_string(),
                        snippet: String::new(),
                    });
                }
            }
        }
        
        i += 1;
    }
    
    results
}

/// Decode HTML entities.
fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

/// URL encoding module.
mod urlencoding {
    pub fn encode(s: &str) -> String {
        url::percent_encoding::percent_encode(s.as_bytes(), url::percent_encoding::NON_ALPHANUMERIC).to_string()
    }
}