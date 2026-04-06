//! Query engine implementation.
//!
//! Manages query lifecycle, state, and orchestration of the query loop.
//!
//! # P0 Features Implemented
//! - Streaming Tool Executor: Tools execute during streaming (not after)
//! - Prompt-Too-Long Recovery: Automatic compaction and retry
//! - Max Output Tokens Recovery: Escalation and multi-turn recovery
//! - Model Fallback Handling: Automatic switch to fallback model
//! - Tool Result Budget: Per-message budget on tool result size

use crate::compact::{auto_compact, should_auto_compact, CompactionConfig};
use crate::error_recovery::{
    classify_api_error, handle_fallback_trigger, handle_max_output_tokens_recovery,
    handle_prompt_too_long_recovery, ApiErrorType, MaxOutputTokensRecovery,
    MAX_OUTPUT_TOKENS_RECOVERY_LIMIT,
};
use crate::state::*;
use crate::stop_hooks::{default_stop_hooks, run_stop_hooks, StopHookContext};
use crate::tool_orchestration::{
    get_max_tool_use_concurrency, partition_tool_calls, run_tools_concurrently, run_tools_serially,
    ToolExecutionResult, ToolRegistry,
};
use crate::tool_result_budget::{
    apply_tool_result_budget, build_tool_name_map, ContentReplacementState, ToolResultBudgetConfig,
};
use hcode_protocol::StreamEvent;
use hcode_types::{ContentBlock, Message, Usage};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio_stream::StreamExt as TokioStreamExt;

/// Query engine configuration
#[derive(Debug, Clone)]
pub struct QueryEngineConfig {
    /// Working directory
    pub cwd: PathBuf,

    /// Initial messages
    pub initial_messages: Vec<Message>,

    /// Custom system prompt (replaces default)
    pub custom_system_prompt: Option<String>,

    /// Append to system prompt
    pub append_system_prompt: Option<String>,

    /// User-specified model
    pub user_specified_model: Option<String>,

    /// Fallback model
    pub fallback_model: Option<String>,

    /// Max turns
    pub max_turns: Option<u32>,

    /// Max budget in USD
    pub max_budget_usd: Option<f64>,

    /// Verbose mode
    pub verbose: bool,
}

impl Default for QueryEngineConfig {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_default(),
            initial_messages: vec![],
            custom_system_prompt: None,
            append_system_prompt: None,
            user_specified_model: None,
            fallback_model: None,
            max_turns: None,
            max_budget_usd: None,
            verbose: false,
        }
    }
}

/// Query engine for running agents.
///
/// Owns query lifecycle and session state for a conversation.
/// One QueryEngine per conversation. Each submit_message() call starts a new turn.
#[allow(dead_code)]
pub struct QueryEngine {
    config: QueryEngineConfig,

    /// Mutable messages
    messages: RwLock<Vec<Message>>,

    /// Permission denials tracking
    permission_denials: RwLock<Vec<PermissionDenial>>,

    /// Total token usage
    total_usage: RwLock<Usage>,

    /// File state cache (for tracking file reads/writes)
    file_cache: RwLock<HashMap<PathBuf, FileCacheEntry>>,

    /// Discovered skill names
    discovered_skill_names: RwLock<Vec<String>>,

    /// Tool registry
    tools: Arc<dyn ToolRegistry>,

    /// LLM Provider
    provider: Arc<dyn hcode_provider::Provider>,

    /// Tool definitions for API
    tool_definitions: Vec<hcode_types::ToolDefinition>,

    /// Budget tracker
    budget_tracker: RwLock<crate::BudgetTracker>,

    /// Session ID
    session_id: String,

    /// Content replacement state for tool result budget
    content_replacement_state: RwLock<ContentReplacementState>,

    /// Current model (may change due to fallback)
    current_model: RwLock<String>,
}

/// Permission denial record
#[derive(Debug, Clone)]
pub struct PermissionDenial {
    pub tool_name: String,
    pub tool_use_id: String,
    pub tool_input: serde_json::Value,
}

/// File cache entry
#[derive(Debug, Clone)]
pub struct FileCacheEntry {
    pub content: String,
    pub timestamp: u64,
}

/// Query output from stream
#[derive(Debug, Clone)]
pub enum QueryOutput {
    /// Stream event from API
    StreamEvent(StreamEventOutput),

    /// Tool result
    ToolResult {
        tool_use_id: String,
        result: String,
        is_error: bool,
    },

    /// Progress message
    Progress { tool_use_id: String, data: String },

    /// Complete
    Complete {
        reason: TerminalReason,
        duration_ms: u64,
        total_cost_usd: f64,
        usage: Usage,
    },

