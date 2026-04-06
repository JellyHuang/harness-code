# PROJECT KNOWLEDGE BASE

**Generated:** Mon Apr 06 2026

## OVERVIEW

HCode - AI coding agent with coordinator/worker architecture in Rust. Multi-agent orchestration with XML notifications.

## STRUCTURE

```
hcode-rust/
├── hcode/            # CLI binary (clap)
├── hcode-types/      # Core domain types
├── hcode-protocol/   # XML notifications, SSE events
├── hcode-provider/   # LLM providers (Anthropic, OpenAI)
├── hcode-tools/      # Tool implementations
├── hcode-permission/ # Permission engine
├── hcode-engine/     # QueryEngine, Coordinator, Worker
├── hcode-config/     # YAML/JSON config loading
├── hcode-session/    # SQLite persistence
└── hcode-mcp/        # MCP protocol client
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| CLI entry | `hcode/src/main.rs` | clap-based subcommands |
| QueryEngine | `hcode-engine/src/query_engine.rs` | Main orchestrator |
| Coordinator | `hcode-engine/src/coordinator.rs` | Multi-agent management |
| Worker | `hcode-engine/src/worker.rs` | Sub-agent execution |
| Tool trait | `hcode-tools/src/tool.rs` | Tool interface |
| Tool registry | `hcode-tools/src/registry.rs` | Tool registration |
| Providers | `hcode-provider/src/` | Anthropic, OpenAI impls |
| Config | `hcode-config/src/lib.rs` | JSON/JSONC loading |
| XML protocol | `hcode-protocol/src/xml.rs` | Task notifications |
| SSE events | `hcode-protocol/src/sse.rs` | Streaming events |

## CONVENTIONS

- **Cargo workspace** with 10 crates
- **Async**: tokio for runtime, async-trait for traits
- **Errors**: thiserror for error types
- **Serialization**: serde with derive
- **State**: parking_lot::RwLock for mutable state
- **CLI**: clap with derive macros

## COMMANDS

```bash
# Build
cargo build --release

# Run CLI
./target/release/hcode run -p "prompt"

# List agents
./target/release/hcode agent list

# Show config
./target/release/hcode config show
```

## NOTES

- Entry point: `hcode/src/main.rs`
- Config paths: `~/.config/hcode/config.json`, `./.hcode/config.json`
- API keys: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY` env vars
- Config format: JSON/JSONC with env substitution via `{env:VAR}`
- XML notifications used for inter-agent communication
