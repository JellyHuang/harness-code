# AGENTS.md - hcode-engine

## OVERVIEW

Core execution engine for HCode. Handles query lifecycle, state management, tool orchestration, error recovery, and streaming execution.

## WHERE TO LOOK

| File | Purpose |
|------|---------|
| `query_engine.rs` | Main orchestrator - query lifecycle, state machine, event handling |
| `streaming_tool_executor.rs` | Tools execute during streaming (not after complete response) |
| `error_recovery.rs` | Prompt-too-long, max output tokens, model fallback recovery |
| `compact/` | Auto-compaction: reactive, micro, auto modes |
| `tool_orchestration.rs` | Concurrent/serial tool execution scheduling |
| `tool_result_budget.rs` | Per-message budget limiting on tool result size |
| `stop_hooks.rs` | Query lifecycle hooks (start, stop, events) |
| `state.rs` | Query state tracking: messages, token usage, tool denials |
| `budget.rs` | Cost tracking in USD |
| `pool.rs` | Worker pool management |
| `event.rs` | Event types for streaming protocol |

## KEY TYPES

- `QueryEngine`: Main orchestrator, owns state and runs query lifecycle
- `QueryEngineConfig`: Configuration for compaction, budget, timeouts, hooks

## P0 FEATURES

- **Streaming Tool Executor**: Tools execute during streaming (not after complete response)
- **Prompt-Too-Long Recovery**: Automatic compaction and retry with context reduction
- **Max Output Tokens Recovery**: Escalation and multi-turn recovery strategies
- **Model Fallback Handling**: Automatic switch to fallback model on failures
- **Tool Result Budget**: Per-message budget limiting total tool result size

## NOTES

- Compaction modes: reactive (on overflow), micro (aggressive), auto (adaptive)
- Tool orchestration supports both concurrent and serial execution patterns
- All cost tracking in USD via `budget.rs`
- Events flow through streaming protocol for real-time UI updates
