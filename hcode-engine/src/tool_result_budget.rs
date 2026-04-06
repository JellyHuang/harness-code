//! Tool result budget management.
//!
//! Enforces per-message budget on aggregate tool result size.
//! Large results are persisted to disk and replaced with previews.

use hcode_types::{ContentBlock, Message};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Maximum tool result size in characters (default: 50KB)
pub const DEFAULT_MAX_RESULT_SIZE_CHARS: usize = 50_000;

/// Maximum tool results per message in characters (default: 150KB)
pub const MAX_TOOL_RESULTS_PER_MESSAGE_CHARS: usize = 150_000;

/// Preview size in bytes
pub const PREVIEW_SIZE_BYTES: usize = 2000;

/// Bytes per token (approximate)
pub const BYTES_PER_TOKEN: usize = 4;

/// Content replacement state for budget enforcement
#[derive(Debug, Clone)]
pub struct ContentReplacementState {
    /// Tool use IDs that have been processed
    pub seen_ids: HashSet<String>,
    /// Tool use ID -> replacement content
    pub replacements: HashMap<String, String>,
}

impl Default for ContentReplacementState {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentReplacementState {
    pub fn new() -> Self {
        Self {
            seen_ids: HashSet::new(),
            replacements: HashMap::new(),
        }
    }

    /// Check if an ID has been seen
    pub fn has_seen(&self, id: &str) -> bool {
        self.seen_ids.contains(id)
    }

    /// Mark an ID as seen
    pub fn mark_seen(&mut self, id: String) {
        self.seen_ids.insert(id);
    }

    /// Get replacement for an ID
    pub fn get_replacement(&self, id: &str) -> Option<&String> {
        self.replacements.get(id)
    }

