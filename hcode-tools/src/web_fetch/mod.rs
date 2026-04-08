//! WebFetchTool implementation.

mod schema;

use crate::{Tool, ToolContext, ToolError};
use async_trait::async_trait;
use hcode_types::ToolResult;
use reqwest::redirect;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
pub use schema::*;

/// WebFetch tool for fetching web content.
pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "webfetch"
    }

    fn description(&self) -> &str {
        "Fetch content from a URL"
    }

    fn input_schema(&self) -> &Value {
        &WEB_FETCH_SCHEMA
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: WebFetchInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        // Build client
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(params.timeout))
            .redirect(if params.follow_redirects {
                redirect::Policy::limited(10)
            } else {
                redirect::Policy::none()
            })
            .user_agent("HCode/0.1.0")
            .build()
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        // Build request
        let method = reqwest::Method::try_from(params.method.as_str())
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;
        
        let mut request = client.request(method, &params.url);
        
        // Add headers
        if let Some(headers) = &params.headers {
            for (k, v) in headers {
                request = request.header(k, v);
            }
        }
        
        // Add body
        if let Some(body) = &params.body {
            request = request.body(body.clone());
        }
        
        // Execute request
        let response = request.send()
            .await
            .map_err(|e| ToolError::Execution(format!("Request failed: {}", e)))?;
        
        let status = response.status().as_u16();
        let final_url = response.url().to_string();
        
        // Extract headers
        let headers: HashMap<String, String> = response.headers()
            .iter()
            .filter_map(|(k, v)| {
                v.to_str().ok().map(|s| (k.to_string(), s.to_string()))
            })
            .collect();
        
        // Get body
        let body = response.text()
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to read response: {}", e)))?;
        
        // Extract text content (strip HTML tags for basic extraction)
        let content = extract_text_content(&body);
        
        Ok(ToolResult::success(
            serde_json::to_value(WebFetchOutput {
                status,
                headers,
                body: content,
                final_url,
                success: true,
            }).unwrap()
        ))
    }
}

/// Extract text content from HTML.
fn extract_text_content(html: &str) -> String {
    // Simple HTML tag stripping
    let mut result = String::new();
    let mut in_tag = false;
    
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    
    // Collapse whitespace
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}