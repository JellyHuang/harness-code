# Query Engine Implementation Plan

## Status: Architecture Complete, Core Logic Missing

**Current Completion:** ~5% (types and structure only)  
**Target:** 100% functional migration  
**Estimated Effort:** 8-12 days for full implementation

---

## What's Implemented ✅

### Phase 0: Architecture (COMPLETE)

1. **Type System** (`hcode-types/src/message.rs`):
   - 15+ message variants (User, Assistant, System, Progress, Attachment, Tombstone, StreamEvent, ToolUseSummary)
   - ContentBlock with Text, Thinking, ToolUse, ToolResult, Image
   - Full serialization support
   - **Lines:** ~350

2. **State Machine** (`hcode-engine/src/state.rs`):
   - QueryState enum with 6 states
   - LoopState for mutable state tracking
   - ContinueReason for transitions
   - CancellationToken integration
   - **Lines:** ~200

3. **Tool Orchestration** (`hcode-engine/src/tool_orchestration.rs`):
   - partition_tool_calls() for concurrent/serial batching
   - run_tools_concurrently() with buffer_unordered
   - run_tools_serially() for non-safe tools
   - **Lines:** ~180

4. **Compaction Module** (`hcode-engine/src/compact/`):
   - auto.rs: Auto-compact based on token threshold
   - micro.rs: Micro-compact within turns
   - reactive.rs: Reactive compact for errors
   - **Lines:** ~250

5. **Stop Hooks** (`hcode-engine/src/stop_hooks.rs`):
   - StopHook trait
   - MaxTurnsHook, BudgetHook implementations
   - run_stop_hooks() coordinator
   - **Lines:** ~150

6. **Error Recovery** (`hcode-engine/src/error_recovery.rs`):
   - handle_max_output_tokens_recovery()
   - handle_prompt_too_long_recovery()
   - handle_rate_limit_recovery()
   - **Lines:** ~120

7. **Budget Tracking** (`hcode-engine/src/budget.rs`):
   - BudgetTracker with turns, costs, tokens
   - **Lines:** ~120

**Total Architecture:** ~1,370 lines of well-structured Rust

---

## What's Missing ❌

### Phase 1: Core Query Loop (CRITICAL)

**File:** `hcode-engine/src/query_engine.rs`

**Task 1.1: Implement submit_message() async generator**
- **Estimated Lines:** ~150
- **Priority:** HIGHEST
- **Dependencies:** None

```rust
impl QueryEngine {
    pub fn submit_message(
        &self,
        prompt: String,
    ) -> impl futures::Stream<Item = QueryOutput> + Send + '_ {
        use async_stream::stream;
        
        stream! {
            let mut state = LoopState::new(self.messages.read().clone());
            let mut query_state = QueryState::Initial;
            
            loop {
                if state.is_cancelled() {
                    yield QueryOutput::Complete {
                        reason: TerminalReason::Cancelled,
                        // ... fields
                    };
                    break;
                }
                
                query_state = match query_state {
                    QueryState::Initial => /* ... */,
                    QueryState::StreamingApi { .. } => /* ... */,
                    QueryState::ToolExecution { .. } => /* ... */,
                    QueryState::Compaction { .. } => /* ... */,
                    QueryState::StopHooks { .. } => /* ... */,
                    QueryState::Terminal { .. } => break,
                };
            }
        }
    }
}
```

**Task 1.2: Implement StreamingApi state**
- **Estimated Lines:** ~200
- **Priority:** HIGHEST
- **Dependencies:** Task 1.1

```rust
QueryState::StreamingApi { turn, usage } => {
    // 1. Normalize messages for API
    let messages_for_api = normalize_messages_for_api(&state.messages);
    
    // 2. Call provider.stream()
    let stream = self.provider.stream(messages_for_api, self.tool_definitions.clone()).await?;
    
    // 3. Process SSE events
    let mut accumulated_usage = Usage::default();
    let mut pending_tool_blocks = vec![];
    let mut assistant_content = vec![];
    
    while let Some(event) = stream.next().await {
        match event {
            StreamEvent::MessageStart { id, model } => { /* ... */ }
            StreamEvent::ContentBlockDelta { delta } => { 
                match delta {
                    ContentDelta::Text { text } => {
                        assistant_content.push(ContentBlock::Text { text });
                        yield QueryOutput::StreamEvent(/* ... */);
                    }
                    // ... other deltas
                }
            }
            StreamEvent::MessageDelta { stop_reason, usage } => { /* ... */ }
            // ... other events
        }
    }
    
    // 4. Create assistant message
    let assistant_msg = create_assistant_message(/* ... */);
    state.messages.push(assistant_msg);
    
    // 5. Determine next state
    if !pending_tool_blocks.is_empty() {
        QueryState::ToolExecution { pending_blocks, concurrent: true }
    } else {
        QueryState::StopHooks { assistant_messages: vec![] }
    }
}
```

**Task 1.3: Implement ToolExecution state**
- **Estimated Lines:** ~150
- **Priority:** HIGH
- **Dependencies:** Task 1.2