    /// Set replacement for an ID
    pub fn set_replacement(&mut self, id: String, replacement: String) {
        self.seen_ids.insert(id.clone());
        self.replacements.insert(id, replacement);
    }
}

/// Tool result candidate for budget evaluation
#[derive(Debug, Clone)]
struct ToolResultCandidate {
    tool_use_id: String,
    content: String,
    size: usize,
}

/// Persisted tool result info
#[derive(Debug, Clone)]
pub struct PersistedToolResult {
    /// File path where result is persisted
    pub filepath: PathBuf,
    /// Original size in bytes
    pub original_size: usize,
    /// Whether content is JSON
    pub is_json: bool,
    /// Preview content
    pub preview: String,
    /// Whether there's more content
    pub has_more: bool,
}

/// Tool result budget configuration
#[derive(Debug, Clone)]
pub struct ToolResultBudgetConfig {
    /// Maximum result size per tool
    pub max_result_size_chars: usize,
    /// Maximum aggregate result size per message
    pub max_per_message_chars: usize,
    /// Preview size for persisted results
    pub preview_size_bytes: usize,
}

impl Default for ToolResultBudgetConfig {
    fn default() -> Self {
        Self {
            max_result_size_chars: DEFAULT_MAX_RESULT_SIZE_CHARS,
            max_per_message_chars: MAX_TOOL_RESULTS_PER_MESSAGE_CHARS,
            preview_size_bytes: PREVIEW_SIZE_BYTES,
        }
    }
}

/// Calculate content size
fn content_size(content: &str) -> usize {
    content.len()
}

/// Check if content is already compacted
fn is_content_already_compacted(content: &str) -> bool {
    content.starts_with("<persisted-output>")
}

/// Generate preview of content
pub fn generate_preview(content: &str, max_bytes: usize) -> (String, bool) {
    if content.len() <= max_bytes {
        return (content.to_string(), false);
    }

    // Find last newline within limit
    let truncated = &content[..max_bytes];
    let last_newline = truncated.rfind('\n');

    let cut_point = last_newline
        .filter(|&pos| pos > max_bytes / 2)
        .unwrap_or(max_bytes);

    (content[..cut_point].to_string(), true)
}

/// Build large tool result message with preview
pub fn build_large_tool_result_message(result: &PersistedToolResult) -> String {
    let original_size = format_size(result.original_size);
    let preview_size = format_size(PREVIEW_SIZE_BYTES);

    let mut message = String::new();
    message.push_str("<persisted-output>\n");
    message.push_str(&format!(
        "Output too large ({}). Full output saved to: {}\n\n",
        original_size,
        result.filepath.display()
    ));
    message.push_str(&format!("Preview (first {}):\n", preview_size));
    message.push_str(&result.preview);
    if result.has_more {
        message.push_str("\n...\n");
    } else {
        message.push('\n');
    }
    message.push_str("</persisted-output>");
    message
}

/// Format file size
fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Extract tool result candidates from messages
fn collect_candidates(messages: &[Message]) -> Vec<ToolResultCandidate> {
    let mut candidates = Vec::new();

    for message in messages {
        if let Message::User(user_msg) = message {
            for block in &user_msg.message.content {
                if let ContentBlock::ToolResult {
                    tool_use_id,
                    content,
                    ..
                } = block
                {
                    // Skip already compacted
                    if is_content_already_compacted(content) {
                        continue;
                    }

                    candidates.push(ToolResultCandidate {
                        tool_use_id: tool_use_id.clone(),
                        content: content.clone(),
                        size: content_size(content),
                    });
                }
            }
        }
    }

    candidates
}

/// Apply tool result budget to messages
///
/// For each user message whose tool_result blocks together exceed the
/// per-message limit, the largest fresh results are persisted to disk
/// and replaced with previews.
pub fn apply_tool_result_budget(
    messages: Vec<Message>,
    state: &mut ContentReplacementState,
    config: &ToolResultBudgetConfig,
    skip_tool_names: &HashSet<String>,
    tool_name_map: &HashMap<String, String>,
) -> Vec<Message> {
    let candidates = collect_candidates(&messages);

    if candidates.is_empty() {
        return messages;
    }

    // Partition by prior decision
    let mut must_reapply: Vec<(ToolResultCandidate, String)> = Vec::new();
    let mut frozen: Vec<ToolResultCandidate> = Vec::new();
    let mut fresh: Vec<ToolResultCandidate> = Vec::new();
    let mut candidate_ids: Vec<String> = Vec::new();

    for candidate in candidates {
        candidate_ids.push(candidate.tool_use_id.clone());
        if let Some(replacement) = state.get_replacement(&candidate.tool_use_id) {
            must_reapply.push((candidate, replacement.clone()));
        } else if state.has_seen(&candidate.tool_use_id) {
            frozen.push(candidate);
        } else {
            // Check if we should skip this tool
            let tool_name = tool_name_map.get(&candidate.tool_use_id);
            if let Some(name) = tool_name {
                if skip_tool_names.contains(name) {
                    state.mark_seen(candidate.tool_use_id.clone());
                    continue;
                }
            }
            fresh.push(candidate);
        }
    }

    // Calculate sizes
    let frozen_size: usize = frozen.iter().map(|c| c.size).sum();
    let fresh_size: usize = fresh.iter().map(|c| c.size).sum();

    // Select candidates to persist
    let selected = if frozen_size + fresh_size > config.max_per_message_chars {
        select_fresh_to_replace(&fresh, frozen_size, config.max_per_message_chars)
    } else {
        vec![]
    };

    // Mark non-selected as seen
    let selected_ids: HashSet<String> = selected.iter().map(|c| c.tool_use_id.clone()).collect();
    for id in candidate_ids {
        if !selected_ids.contains(&id) {
            state.mark_seen(id);
        }
    }

    // Build replacement map
    let mut replacement_map: HashMap<String, String> = HashMap::new();

    // Re-apply cached replacements
    for (candidate, replacement) in must_reapply {
        replacement_map.insert(candidate.tool_use_id.clone(), replacement);
    }

    // Create new replacements (simulated - in real impl would persist to disk)
    for candidate in &selected {
        let replacement = create_replacement(candidate, config);
        state.set_replacement(candidate.tool_use_id.clone(), replacement.clone());
        replacement_map.insert(candidate.tool_use_id.clone(), replacement);
    }

    // Apply replacements
    if replacement_map.is_empty() {
        messages
    } else {
        replace_tool_result_contents(&messages, &replacement_map)
    }
}

/// Select fresh results to replace
fn select_fresh_to_replace(
    fresh: &[ToolResultCandidate],
    frozen_size: usize,
    limit: usize,
) -> Vec<ToolResultCandidate> {
    let mut sorted = fresh.to_vec();
    sorted.sort_by(|a, b| b.size.cmp(&a.size));

    let mut selected = Vec::new();
    let fresh_total: usize = fresh.iter().map(|c| c.size).sum();
    let mut remaining = frozen_size + fresh_total;

    for candidate in sorted {
        if remaining <= limit {
            break;
        }
        selected.push(candidate.clone());
        remaining -= candidate.size;
    }

    selected
}

/// Create replacement for large tool result
fn create_replacement(candidate: &ToolResultCandidate, config: &ToolResultBudgetConfig) -> String {
    let (preview, has_more) = generate_preview(&candidate.content, config.preview_size_bytes);

    // In a real implementation, we would:
    // 1. Persist content to disk
    // 2. Create a PersistedToolResult
    // 3. Return the formatted message

    // For now, create a placeholder
    let mut message = String::new();
    message.push_str("<persisted-output>\n");
    message.push_str(&format!(
        "Output too large ({}). Tool result: {}\n\n",
        format_size(candidate.size),
        candidate.tool_use_id
    ));
    message.push_str(&format!(
        "Preview (first {}):\n",
        format_size(config.preview_size_bytes)
    ));
    message.push_str(&preview);
    if has_more {
        message.push_str("\n...\n");
    }
    message.push_str("</persisted-output>");

    message
}

/// Replace tool result contents in messages
fn replace_tool_result_contents(
    messages: &[Message],
    replacement_map: &HashMap<String, String>,
) -> Vec<Message> {
    messages
        .iter()
        .map(|message| {
            if let Message::User(user_msg) = message {
                let mut needs_replace = false;
                for block in &user_msg.message.content {
                    if let ContentBlock::ToolResult { tool_use_id, .. } = block {
                        if replacement_map.contains_key(tool_use_id) {
                            needs_replace = true;
                            break;
                        }
                    }
                }

                if !needs_replace {
                    return message.clone();
                }

                // Create new message with replaced content
                let new_content: Vec<ContentBlock> = user_msg
                    .message
                    .content
                    .iter()
                    .map(|block| {
                        if let ContentBlock::ToolResult {
                            tool_use_id,
                            content: _,
                            is_error,
                        } = block
                        {
                            if let Some(replacement) = replacement_map.get(tool_use_id) {
                                ContentBlock::ToolResult {
                                    tool_use_id: tool_use_id.clone(),
                                    content: replacement.clone(),
                                    is_error: *is_error,
                                }
                            } else {
                                block.clone()
                            }
                        } else {
                            block.clone()
                        }
                    })
                    .collect();

                Message::User(hcode_types::UserMessage {
                    uuid: user_msg.uuid.clone(),
                    timestamp: user_msg.timestamp,
                    message: hcode_types::UserMessageContent {
                        role: user_msg.message.role,
                        content: new_content,
                    },
                    is_meta: user_msg.is_meta,
                    tool_use_result: user_msg.tool_use_result.clone(),
                    image_paste_ids: user_msg.image_paste_ids.clone(),
                })
            } else {
                message.clone()
            }
        })
        .collect()
}

/// Build tool name map from messages (tool_use_id -> tool_name)
pub fn build_tool_name_map(messages: &[Message]) -> HashMap<String, String> {
    let mut map = HashMap::new();

    for message in messages {
        if let Message::Assistant(assistant_msg) = message {
            for block in &assistant_msg.message.content {
                if let ContentBlock::ToolUse { id, name, .. } = block {
                    map.insert(id.clone(), name.clone());
                }
            }
        }
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_replacement_state() {
        let mut state = ContentReplacementState::new();

        assert!(!state.has_seen("test"));

        state.mark_seen("test".to_string());
        assert!(state.has_seen("test"));

        state.set_replacement("test".to_string(), "replaced".to_string());
        assert_eq!(state.get_replacement("test"), Some(&"replaced".to_string()));
    }

    #[test]
    fn test_generate_preview() {
        let content = "line1\nline2\nline3\nline4\nline5";
        let (preview, has_more) = generate_preview(content, 10);
        assert!(has_more);
        assert!(preview.contains("line1"));
    }

    #[test]
    fn test_generate_preview_no_truncation() {
        let content = "short";
        let (preview, has_more) = generate_preview(content, 100);
        assert!(!has_more);
        assert_eq!(preview, content);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500B");
        assert_eq!(format_size(1024), "1.0KB");
        assert_eq!(format_size(1024 * 1024), "1.0MB");
    }

    #[test]
    fn test_select_fresh_to_replace() {
        let candidates = vec![
            ToolResultCandidate {
                tool_use_id: "1".to_string(),
                content: "a".repeat(100),
                size: 100,
            },
            ToolResultCandidate {
                tool_use_id: "2".to_string(),
                content: "b".repeat(200),
                size: 200,
            },
            ToolResultCandidate {
                tool_use_id: "3".to_string(),
                content: "c".repeat(300),
                size: 300,
            },
        ];

        // Total fresh: 600, limit 200, so should select largest first
        let selected = select_fresh_to_replace(&candidates, 0, 200);
        assert!(selected.len() >= 1);
        assert!(selected.iter().any(|c| c.tool_use_id == "3")); // Largest
    }
}