    /// Error
    Error { message: String },
}

/// Stream event output
#[derive(Debug, Clone)]
pub struct StreamEventOutput {
    pub event_type: String,
    pub content: Option<String>,
    pub tool_use: Option<ToolUseBlock>,
}

impl QueryEngine {
    /// Create new query engine with config
    pub fn new(
        config: QueryEngineConfig,
        tools: Arc<dyn ToolRegistry>,
        provider: Arc<dyn hcode_provider::Provider>,
    ) -> Self {
        let initial_messages = config.initial_messages.clone();

        // Build tool definitions from registry
        let tool_definitions: Vec<hcode_types::ToolDefinition> = tools
            .list()
            .iter()
            .map(|t| {
                hcode_types::ToolDefinition::new(
                    t.name(),
                    t.description(),
                    t.input_schema().clone(),
                )
            })
            .collect();

        let budget_tracker = crate::BudgetTracker::new(config.max_turns, config.max_budget_usd);
        let session_id = uuid::Uuid::new_v4().to_string();
        let current_model = config
            .user_specified_model
            .clone()
            .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());

        Self {
            config,
            messages: RwLock::new(initial_messages),
            permission_denials: RwLock::new(vec![]),
            total_usage: RwLock::new(Usage::default()),
            file_cache: RwLock::new(HashMap::new()),
            discovered_skill_names: RwLock::new(vec![]),
            tools,
            provider,
            tool_definitions,
            budget_tracker: RwLock::new(budget_tracker),
            session_id,
            content_replacement_state: RwLock::new(ContentReplacementState::new()),
            current_model: RwLock::new(current_model),
        }
    }

    /// Get all messages
    pub fn get_messages(&self) -> Vec<Message> {
        self.messages.read().clone()
    }

    /// Get current model
    pub fn get_current_model(&self) -> String {
        self.current_model.read().clone()
    }

    /// Switch to fallback model
    pub fn switch_to_fallback_model(&self) -> bool {
        if let Some(fallback) = &self.config.fallback_model {
            *self.current_model.write() = fallback.clone();
            true
        } else {
            false
        }
    }

    /// Add a message to the conversation history.
    /// Used by interactive mode to accumulate messages across turns.
    pub fn add_message(&self, message: Message) {
        self.messages.write().push(message);
    }

    /// Clear all messages from conversation history.
    /// Used by interactive mode's /clear command.
    pub fn clear_messages(&self) {
        self.messages.write().clear();
    }

    /// Set messages directly (for session restoration).
    pub fn set_messages(&self, messages: Vec<Message>) {
        *self.messages.write() = messages;
    }

    /// Submit a message and stream results
    #[allow(unused_assignments)]
    pub fn submit_message(
        &self,
        prompt: String,
    ) -> impl futures::Stream<Item = QueryOutput> + Send + '_ {
        // Extract config values upfront to avoid holding locks across await
        let cwd = self.config.cwd.clone();
        let session_id = self.session_id.clone();
        let max_turns = self.config.max_turns;
        let max_budget_usd = self.config.max_budget_usd;
        let tool_definitions = self.tool_definitions.clone();

        // Clone Arc references
        let tools = self.tools.clone();
        let provider = self.provider.clone();

        // Get initial messages
        let initial_messages: Vec<Message> = self.messages.read().clone();
        let budget_tracker = self.budget_tracker.read().clone();
        let total_usage = self.total_usage.read().clone();

        async_stream::stream! {
                    let start_time = Instant::now();
                    let mut budget_tracker = budget_tracker;
                    let mut total_usage = total_usage;
                    let mut messages = initial_messages;

                    // Create user message and add to conversation
                    let user_message = Message::user_text(prompt);
                    messages.push(user_message.clone());

                    // Initialize loop state
                    let mut loop_state = LoopState::new(messages.clone());
                    let mut query_state: QueryState = QueryState::Initial;
                    let mut assistant_messages: Vec<Message> = Vec::new();
                    let mut turn_usage = Usage::default();

                    // Main state machine loop
                    loop {
                        // Check cancellation
                        if loop_state.is_cancelled() {
                            yield QueryOutput::Complete {
                                reason: TerminalReason::Cancelled,
                                duration_ms: start_time.elapsed().as_millis() as u64,
                                total_cost_usd: budget_tracker.current_cost_usd,
                                usage: total_usage.clone(),
                            };
                            return;
                        }

                        // Check budget limits
                        if budget_tracker.is_max_turns_reached() {
                            yield QueryOutput::Complete {
                                reason: TerminalReason::MaxTurns {
                                    turn_count: budget_tracker.current_turn,
                                    max_turns: budget_tracker.max_turns.unwrap_or(0),
                                },
                                duration_ms: start_time.elapsed().as_millis() as u64,
                                total_cost_usd: budget_tracker.current_cost_usd,
                                usage: total_usage.clone(),
                            };
                            return;
                        }
                        if budget_tracker.is_budget_exceeded() {
                            yield QueryOutput::Complete {
                                reason: TerminalReason::BudgetExceeded {
                                    cost_usd: budget_tracker.current_cost_usd,
                                    max_budget_usd: budget_tracker.max_budget_usd.unwrap_or(0.0),
                                },
                                duration_ms: start_time.elapsed().as_millis() as u64,
                                total_cost_usd: budget_tracker.current_cost_usd,
                                usage: total_usage.clone(),
                            };
                            return;
                        }

                        // Handle each state
                        match query_state {
                            QueryState::Initial => {
                                // Transition to streaming API
                                budget_tracker.increment_turn();
                                query_state = QueryState::StreamingApi {
                                    turn: loop_state.turn_count,
                                    usage: Usage::default(),
                                };
                            }

                            QueryState::StreamingApi { turn: _, usage: _ } => {
                                // Prepare messages for API (filter non-API messages)
                                let mut messages_for_api: Vec<Message> = loop_state.messages
                                    .iter()
                                    .filter(|m| matches!(m, Message::User(_) | Message::Assistant(_)))
                                    .cloned()
                                    .collect();

                                // Apply tool result budget (P0 feature)
                                let tool_name_map = build_tool_name_map(&messages_for_api);
                                let skip_tool_names: HashSet<String> = HashSet::new();
                                let budget_config = ToolResultBudgetConfig::default();
                                let mut content_replacement_state = self.content_replacement_state.read().clone();
                                messages_for_api = apply_tool_result_budget(
                                    messages_for_api,
                                    &mut content_replacement_state,
                                    &budget_config,
                                    &skip_tool_names,
                                    &tool_name_map,
                                );
                                // Persist the updated state
                                *self.content_replacement_state.write() = content_replacement_state;

                                // Call provider stream
                                let stream_result = provider.stream(
                                    messages_for_api,
                                    tool_definitions.clone(),
                                ).await;

                                match stream_result {
                                    Ok(mut stream) => {
                                        let mut content_blocks: Vec<ContentBlock> = Vec::new();
                                        let mut current_text = String::new();
                                        let mut current_thinking = String::new();
                                        let mut tool_use_blocks: Vec<ToolUseBlock> = Vec::new();
                                        let mut message_id = String::new();
                                        let mut model = String::new();
                                        let mut stop_reason: Option<String> = None;
                                        let mut stream_usage = Usage::default();
                                        let mut current_tool_input_json = String::new();
                                        let mut current_tool_id = String::new();
                                        let mut current_tool_name = String::new();

                                        // Process SSE events
                                        while let Some(event) = TokioStreamExt::next(&mut stream).await {
                                            match event {
                                                StreamEvent::MessageStart { id, model: m } => {
                                                    message_id = id;
                                                    model = m;
                                                    yield QueryOutput::StreamEvent(StreamEventOutput {
                                                        event_type: "message_start".to_string(),
                                                        content: None,
                                                        tool_use: None,
                                                    });
                                                }
                                                StreamEvent::ContentBlockStart { index, block_type } => {
                                                    match block_type {
                                                        hcode_protocol::ContentBlockType::Text => {
                                                            current_text = String::new();
                                                        }
                                                        hcode_protocol::ContentBlockType::Thinking => {
                                                            current_thinking = String::new();
                                                        }
                                                        hcode_protocol::ContentBlockType::ToolUse => {
                                                            current_tool_input_json = String::new();
                                                            current_tool_id = String::new();
                                                            current_tool_name = String::new();
                                                        }
                                                    }
                                                    yield QueryOutput::StreamEvent(StreamEventOutput {
                                                        event_type: "content_block_start".to_string(),
                                                        content: Some(format!("index={}, type={}", index, match block_type {
                                                            hcode_protocol::ContentBlockType::Text => "text",
                                                            hcode_protocol::ContentBlockType::Thinking => "thinking",
                                                            hcode_protocol::ContentBlockType::ToolUse => "tool_use",
                                                        })),
                                                        tool_use: None,
                                                    });
                                                }
                                                StreamEvent::ContentBlockDelta { index: _, delta } => {
                                                    match delta {
                                                        hcode_protocol::ContentDelta::Text { text } => {
                                                            current_text.push_str(&text);
                                                            yield QueryOutput::StreamEvent(StreamEventOutput {
                                                                event_type: "content_block_delta".to_string(),
                                                                content: Some(text),
                                                                tool_use: None,
                                                            });
                                                        }
                                                        hcode_protocol::ContentDelta::Thinking { thinking } => {
                                                            current_thinking.push_str(&thinking);
                                                            yield QueryOutput::StreamEvent(StreamEventOutput {
                                                                event_type: "thinking_delta".to_string(),
                                                                content: Some(thinking),
                                                                tool_use: None,
                                                            });
                                                        }
                                                        hcode_protocol::ContentDelta::InputJsonDelta { partial_json } => {
                                                            current_tool_input_json.push_str(&partial_json);
                                                        }
                                                    }
                                                }
                                                StreamEvent::ContentBlockStop { index } => {
                                                    // Finalize the content block
                                                    if !current_text.is_empty() {
                                                        content_blocks.push(ContentBlock::Text { text: current_text.clone() });
                                                        current_text = String::new();
                                                    }
                                                    if !current_thinking.is_empty() {
                                                        content_blocks.push(ContentBlock::Thinking { thinking: current_thinking.clone() });
                                                        current_thinking = String::new();
                                                    }
                                                    if !current_tool_input_json.is_empty() {
                                                        let input: serde_json::Value = serde_json::from_str(&current_tool_input_json)
                                                            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                                                        tool_use_blocks.push(ToolUseBlock {
                                                            id: current_tool_id.clone(),
                                                            name: current_tool_name.clone(),
                                                            input,
                                                        });
                                                        yield QueryOutput::StreamEvent(StreamEventOutput {
                                                            event_type: "content_block_stop".to_string(),
                                                            content: Some(format!("index={}", index)),
                                                            tool_use: Some(ToolUseBlock {
                                                                id: current_tool_id.clone(),
                                                                name: current_tool_name.clone(),
                                                                input: serde_json::Value::Null,
                                                            }),
                                                        });
                                                        current_tool_input_json = String::new();
                                                    } else {
                                                        yield QueryOutput::StreamEvent(StreamEventOutput {
                                                            event_type: "content_block_stop".to_string(),
                                                            content: Some(format!("index={}", index)),
                                                            tool_use: None,
                                                        });
                                                    }
                                                }
                                                StreamEvent::MessageDelta { stop_reason: sr, usage: u } => {
                                                    stop_reason = sr;
                                                    stream_usage.input_tokens = u.input_tokens as u64;
                                                    stream_usage.output_tokens = u.output_tokens as u64;
                                                    stream_usage.cache_creation_input_tokens = u.cache_read_tokens.unwrap_or(0) as u64;
                                                    stream_usage.cache_read_input_tokens = u.cache_write_tokens.unwrap_or(0) as u64;

                                                    yield QueryOutput::StreamEvent(StreamEventOutput {
                                                        event_type: "message_delta".to_string(),
                                                        content: stop_reason.clone(),
                                                        tool_use: None,
                                                    });
                                                }
                                                StreamEvent::MessageStop => {
                                                    yield QueryOutput::StreamEvent(StreamEventOutput {
                                                        event_type: "message_stop".to_string(),
                                                        content: None,
                                                        tool_use: None,
                                                    });
                                                    break;
                                                }
        StreamEvent::Error { message } => {
                                                    let error_msg = message.clone();
                                                    let error_type = classify_api_error(&error_msg);

                                                    // Handle different error types with recovery
                                                    match error_type {
                                                        ApiErrorType::PromptTooLong { current_tokens, max_tokens } => {
                                                            // Attempt reactive compact recovery
                                                            let compact_config = CompactionConfig::default();
                                                            if let Some(action) = handle_prompt_too_long_recovery(
                                                                &mut loop_state,
                                                                current_tokens,
                                                                max_tokens,
                                                                &compact_config,
                                                            ).await {
                                                                match action {
                                                                    crate::error_recovery::RecoveryAction::Continue { messages, .. } => {
                                                                        loop_state.messages = messages;
                                                                        query_state = QueryState::StreamingApi {
                                                                            turn: loop_state.turn_count,
                                                                            usage: Usage::default(),
                                                                        };
                                                                        continue;
                                                                    }
                                                                    crate::error_recovery::RecoveryAction::Abort { error } => {
                                                                        yield QueryOutput::Error { message: error.clone() };
                                                                        query_state = QueryState::Terminal {
                                                                            reason: TerminalReason::Error { message: error },
                                                                        };
                                                                    }
                                                                    _ => {}
                                                                }
                                                            }
                                                            continue;
                                                        }
                                                        ApiErrorType::MaxOutputTokens => {
                                                            // Attempt max_output_tokens recovery
                                                            if let Some(recovery) = handle_max_output_tokens_recovery(
                                                                &mut loop_state,
                                                                None, // current_override
                                                                MAX_OUTPUT_TOKENS_RECOVERY_LIMIT,
                                                            ).await {
                                                                match recovery {
                                                                    MaxOutputTokensRecovery::Escalate { new_limit } => {
                                                                        // Retry with escalated limit
                                                                        tracing::info!("Escalating max_output_tokens to {}", new_limit);
                                                                        query_state = QueryState::StreamingApi {
                                                                            turn: loop_state.turn_count,
                                                                            usage: Usage::default(),
                                                                        };
                                                                        continue;
                                                                    }
                                                                    MaxOutputTokensRecovery::RetryWithMessage { attempt } => {
                                                                        // Inject recovery message and retry
                                                                        let recovery_msg = Message::user_text(
                                                                            crate::error_recovery::create_max_tokens_recovery_message()
                                                                        );
                                                                        loop_state.messages.push(recovery_msg);
                                                                        tracing::info!("Max output tokens recovery attempt {}", attempt);
                                                                        query_state = QueryState::StreamingApi {
                                                                            turn: loop_state.turn_count,
                                                                            usage: Usage::default(),
                                                                        };
                                                                        continue;
                                                                    }
                                                                }
                                                            }
                                                            yield QueryOutput::Error { message: error_msg.clone() };
                                                            query_state = QueryState::Terminal {
                                                                reason: TerminalReason::Error { message: error_msg },
                                                            };
                                                            continue;
                                                        }
                                                        ApiErrorType::ModelOverloaded => {
                                                            // Attempt fallback model
                                                            let fallback_model = self.config.fallback_model.clone();
                                                            if let Some(action) = handle_fallback_trigger(
                                                                &self.current_model.read().clone(),
                                                                fallback_model.as_deref(),
                                                            ) {
                                                                if let crate::error_recovery::RecoveryAction::RetryWithFallback { fallback_model, .. } = action {
                                                                    tracing::info!("Switching to fallback model: {}", fallback_model);
                                                                    *self.current_model.write() = fallback_model.clone();
                                                                    // Retry with fallback model
                                                                    query_state = QueryState::StreamingApi {
                                                                        turn: loop_state.turn_count,
                                                                        usage: Usage::default(),
                                                                    };
                                                                    continue;
                                                                }
                                                            }
                                                            yield QueryOutput::Error { message: error_msg.clone() };
                                                            query_state = QueryState::Terminal {
                                                                reason: TerminalReason::Error { message: error_msg },
                                                            };
                                                            continue;
                                                        }
                                                        ApiErrorType::RateLimit { retry_after_ms } => {
                                                            // Wait and retry
                                                            if let Some(delay_ms) = retry_after_ms {
                                                                tracing::info!("Rate limited, waiting {}ms", delay_ms);
                                                                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                                                            }
                                                            query_state = QueryState::StreamingApi {
                                                                turn: loop_state.turn_count,
                                                                usage: Usage::default(),
                                                            };
                                                            continue;
                                                        }
                                                        ApiErrorType::AuthenticationError | ApiErrorType::Generic { .. } => {
                                                            yield QueryOutput::Error { message: error_msg.clone() };
                                                            query_state = QueryState::Terminal {
                                                                reason: TerminalReason::Error { message: error_msg },
                                                            };
                                                            continue;
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        // Update usage tracking
                                        turn_usage = stream_usage.clone();
                                        total_usage.input_tokens += stream_usage.input_tokens;
                                        total_usage.output_tokens += stream_usage.output_tokens;
                                        total_usage.cache_creation_input_tokens += stream_usage.cache_creation_input_tokens;
                                        total_usage.cache_read_input_tokens += stream_usage.cache_read_input_tokens;

                                        // Add tool_use blocks to content
                                        for tb in &tool_use_blocks {
                                            content_blocks.push(ContentBlock::ToolUse {
                                                id: tb.id.clone(),
                                                name: tb.name.clone(),
                                                input: tb.input.clone(),
                                            });
                                        }

                                        // Create assistant message
                                        let assistant_message = create_assistant_message(
                                            message_id,
                                            model,
                                            content_blocks.clone(),
                                            stream_usage.clone(),
                                            stop_reason.clone(),
                                        );
                                        assistant_messages.push(assistant_message.clone());
                                        loop_state.messages.push(assistant_message);

                                        // Decide next state based on stop_reason and tool_use blocks
                                        let has_tool_calls = tool_use_blocks.len() > 0;
                                        let stop_is_tool_use = stop_reason.as_deref() == Some("tool_use");

                                        if has_tool_calls || stop_is_tool_use {
                                            // Transition to ToolExecution
                                            query_state = QueryState::ToolExecution {
                                                pending_blocks: tool_use_blocks,
                                                concurrent: true,
                                            };
                                        } else {
                                            // Transition to StopHooks
                                            query_state = QueryState::StopHooks {
                                                assistant_messages: assistant_messages.clone(),
                                            };
                                        }
                                    }
                                    Err(e) => {
                                        yield QueryOutput::Error { message: e.to_string() };
                                        query_state = QueryState::Terminal {
                                            reason: TerminalReason::Error { message: e.to_string() },
                                        };
                                    }
                                }
                            }

                            QueryState::ToolExecution { pending_blocks, concurrent: _ } => {
                                if pending_blocks.is_empty() {
                                    // No tools to execute, go to StopHooks
                                    query_state = QueryState::StopHooks {
                                        assistant_messages: assistant_messages.clone(),
                                    };
                                    continue;
                                }

                                // Partition tool calls into batches
                                let batches = partition_tool_calls(pending_blocks.clone(), &tools);

                                // Execute each batch
                                for batch in batches {
                                    let tool_results: Vec<ToolExecutionResult> = if batch.is_concurrent {
                                        // Await the future to get the stream
                                        let result_stream = run_tools_concurrently(
                                            batch.blocks.clone(),
                                            tools.clone(),
                                            hcode_tools::ToolContext::new(
                                                cwd.clone(),
                                                session_id.clone(),
                                                batch.blocks.first().map(|b| b.id.clone()).unwrap_or_default(),
                                            ),
                                            get_max_tool_use_concurrency(),
                                        ).await;
                                        // Collect from the stream using futures::StreamExt
                                        futures::StreamExt::collect::<Vec<_>>(result_stream).await
                                    } else {
                                        // Await the future to get the stream
                                        let result_stream = run_tools_serially(
                                            batch.blocks.clone(),
                                            tools.clone(),
                                            hcode_tools::ToolContext::new(
                                                cwd.clone(),
                                                session_id.clone(),
                                                batch.blocks.first().map(|b| b.id.clone()).unwrap_or_default(),
                                            ),
                                        ).await;
                                        // Collect from the stream using futures::StreamExt
                                        futures::StreamExt::collect::<Vec<_>>(result_stream).await
                                    };

                                    // Yield tool results and add to messages
                                    let mut tool_result_content: Vec<ContentBlock> = Vec::new();
                                    for result in tool_results {
                                        yield QueryOutput::ToolResult {
                                            tool_use_id: result.tool_use_id.clone(),
                                            result: match &result.result {
                                                Ok(r) => r.content.clone(),
                                                Err(e) => e.to_string(),
                                            },
                                            is_error: result.result.is_err(),
                                        };

                                        // Collect content blocks for user message
                                        tool_result_content.extend(result.content_blocks);
                                    }

                                    // Create user message with tool results
                                    if !tool_result_content.is_empty() {
                                        let tool_result_message = create_user_message_with_tool_results(tool_result_content);
                                        loop_state.messages.push(tool_result_message.clone());
                                        assistant_messages.clear(); // Reset for next turn
                                    }
                                }

                                // Transition back to StreamingApi for next turn
                                loop_state.turn_count += 1;
                                query_state = QueryState::StreamingApi {
                                    turn: loop_state.turn_count,
                                    usage: turn_usage.clone(),
                                };
                            }

                            QueryState::StopHooks { assistant_messages: am } => {
                                // Run stop hooks
                                let hook_context = StopHookContext {
                                    turn: loop_state.turn_count,
                                    max_turns,
                                    current_cost_usd: budget_tracker.current_cost_usd,
                                    max_budget_usd,
                                };
                                let hooks = default_stop_hooks();
                                let hook_result = run_stop_hooks(&loop_state.messages, &am, &hook_context, &hooks).await;

                                if hook_result.prevent_continuation {
                                    // Hooks blocked continuation
                                    let reason = if let Some(max) = max_turns {
                                        if loop_state.turn_count >= max {
                                            TerminalReason::MaxTurns {
                                                turn_count: loop_state.turn_count,
                                                max_turns: max,
                                            }
                                        } else if let Some(max_budget) = max_budget_usd {
                                            TerminalReason::BudgetExceeded {
                                                cost_usd: budget_tracker.current_cost_usd,
                                                max_budget_usd: max_budget,
                                            }
                                        } else {
                                            TerminalReason::StopHookPrevented
                                        }
                                    } else {
                                        TerminalReason::StopHookPrevented
                                    };

                                    query_state = QueryState::Terminal { reason };
                                } else {
                                    // Check for auto-compact
                                    let current_tokens = estimate_tokens(&loop_state.messages);
                                    let compact_config = CompactionConfig::default();

                                    if should_auto_compact(&loop_state.messages, current_tokens, &compact_config, None) {
                                        query_state = QueryState::Compaction {
                                            trigger: crate::state::CompactionTrigger::Auto {
                                                token_threshold: compact_config.auto_compact_threshold,
                                            },
                                        };
                                    } else {
                                        // Normal completion
                                        query_state = QueryState::Terminal {
                                            reason: TerminalReason::Completed,
                                        };
                                    }
                                }
                            }

                            QueryState::Compaction { trigger } => {
                                let compact_config = CompactionConfig::default();

                                let compact_result = match trigger {
                                    crate::state::CompactionTrigger::Auto { token_threshold: _ } => {
                                        auto_compact(loop_state.messages.clone(), &compact_config).await
                                    }
                                    crate::state::CompactionTrigger::Manual => {
                                        auto_compact(loop_state.messages.clone(), &compact_config).await
                                    }
                                    crate::state::CompactionTrigger::Reactive { error } => {
                                        // Reactive compaction for PTL recovery
                                        tracing::info!("Reactive compact triggered by error: {}", error);
                                        use crate::compact::reactive::{reactive_compact, ReactiveTrigger};
                                        reactive_compact(
                                            loop_state.messages.clone(),
                                            ReactiveTrigger::PromptTooLong {
                                                current_tokens: 0,
                                                max_tokens: 0,
                                            },
                                            &compact_config,
                                        ).await.map(|r| crate::compact::CompactionResult {
                                            messages: r.messages,
                                            tokens_saved: r.tokens_saved,
                                            compacted: r.compacted,
                                        })
                                    }
                                };

                                match compact_result {
                                    Ok(result) => {
                                        if result.compacted {
                                            loop_state.messages = result.messages;

                                            // Reset tracking after compact
                                            loop_state.turn_count = 1;
                                            loop_state.has_attempted_reactive_compact = false;

                                            // After compaction, continue the query (don't terminate)
                                            query_state = QueryState::StreamingApi {
                                                turn: loop_state.turn_count,
                                                usage: Usage::default(),
                                            };
                                        } else {
                                            // No compaction needed, complete normally
                                            query_state = QueryState::Terminal {
                                                reason: TerminalReason::Completed,
                                            };
                                        }
                                    }
                                    Err(e) => {
                                        yield QueryOutput::Error { message: e.to_string() };
                                        query_state = QueryState::Terminal {
                                            reason: TerminalReason::Error { message: e.to_string() },
                                        };
                                    }
                                }
                            }

                            QueryState::Terminal { reason } => {
                                // Final output
                                yield QueryOutput::Complete {
                                    reason,
                                    duration_ms: start_time.elapsed().as_millis() as u64,
                                    total_cost_usd: budget_tracker.current_cost_usd,
                                    usage: total_usage.clone(),
                                };
                                return;
                            }
                        }
                    }
                }
    }
}

/// Create an assistant message
fn create_assistant_message(
    id: String,
    model: String,
    content: Vec<ContentBlock>,
    usage: Usage,
    stop_reason: Option<String>,
) -> Message {
    use chrono::Utc;
    use hcode_types::{AssistantMessage, AssistantMessageContent, Role};

    Message::Assistant(AssistantMessage {
        uuid: uuid::Uuid::new_v4().to_string(),
        timestamp: Utc::now(),
        message: AssistantMessageContent {
            id,
            role: Role::Assistant,
            model,
            content,
            stop_reason,
            stop_sequence: None,
            usage: Some(usage),
        },
        is_api_error_message: None,
        api_error: None,
        request_id: None,
    })
}

/// Create a user message with tool results
fn create_user_message_with_tool_results(content: Vec<ContentBlock>) -> Message {
    use chrono::Utc;
    use hcode_types::{Role, UserMessage, UserMessageContent};

    Message::User(UserMessage {
        uuid: uuid::Uuid::new_v4().to_string(),
        timestamp: Utc::now(),
        message: UserMessageContent {
            role: Role::User,
            content,
        },
        is_meta: false,
        tool_use_result: None,
        image_paste_ids: None,
    })
}

/// Estimate tokens for messages (simplified)
fn estimate_tokens(messages: &[Message]) -> u64 {
    let mut tokens = 0u64;
    for msg in messages {
        match msg {
            Message::User(m) => {
                for block in &m.message.content {
                    if let ContentBlock::Text { text } = block {
                        tokens += (text.len() / 4) as u64;
                    }
                }
            }
            Message::Assistant(m) => {
                for block in &m.message.content {
                    if let ContentBlock::Text { text } = block {
                        tokens += (text.len() / 4) as u64;
                    }
                }
            }
            _ => {}
        }
    }
    tokens
}

/// Simple tool registry implementation
pub struct SimpleToolRegistry {
    tools: HashMap<String, Arc<dyn hcode_tools::Tool>>,
}

impl SimpleToolRegistry {
    pub fn new(tools: Vec<Arc<dyn hcode_tools::Tool>>) -> Self {
        let map = tools
            .into_iter()
            .map(|t| (t.name().to_string(), t))
            .collect();
        Self { tools: map }
    }
}

#[async_trait::async_trait]
impl ToolRegistry for SimpleToolRegistry {
    fn get(&self, name: &str) -> Option<Arc<dyn hcode_tools::Tool>> {
        self.tools.get(name).cloned()
    }

    fn list(&self) -> Vec<Arc<dyn hcode_tools::Tool>> {
        self.tools.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use futures::Stream;
    use hcode_provider::{CompletionResponse, Provider, ProviderError};
    use hcode_tools::{Tool, ToolError, ToolResult};
    use serde_json::Value;
    use std::pin::Pin;

    /// Empty tool registry for testing
    struct EmptyToolRegistry;

    #[async_trait::async_trait]
    impl ToolRegistry for EmptyToolRegistry {
        fn get(&self, _name: &str) -> Option<Arc<dyn Tool>> {
            None
        }
        fn list(&self) -> Vec<Arc<dyn Tool>> {
            vec![]
        }
    }

    /// Empty provider for testing
    struct EmptyProvider;

    #[async_trait::async_trait]
    impl Provider for EmptyProvider {
        fn name(&self) -> &str {
            "empty"
        }
        fn model(&self) -> &str {
            "empty-model"
        }

        async fn stream(
            &self,
            _messages: Vec<Message>,
            _tools: Vec<hcode_types::ToolDefinition>,
        ) -> Result<Pin<Box<dyn Stream<Item = StreamEvent> + Send>>, ProviderError> {
            // Return an error stream
            Err(ProviderError::Api(
                "Empty provider - no API configured".to_string(),
            ))
        }

        async fn complete(
            &self,
            _messages: Vec<Message>,
            _tools: Vec<hcode_types::ToolDefinition>,
        ) -> Result<CompletionResponse, ProviderError> {
            Err(ProviderError::Api("Empty provider".to_string()))
        }
    }

    #[test]
    fn test_engine_creation() {
        let engine = QueryEngine::new(
            QueryEngineConfig::default(),
            Arc::new(EmptyToolRegistry),
            Arc::new(EmptyProvider),
        );
        assert!(engine.get_messages().is_empty());
    }

    #[tokio::test]
    async fn test_submit_message() {
        use futures::StreamExt;

        let engine = QueryEngine::new(
            QueryEngineConfig::default(),
            Arc::new(EmptyToolRegistry),
            Arc::new(EmptyProvider),
        );
        let stream = engine.submit_message("Hello".to_string());

        // Pin the stream
        futures::pin_mut!(stream);

        let mut count = 0;
        while let Some(output) = futures::StreamExt::next(&mut stream).await {
            count += 1;
            match output {
                QueryOutput::Complete { reason, .. } => {
                    assert!(matches!(reason, TerminalReason::Error { .. }));
                }
                QueryOutput::Error { .. } => {
                    // Expected since EmptyProvider returns error
                }
                _ => {}
            }
        }

        assert!(count > 0);
    }

    #[test]
    fn test_query_engine_config() {
        let config = QueryEngineConfig {
            cwd: PathBuf::from("/test"),
            initial_messages: vec![Message::user_text("test")],
            custom_system_prompt: Some("custom".to_string()),
            append_system_prompt: Some("append".to_string()),
            user_specified_model: Some("model".to_string()),
            fallback_model: Some("fallback".to_string()),
            max_turns: Some(5),
            max_budget_usd: Some(10.0),
            verbose: true,
        };

        assert_eq!(config.cwd, PathBuf::from("/test"));
        assert_eq!(config.max_turns, Some(5));
        assert_eq!(config.max_budget_usd, Some(10.0));
    }
}
