# HCode

**AI coding agent with coordinator/worker architecture written in Rust.**

HCode is an AI-powered coding assistant that combines the best patterns from [Claude Code](https://github.com/anthropics/claude-code) and [OpenCode](https://github.com/opencode-ai/opencode).

## Features

- **Coordinator/Worker Architecture**: Multi-agent orchestration with XML notifications
- **Multi-Provider Support**: Anthropic, OpenAI, OpenRouter, Azure, Bedrock
- **Streaming Responses**: Real-time SSE streaming with thinking block support
- **Tool System**: Bash, Read, Write, Edit, Glob, Grep, WebFetch, WebSearch
- **Permission System**: Fine-grained access control for tools
- **MCP Protocol**: Model Context Protocol client support

## Installation

```bash
# Build from source
cd hcode-rust
cargo build --release

# The binary will be at target/release/hcode
```

## Usage

```bash
# Run with a prompt
hcode run -p "Help me implement a REST API"

# Interactive mode
hcode run

# Use a specific provider
hcode run --provider anthropic --model claude-sonnet-4-20250514

# List available agents
hcode agent list

# Run a specific agent
hcode agent run researcher "Investigate the codebase structure"
```

## Configuration

HCode looks for configuration in:
1. `~/.config/hcode/config.yaml`
2. `./.hcode/config.yaml`
3. `HCODE_CONFIG_PATH` environment variable

Example configuration:

```yaml
provider:
  anthropic:
    api_key: ${ANTHROPIC_API_KEY}
    model: claude-sonnet-4-20250514
  
agents:
  coordinator:
    name: Coordinator
    model: claude-sonnet-4-20250514
    tools: [agent, send_message, task_stop]
    
  researcher:
    name: Researcher
    model: claude-sonnet-4-20250514
    tools: [bash, read, glob, grep, webfetch]
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         HCODE ARCHITECTURE                          │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────┐
│   Coordinator   │  (Main orchestrator)
│   QueryEngine   │
└────────┬────────┘
         │ AgentTool.spawn()
         ▼
┌─────────────────┐     ┌─────────────────┐
│    Worker 1     │     │    Worker 2     │
│  (sub-agent)    │     │  (sub-agent)    │
└────────┬────────┘     └────────┬────────┘
         │ XML notification
         ▼
┌─────────────────────────────────────────────────────────────────────┐
│  <task-notification>                                                 │
│    <task-id>agent-abc123</task-id>                                   │
│    <status>completed</status>                                         │
│    <summary>Task completed</summary>                                  │
│  </task-notification>                                                │
└─────────────────────────────────────────────────────────────────────┘
```

## Crate Structure

```
hcode-rust/
├── hcode-types/      # Core domain types
├── hcode-protocol/   # XML notifications, SSE events
├── hcode-provider/   # LLM provider abstraction
├── hcode-tools/      # Tool implementations
├── hcode-permission/ # Permission engine
├── hcode-engine/     # QueryEngine, Coordinator, Worker
├── hcode-config/     # Configuration loading
├── hcode-session/    # Session persistence
├── hcode-mcp/        # MCP protocol client
└── hcode/            # CLI binary
```

## API Keys

Set environment variables:
- `ANTHROPIC_API_KEY` for Anthropic Claude
- `OPENAI_API_KEY` for OpenAI GPT

## Reference Implementations

HCode follows patterns from:
- [cc-haha-main](./thirdparty/cc-haha-main) - Claude Code TypeScript patterns
- [opencode-dev](./thirdparty/opencode-dev) - OpenCode TypeScript patterns

## License

MIT