```rust
QueryState::ToolExecution { pending_blocks, concurrent } => {
    // 1. Build tool context
    let tool_context = ToolContext {
        cwd: self.config.cwd.clone(),
        messages: state.messages.clone(),
        // ...
    };
    
    // 2. Partition tools
    let batches = partition_tool_calls(pending_blocks, &self.tools);
    
    // 3. Execute each batch
    for batch in batches {
        let results = if batch.is_concurrent {
            run_tools_concurrently(batch.blocks, self.tools.clone(), tool_context.clone(), 10)
        } else {
            run_tools_serially(batch.blocks, self.tools.clone(), tool_context.clone())
        };
        
        // 4. Yield results
        while let Some(result) = results.next().await {
            yield QueryOutput::ToolResult { /* ... */ };
            
            // 5. Add tool result message
            let tool_result_msg = Message::user_tool_result(/* ... */);
            state.messages.push(tool_result_msg);
        }
    }
    
    // 6. Transition back to streaming
    state.turn_count += 1;
    QueryState::StreamingApi { turn: state.turn_count, usage: state.total_usage.clone() }
}
```

**Task 1.4: Implement StopHooks state**
- **Estimated Lines:** ~50
- **Priority:** MEDIUM
- **Dependencies:** Task 1.2

```rust
QueryState::StopHooks { assistant_messages } => {
    let hooks = default_stop_hooks();
    let context = StopHookContext {
        turn: state.turn_count,
        max_turns: self.config.max_turns,
        // ...
    };
    
    let result = run_stop_hooks(&state.messages, &assistant_messages, &context, &hooks).await;
    
    if result.prevent_continuation {
        QueryState::Terminal { reason: TerminalReason::StopHookPrevented }
    } else {
        QueryState::Terminal { reason: TerminalReason::Completed }
    }
}
```

**Task 1.5: Implement Compaction state**
- **Estimated Lines:** ~100
- **Priority:** MEDIUM
- **Dependencies:** Task 1.2

```rust
QueryState::Compaction { trigger } => {
    let result = match trigger {
        CompactionTrigger::Auto { token_threshold } => {
            auto_compact(&state.messages, token_threshold).await
        }
        CompactionTrigger::Reactive { error } => {
            reactive_compact(&state.messages, &error).await
        }
        CompactionTrigger::Manual => { /* ... */ }
    };
    
    match result {
        Ok(compaction_result) => {
            state.messages = compaction_result.messages;
            QueryState::StreamingApi { turn: state.turn_count, usage: state.total_usage.clone() }
        }
        Err(e) => QueryState::Terminal { reason: TerminalReason::Error { message: e.to_string() } }
    }
}
```

---

### Phase 2: Integration (IMPORTANT)

**Task 2.1: Add get_messages() accessor**
- **Estimated Lines:** ~5
- **Priority:** HIGH

```rust
pub fn get_messages(&self) -> Vec<Message> {
    self.messages.read().clone()
}
```

**Task 2.2: Wire budget tracking**
- **Estimated Lines:** ~30
- **Priority:** MEDIUM

```rust
// In StreamingApi state, after MessageDelta:
self.budget_tracker.write().add_cost(0.0, 
    accumulated_usage.input_tokens + accumulated_usage.output_tokens
);

if self.budget_tracker.read().is_budget_exceeded() {
    query_state = QueryState::Terminal {
        reason: TerminalReason::BudgetExceeded { /* ... */ }
    };
}
```

**Task 2.3: Wire error recovery**
- **Estimated Lines:** ~50
- **Priority:** MEDIUM

```rust
// In StreamingApi error handling:
match event {
    StreamEvent::Error { message } => {
        if message.contains("max_output_tokens") {
            if let Some(reason) = handle_max_output_tokens_recovery(&mut state, 3).await {
                query_state = QueryState::StreamingApi { /* ... */ };
                continue;
            }
        }
        if message.contains("prompt_too_long") {
            query_state = QueryState::Compaction { 
                trigger: CompactionTrigger::Reactive { error: message }
            };
            continue;
        }
        // ...
    }
}
```

---

### Phase 3: Testing (REQUIRED)

**Task 3.1: Fix test compilation**
- **Estimated Lines:** ~20
- **Priority:** HIGH

```rust
// Add at end of query_engine.rs:
struct EmptyToolRegistry;
struct EmptyProvider;

#[async_trait::async_trait]
impl ToolRegistry for EmptyToolRegistry {
    fn get(&self, _: &str) -> Option<Arc<dyn Tool>> { None }
    fn list(&self) -> Vec<Arc<dyn Tool>> { vec![] }
}

#[async_trait::async_trait]
impl hcode_provider::Provider for EmptyProvider {
    fn name(&self) -> &str { "empty" }
    fn model(&self) -> &str { "none" }
    async fn stream(&self, _: Vec<Message>, _: Vec<ToolDefinition>) 
        -> Result<Pin<Box<dyn Stream<Item = StreamEvent> + Send>>, ProviderError> {
        Err(ProviderError::Api("No provider".to_string()))
    }
    async fn complete(&self, _: Vec<Message>, _: Vec<ToolDefinition>) 
        -> Result<CompletionResponse, ProviderError> {
        Err(ProviderError::Api("No provider".to_string()))
    }
}
```

