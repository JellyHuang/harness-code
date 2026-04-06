# Architecture Overview

## Module Structure

```
hcode.dev/
├── pkg/                    # Public packages (importable by external projects)
│   ├── agent/             # Agent definitions and registry
│   │   ├── definition.go  # Definition, Registry, PermissionMode
│   │   └── options.go     # CoordinatorOptions, WorkerOptions
│   │
│   ├── protocol/          # XML notification protocol
│   │   └── notification.go # TaskNotification, TaskStatus, AgentMessage
│   │
│   ├── llm/               # LLM provider abstraction
│   │   ├── types.go       # Message, Response, Tool, Usage
│   │   ├── provider.go    # Provider interface, Registry
│   │   └── providers.go   # AnthropicProvider, OpenAIProvider
│   │
│   ├── tools/             # Tool system
│   │   ├── registry.go    # Registry, BaseTool interface, ToolInfo
│   │   └── builtin.go     # Bash, Read, Write, Edit, Grep, Glob
│   │
│   └── core/              # Composition package
│       └── core.go        # Imports all pkg/* modules
│
├── internal/              # Private packages (not importable)
│   ├── agent/             # Execution layer
│   │   ├── coordinator.go # Coordinator orchestrates workers
│   │   ├── worker.go      # Worker executes tasks
│   │   └── pool.go        # Pool manages multiple workers
│   │
│   ├── config/            # Configuration management
│   │   └── config.go      # Load, Save, DefaultConfig
│   │
│   ├── state/             # State persistence
│   │   ├── state.go       # AgentState, Store interface
│   │   ├── sqlite_store.go # SQLite implementation
│   │   └── json_store.go  # JSON file implementation
│   │
│   ├── messaging/         # Message queue
│   │   ├── queue.go       # Queue interface, MemoryQueue
│   │   ├── sqlite_queue.go # SQLite implementation
│   │   └── file_queue.go  # File-based implementation
│   │
│   ├── scratchpad/        # State sharing
│   │   └── scratchpad.go  # Filesystem-based sharing
│   │
│   └── worktree/          # Git isolation
│       └── worktree.go    # Git worktree management
│
└── cmd/hcode/             # CLI entry point
    └── main.go            # Cobra commands
```

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI Layer                                │
│                         cmd/hcode                                │
│                    (Cobra commands: agent, config)              │
└──────────────────────────┬──────────────────────────────────────┘
                           │
┌──────────────────────────┴──────────────────────────────────────┐
│                     Application Layer                            │
│                      internal/agent                              │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐        │
│  │ Coordinator │───▶│    Pool     │───▶│   Worker    │        │
│  │ (Orchestr.) │    │ (Manage)    │    │ (Execute)   │        │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘        │
│         │                  │                   │                │
│         │                  └───────────────────┘                │
│         │                          │                            │
│         ▼                          ▼                            │
│  ┌─────────────┐           ┌─────────────┐                     │
│  │  messaging  │           │    state    │                     │
│  │   (Queue)   │           │   (Store)   │                     │
│  └─────────────┘           └─────────────┘                     │
└─────────────────────────────────────────────────────────────────┘
                           │
┌──────────────────────────┴──────────────────────────────────────┐
│                       Core Layer (pkg/)                          │
│                     (Importable by external)                     │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │    agent    │  │   protocol  │  │     llm     │             │
│  │ (Def, Reg)  │  │  (XML Not.) │  │ (Providers) │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐                               │
│  │    tools    │  │    core     │                               │
│  │ (Registry)  │  │ (Compose)   │                               │
│  └─────────────┘  └─────────────┘                               │
└─────────────────────────────────────────────────────────────────┘
```

## Data Flow

```
User Prompt
     │
     ▼
┌─────────────┐
│ Coordinator │ ──── Spawn Workers ────┐
└──────┬──────┘                         │
       │                                ▼
       │                        ┌─────────────┐
       │                        │   Worker    │
       │                        │             │
       │                        │ 1. Call LLM │
       │                        │ 2. Exec Tool│
       │                        │ 3. Loop     │
       │                        └──────┬──────┘
       │                               │
       │◄──── XML Notification ────────┘
       │        (Task completed)
       │
       ▼
┌─────────────┐
│ Update State│
└─────────────┘
```

## Module Dependencies

```
cmd/hcode
    │
    ├── internal/agent (execution layer)
    │       ├── pkg/agent (definitions)
    │       ├── pkg/llm (providers)
    │       ├── pkg/tools (tool registry)
    │       ├── pkg/protocol (notifications)
    │       ├── internal/state (persistence)
    │       └── internal/messaging (queue)
    │
    └── internal/config (configuration)

pkg/core (composition)
    ├── pkg/agent
    ├── pkg/llm
    ├── pkg/tools
    └── pkg/protocol
```

## Key Design Decisions

### 1. pkg/ vs internal/

- **pkg/**: Public packages that can be imported by external projects
  - `pkg/agent`: Agent definitions (no internal dependencies)
  - `pkg/protocol`: Protocol types (no dependencies)
  - `pkg/llm`: LLM abstraction (no internal dependencies)
  - `pkg/tools`: Tool system (no internal dependencies)

- **internal/**: Private packages for application logic
  - `internal/agent`: Coordinator/Worker execution (depends on pkg/*)
  - `internal/state`: State persistence (application-specific)
  - `internal/messaging`: Message queue (application-specific)

### 2. Go Workspace

Each `pkg/*` module has its own `go.mod`, enabling:
- Independent versioning
- Separate publishing
- Clear dependency boundaries
- Future C++ migration alignment

### 3. Protocol Layer

The `pkg/protocol` package defines the XML notification format:
- Language-agnostic (can be implemented in any language)
- Cross-module communication
- Future C++ migration target

## Future Expansion

The `apps/` directory is reserved for:
- `apps/cli/`: Enhanced CLI (current functionality)
- `apps/web/`: Web interface (like OpenClaw)
- `apps/desktop/`: Desktop application (Tauri/Electron)