**Task 3.2: Add integration tests**
- **Estimated Lines:** ~200
- **Priority:** MEDIUM

```rust
#[tokio::test]
async fn test_streaming_basic() {
    // Mock provider that returns text
    let provider = Arc::new(MockProvider::new("Hello world"));
    let engine = QueryEngine::new(config, tools, provider);
    
    let stream = engine.submit_message("test".to_string());
    pin_mut!(stream);
    
    let mut events = vec![];
    while let Some(output) = stream.next().await {
        events.push(output);
    }
    
    assert!(events.iter().any(|e| matches!(e, QueryOutput::StreamEvent(_))));
}

#[tokio::test]
async fn test_tool_execution() {
    // Mock provider that returns tool_use block
    // Verify tool is called
    // Verify tool_result is added
}
```

---

## Implementation Order

### Wave 1 (Critical Path)
1. Task 1.1: submit_message() structure
2. Task 1.2: StreamingApi state
3. Task 2.1: get_messages() accessor
4. Task 3.1: Fix test compilation

**Milestone:** Basic streaming works, tests compile

### Wave 2 (Core Functionality)
5. Task 1.3: ToolExecution state
6. Task 1.4: StopHooks state
7. Task 2.2: Wire budget tracking

**Milestone:** Tool execution works, budget enforced

### Wave 3 (Robustness)
8. Task 1.5: Compaction state
9. Task 2.3: Wire error recovery
10. Task 3.2: Integration tests

**Milestone:** Full functionality, all edge cases handled

---

## Reference Implementation

**TypeScript Source:** `thirdparty/cc-haha-main/src/query.ts`
- Lines 219-239: query() generator
- Lines 241-999: queryLoop() implementation
- Lines 659-863: LLM streaming loop
- Lines 841-862: Tool execution streaming
- Lines 999+: Stop hooks

**Key Patterns to Replicate:**
1. Async generator with yield*
2. State machine with transitions
3. SSE event processing
4. Tool accumulation and execution
5. Error handling and recovery

---

## Complexity Analysis

| Component | TypeScript Lines | Rust Lines (Est.) | Complexity |
|-----------|-----------------|-------------------|------------|
| submit_message() | 80 | 150 | Medium |
| StreamingApi | 300 | 200 | High |
| ToolExecution | 100 | 150 | Medium |
| StopHooks | 50 | 50 | Low |
| Compaction | 80 | 100 | Medium |
| Integration | 100 | 80 | Low |
| Tests | 150 | 200 | Medium |
| **Total** | **860** | **~930** | - |

---

## Acceptance Criteria

**Minimum Viable:**
- [ ] submit_message() exists and returns Stream
- [ ] StreamingApi calls provider.stream()
- [ ] Text content is accumulated
- [ ] Assistant messages are created
- [ ] Tests compile and pass

**Full Functionality:**
- [ ] Tool execution works
- [ ] Stop hooks prevent continuation
- [ ] Budget tracking enforced
- [ ] Error recovery handled
- [ ] Compaction triggered

**Production Ready:**
- [ ] Integration tests pass
- [ ] Edge cases handled
- [ ] Performance acceptable
- [ ] Documentation complete

---

## Next Steps

**For User:**
1. Review this implementation plan
2. Decide if architecture meets expectations
3. Choose: continue implementation OR delegate to another session/developer

**For Continuation:**
1. Start with Task 1.1 (submit_message structure)
2. Follow Wave 1 tasks sequentially
3. Verify each task with tests before proceeding
4. Use Oracle verification after each wave

**For Delegation:**
1. Share this plan with implementer
2. Provide TypeScript reference code
3. Review after each wave completion
4. Use Oracle for final verification

---

## Files to Modify

| File | Current Lines | Target Lines | Changes |
|------|--------------|--------------|---------|
| query_engine.rs | 303 | ~900 | +600 lines |
| lib.rs | 14 | 14 | None |
| state.rs | 200 | 200 | None |
| tool_orchestration.rs | 188 | 188 | None |
| compact/mod.rs | 40 | 40 | None |
| compact/auto.rs | 70 | 70 | None |
| compact/reactive.rs | 120 | 120 | None |
| stop_hooks.rs | 150 | 150 | None |
| error_recovery.rs | 120 | 120 | None |
| budget.rs | 120 | 120 | None |

---

## Conclusion

**What You Have:**
- Complete architecture matching TypeScript design
- All supporting modules (tools, compaction, hooks, recovery, budget)
- Clean compilation
- Well-structured foundation

**What You Need:**
- ~600 lines of core logic in query_engine.rs
- Primarily in submit_message() and StreamingApi state
- Estimated 8-12 days for careful implementation

**Recommendation:**
Use this plan to continue implementation in a fresh session, or delegate to a developer familiar with Rust async patterns. The architecture is sound and ready for the core logic.