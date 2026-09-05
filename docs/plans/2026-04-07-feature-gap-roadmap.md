# Feature Gap Roadmap - hcode-rust vs cc-haha-main

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement each phase.

**Goal:** Systematically close the feature gap between hcode-rust and cc-haha-main, prioritizing core functionality first.

**Current Coverage:** ~15-20%

**Target:** ~80% feature parity (excluding UI-specific features)

---

## Phase Overview

| Phase | Focus | Duration | Coverage After |
|-------|-------|----------|----------------|
| Phase 1 | Agent Architecture | 2-3 days | ~25% |
| Phase 2 | Extension System | 2-3 days | ~35% |
| Phase 3 | Network Tools | 1-2 days | ~45% |
| Phase 4 | Task & Communication Tools | 2 days | ~55% |
| Phase 5 | Intelligence Tools | 2-3 days | ~65% |
| Phase 6 | Additional Providers | 1-2 days | ~70% |
| Phase 7 | Commands & UX | 2 days | ~80% |

---

## Phase 1: Agent Architecture (P0 - Critical)

**Goal:** Implement multi-agent orchestration with Coordinator/Worker pattern.

### Task 1.1: AgentTool Implementation

**Files:**
- Create: `hcode-tools/src/agent/mod.rs`
- Create: `hcode-tools/src/agent/schema.rs`
- Create: `hcode-tools/src/agent/spawner.rs`
- Create: `hcode-tools/src/agent/built_in.rs`
- Test: `hcode-tools/tests/agent_tool_test.rs`

**Dependencies:** None

**Steps:**

1. Create agent module structure with schema definitions:
```rust
// schema.rs
pub struct AgentInput {
    pub agent_name: String,
    pub prompt: String,
    pub tools: Option<Vec<String>>,
    pub model: Option<String>,
    pub disallowed_tools: Option<Vec<String>>,
}

pub struct AgentOutput {
    pub agent_id: String,
    pub status: String, // "running", "completed", "failed"
    pub result: Option<String>,
}

pub static AGENT_SCHEMA: Value = json!({
    "type": "object",
    "properties": {
        "agent_name": { "type": "string", "description": "Name of agent to spawn" },
        "prompt": { "type": "string", "description": "Task for the agent" },
        "tools": { "type": "array", "items": { "type": "string" } },
        "model": { "type": "string" },
        "disallowed_tools": { "type": "array", "items": { "type": "string" } }
    },
    "required": ["agent_name", "prompt"]
});
```

2. Implement agent spawner with Worker integration:
```rust
// spawner.rs
pub async fn spawn_agent(
    input: AgentInput,
    context: ToolContext,
    coordinator: Arc<Coordinator>,
) -> Result<AgentOutput, ToolError> {
    let agent_id = uuid::Uuid::new_v4().to_string();
    
    // Create worker for sub-agent
    let worker = Worker::new(
        agent_id.clone(),
        input.prompt,
        input.tools,
        input.model,
    );
    
    // Register with coordinator
    coordinator.register_worker(agent_id.clone(), worker.clone()).await;
    
    // Start worker execution
    tokio::spawn(async move {
        worker.run().await;
    });
    
    Ok(AgentOutput {
        agent_id,
        status: "running".to_string(),
        result: None,
    })
}
```

3. Add built-in agents:
```rust
// built_in.rs
pub fn get_builtin_agents() -> Vec<AgentDefinition> {
    vec![
        AgentDefinition {
            name: "researcher".to_string(),
            description: "Search and analyze codebase".to_string(),
            tools: vec!["bash", "read", "glob", "grep", "webfetch"],
            model: None,
        },
        AgentDefinition {
            name: "coder".to_string(),
            description: "Write and edit code".to_string(),
            tools: vec!["bash", "read", "write", "edit", "glob", "grep"],
            model: None,
        },
        AgentDefinition {
            name: "reviewer".to_string(),
            description: "Review and verify code".to_string(),
            tools: vec!["read", "glob", "grep"],
            model: None,
        },
    ]
}
```

4. Write tests for AgentTool

**Verification:** `cargo test --package hcode-tools agent_tool`

---

### Task 1.2: Coordinator Implementation

**Files:**
- Modify: `hcode-engine/src/coordinator.rs`
- Create: `hcode-engine/src/coordinator/worker_registry.rs`
- Create: `hcode-engine/src/coordinator/notification.rs`
- Create: `hcode-engine/src/coordinator/message_router.rs`
- Test: `hcode-engine/tests/coordinator_test.rs`

**Dependencies:** Task 1.1

**Steps:**

1. Implement WorkerRegistry:
```rust
// worker_registry.rs
pub struct WorkerRegistry {
    workers: RwLock<HashMap<String, WorkerHandle>>,
    notifications: mpsc::Receiver<WorkerNotification>,
}

pub struct WorkerHandle {
    pub id: String,
    pub status: WorkerStatus,
    pub sender: mpsc::Sender<WorkerMessage>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum WorkerStatus {
    Running,
    Completed,
    Failed,
    Timeout,
}

#[derive(Debug, Clone)]
pub struct WorkerNotification {
    pub worker_id: String,
    pub event: NotificationEvent,
}

pub enum NotificationEvent {
    Started,
    Progress { message: String },
    ToolUse { tool: String, input: Value },
    Completed { result: String },
    Failed { error: String },
}
```

2. Implement XML notification system:
```rust
// notification.rs
pub fn format_xml_notification(notification: &WorkerNotification) -> String {
    format!(
        r#"<task-notification>
  <task-id>{}</task-id>
  <status>{}</status>
  <summary>{}</summary>
</task-notification>"#,
        notification.worker_id,
        notification.event.status(),
        notification.event.summary()
    )
}
```

3. Implement message router:
```rust
// message_router.rs
pub struct MessageRouter {
    coordinator_sender: mpsc::Sender<CoordinatorMessage>,
}

impl MessageRouter {
    pub async fn route_to_worker(&self, worker_id: &str, message: WorkerMessage) {
        // Route message to specific worker
    }
    
    pub async fn broadcast(&self, message: CoordinatorMessage) {
        // Broadcast to all workers
    }
}
```

4. Complete Coordinator struct:
```rust
// coordinator.rs
pub struct Coordinator {
    registry: Arc<WorkerRegistry>,
    router: MessageRouter,
    notification_tx: mpsc::Sender<WorkerNotification>,
    config: CoordinatorConfig,
}

impl Coordinator {
    pub async fn spawn_worker(&self, spec: WorkerSpec) -> Result<String, CoordinatorError>;
    pub async fn stop_worker(&self, worker_id: &str) -> Result<(), CoordinatorError>;
    pub async fn get_worker_status(&self, worker_id: &str) -> Option<WorkerStatus>;
    pub async fn list_workers(&self) -> Vec<WorkerInfo>;
    pub async fn wait_for_completion(&self, worker_id: &str, timeout: Duration) -> Result<String, CoordinatorError>;
}
```

5. Write tests

**Verification:** `cargo test --package hcode-engine coordinator`

---

### Task 1.3: Worker Implementation

**Files:**
- Modify: `hcode-engine/src/worker.rs`
- Create: `hcode-engine/src/worker/execution.rs`
- Create: `hcode-engine/src/worker/state.rs`
- Create: `hcode-engine/src/worker/communication.rs`
- Test: `hcode-engine/tests/worker_test.rs`

**Dependencies:** Task 1.2

**Steps:**

1. Implement Worker execution:
```rust
// execution.rs
pub struct WorkerExecutor {
    worker_id: String,
    prompt: String,
    tools: ToolRegistry,
    model: Option<String>,
    provider: Arc<dyn Provider>,
}

impl WorkerExecutor {
    pub async fn run(&mut self) -> WorkerResult {
        // 1. Build system prompt
        // 2. Initialize message history
        // 3. Execute conversation loop
        // 4. Stream results via notifications
        // 5. Handle tool calls
        // 6. Return final result
    }
}
```

2. Implement Worker state machine:
```rust
// state.rs
pub enum WorkerState {
    Idle,
    Running { turn: u32 },
    WaitingForTool { tool_use_id: String },
    Completed,
    Failed { error: String },
}

pub struct WorkerStateMachine {
    state: WorkerState,
    messages: Vec<Message>,
    tool_results: HashMap<String, ToolResult>,
}
```

3. Implement communication:
```rust
// communication.rs
pub struct WorkerCommunication {
    notification_tx: mpsc::Sender<WorkerNotification>,
    message_rx: mpsc::Receiver<WorkerMessage>,
}

impl WorkerCommunication {
    pub async fn send_notification(&self, event: NotificationEvent);
    pub async fn receive_message(&self) -> Option<WorkerMessage>;
}
```

4. Complete Worker struct:
```rust
// worker.rs
pub struct Worker {
    id: String,
    executor: WorkerExecutor,
    state: WorkerStateMachine,
    comm: WorkerCommunication,
    config: WorkerConfig,
}

impl Worker {
    pub async fn run(mut self) -> WorkerResult;
    pub async fn handle_tool_call(&mut self, tool_use: ToolUse) -> Result<(), WorkerError>;
    pub async fn send_progress(&self, message: &str);
}
```

5. Write tests

**Verification:** `cargo test --package hcode-engine worker`

---

### Task 1.4: TaskOutputTool Implementation

**Files:**
- Create: `hcode-tools/src/task_output/mod.rs`
- Create: `hcode-tools/src/task_output/schema.rs`
- Test: `hcode-tools/tests/task_output_tool_test.rs`

**Dependencies:** Task 1.2, 1.3

**Steps:**

1. Create schema:
```rust
pub struct TaskOutputInput {
    pub task_id: String,
    pub wait: Option<bool>, // Wait for completion
    pub timeout: Option<u64>,
}

pub struct TaskOutputResult {
    pub status: String,
    pub result: Option<String>,
    pub error: Option<String>,
}
```

2. Implement tool:
```rust
impl Tool for TaskOutputTool {
    fn name(&self) -> &str { "task_output" }
    
    async fn call(&self, input: Value, context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: TaskOutputInput = serde_json::from_value(input)?;
        
        // Get coordinator from context
        let coordinator = context.coordinator.ok_or(ToolError::NotAvailable)?;
        
        if params.wait.unwrap_or(false) {
            let result = coordinator.wait_for_completion(
                &params.task_id,
                Duration::from_millis(params.timeout.unwrap_or(60000))
            ).await?;
            
            Ok(ToolResult::success(json!(TaskOutputResult {
                status: "completed".to_string(),
                result: Some(result),
                error: None,
            })))
        } else {
            let status = coordinator.get_worker_status(&params.task_id);
            Ok(ToolResult::success(json!(TaskOutputResult {
                status: status.map(|s| s.to_string()).unwrap_or("not_found".to_string()),
                result: None,
                error: None,
            })))
        }
    }
}
```

3. Write tests

**Verification:** `cargo test --package hcode-tools task_output`

---

### Task 1.5: TaskStopTool Implementation

**Files:**
- Create: `hcode-tools/src/task_stop/mod.rs`
- Create: `hcode-tools/src/task_stop/schema.rs`
- Test: `hcode-tools/tests/task_stop_tool_test.rs`

**Dependencies:** Task 1.2

**Steps:**

1. Create schema and implement tool to stop running workers

**Verification:** `cargo test --package hcode-tools task_stop`

---

### Task 1.6: SendMessageTool Implementation

**Files:**
- Create: `hcode-tools/src/send_message/mod.rs`
- Create: `hcode-tools/src/send_message/schema.rs`
- Test: `hcode-tools/tests/send_message_tool_test.rs`

**Dependencies:** Task 1.2

**Steps:**

1. Create schema:
```rust
pub struct SendMessageInput {
    pub message: String,
    pub target: Option<String>, // None = coordinator, Some(id) = specific worker
}
```

2. Implement tool to send messages via coordinator

**Verification:** `cargo test --package hcode-tools send_message`

---

### Phase 1 Milestone

- [ ] AgentTool spawns sub-agents
- [ ] Coordinator manages multiple workers
- [ ] Workers execute independently
- [ ] XML notifications flow correctly
- [ ] TaskOutputTool retrieves worker results
- [ ] TaskStopTool stops workers
- [ ] SendMessageTool sends messages
- [ ] All tests pass

**Commands:**
```bash
cargo test --package hcode-tools agent
cargo test --package hcode-engine coordinator
cargo test --package hcode-engine worker
```

---

## Phase 2: Extension System (P0 - Critical)

**Goal:** Enable user-defined extensions via Skills, Plugins, and Hooks.

### Task 2.1: Hook System Foundation

**Files:**
- Create: `hcode-engine/src/hooks/mod.rs`
- Create: `hcode-engine/src/hooks/registry.rs`
- Create: `hcode-engine/src/hooks/executor.rs`
- Create: `hcode-engine/src/hooks/events.rs`
- Test: `hcode-engine/tests/hooks_test.rs`

**Dependencies:** None

**Steps:**

1. Define hook events:
```rust
// events.rs
#[derive(Debug, Clone, Serialize)]
pub enum HookEvent {
    PreToolUse { tool: String, input: Value },
    PostToolUse { tool: String, result: ToolResult },
    PreQuery { prompt: String },
    PostQuery { response: String },
    SessionStart,
    SessionEnd,
    Error { error: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct HookConfig {
    pub event: String,
    pub command: String,
    pub timeout: Option<u64>,
    pub enabled: Option<bool>,
}
```

2. Implement registry:
```rust
// registry.rs
pub struct HookRegistry {
    hooks: HashMap<String, Vec<HookConfig>>,
}

impl HookRegistry {
    pub fn register(&mut self, event: &str, hook: HookConfig);
    pub fn get_hooks(&self, event: &str) -> Vec<&HookConfig>;
    pub fn load_from_config(&mut self, config: &Config);
}
```

3. Implement executor:
```rust
// executor.rs
pub struct HookExecutor {
    registry: Arc<HookRegistry>,
}

impl HookExecutor {
    pub async fn execute(&self, event: HookEvent) -> Result<Option<Value>, HookError> {
        let hooks = self.registry.get_hooks(event.event_name());
        
        for hook in hooks {
            if !hook.enabled.unwrap_or(true) {
                continue;
            }
            
            let result = self.run_hook(hook, &event).await?;
            
            if let Some(modification) = result {
                return Ok(Some(modification));
            }
        }
        
        Ok(None)
    }
    
    async fn run_hook(&self, hook: &HookConfig, event: &HookEvent) -> Result<Option<Value>, HookError> {
        // Execute command with event as stdin JSON
        // Parse stdout as optional modification
    }
}
```

4. Integrate with QueryEngine:
```rust
// In query_engine.rs
impl QueryEngine {
    async fn execute_tool(&self, tool_use: ToolUse) -> Result<ToolResult, QueryError> {
        // Pre-tool hook
        if let Some(modification) = self.hooks.execute(HookEvent::PreToolUse {
            tool: tool_use.name.clone(),
            input: tool_use.input.clone(),
        }).await? {
            // Apply modification
        }
        
        let result = self.tool_registry.execute(&tool_use.name, tool_use.input, context).await?;
        
        // Post-tool hook
        self.hooks.execute(HookEvent::PostToolUse {
            tool: tool_use.name,
            result: result.clone(),
        }).await?;
        
        Ok(result)
    }
}
```

5. Write tests

**Verification:** `cargo test --package hcode-engine hooks`

---

### Task 2.2: Skill System

**Files:**
- Create: `hcode-tools/src/skill/mod.rs`
- Create: `hcode-tools/src/skill/schema.rs`
- Create: `hcode-tools/src/skill/loader.rs`
- Create: `hcode-tools/src/skill/executor.rs`
- Create: `hcode-tools/src/skill/bundled.rs`
- Test: `hcode-tools/tests/skill_tool_test.rs`

**Dependencies:** Task 2.1

**Steps:**

1. Define skill schema:
```rust
// schema.rs
#[derive(Debug, Clone, Deserialize)]
pub struct SkillDefinition {
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub trigger: SkillTrigger,
    pub steps: Vec<SkillStep>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum SkillTrigger {
    Manual,
    Keyword { keywords: Vec<String> },
    Regex { pattern: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkillStep {
    pub prompt: String,
    pub tools: Option<Vec<String>>,
    pub condition: Option<String>,
}

pub struct SkillInput {
    pub skill_name: String,
    pub parameters: Option<Value>,
}

pub struct SkillOutput {
    pub result: String,
    pub steps_completed: usize,
}
```

2. Implement loader:
```rust
// loader.rs
pub struct SkillLoader {
    skill_dirs: Vec<PathBuf>,
    cache: HashMap<String, SkillDefinition>,
}

impl SkillLoader {
    pub fn load_all(&mut self) -> Result<Vec<SkillDefinition>, SkillError> {
        let mut skills = Vec::new();
        
        for dir in &self.skill_dirs {
            for entry in fs::read_dir(dir)? {
                let path = entry?.path();
                if path.extension() == Some("md") || path.extension() == Some("yaml") {
                    let skill = self.load_skill(&path)?;
                    skills.push(skill);
                }
            }
        }
        
        Ok(skills)
    }
    
    fn load_skill(&self, path: &Path) -> Result<SkillDefinition, SkillError> {
        // Parse frontmatter + markdown
    }
}
```

3. Implement executor:
```rust
// executor.rs
pub struct SkillExecutor {
    loader: Arc<SkillLoader>,
    tool_registry: Arc<ToolRegistry>,
}

impl SkillExecutor {
    pub async fn execute(&self, input: SkillInput, context: ToolContext) -> Result<SkillOutput, SkillError> {
        let skill = self.loader.get(&input.skill_name)
            .ok_or(SkillError::NotFound)?;
        
        let mut steps_completed = 0;
        let mut results = Vec::new();
        
        for step in &skill.steps {
            // Build prompt with parameters
            let prompt = self.interpolate_prompt(&step.prompt, &input.parameters);
            
            // Execute step (simplified - would use QueryEngine in reality)
            let result = self.execute_step(&prompt, step.tools.as_ref(), context.clone()).await?;
            
            results.push(result);
            steps_completed += 1;
        }
        
        Ok(SkillOutput {
            result: results.join("\n"),
            steps_completed,
        })
    }
}
```

4. Add bundled skills:
```rust
// bundled.rs
pub fn get_bundled_skills() -> Vec<SkillDefinition> {
    vec![
        SkillDefinition {
            name: "explain-code".to_string(),
            description: "Explain code in detail".to_string(),
            trigger: SkillTrigger::Manual,
            steps: vec![
                SkillStep {
                    prompt: "Analyze and explain the following code:\n\n{code}".to_string(),
                    tools: Some(vec!["read".to_string()]),
                    condition: None,
                },
            ],
        },
        // More bundled skills...
    ]
}
```

5. Implement SkillTool:
```rust
// mod.rs
pub struct SkillTool;

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str { "skill" }
    
    async fn call(&self, input: Value, context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: SkillInput = serde_json::from_value(input)?;
        let executor = context.skill_executor.ok_or(ToolError::NotAvailable)?;
        
        let result = executor.execute(params, context).await?;
        
        Ok(ToolResult::success(json!(result)))
    }
}
```

6. Write tests

**Verification:** `cargo test --package hcode-tools skill`

---

### Task 2.3: Plugin System

**Files:**
- Create: `hcode-engine/src/plugins/mod.rs`
- Create: `hcode-engine/src/plugins/loader.rs`
- Create: `hcode-engine/src/plugins/registry.rs`
- Create: `hcode-engine/src/plugins/types.rs`
- Test: `hcode-engine/tests/plugins_test.rs`

**Dependencies:** Task 2.1, 2.2

**Steps:**

1. Define plugin types:
```rust
// types.rs
#[derive(Debug, Clone, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub main: String, // Entry point
    pub tools: Option<Vec<ToolDefinition>>,
    pub hooks: Option<Vec<HookConfig>>,
    pub commands: Option<Vec<CommandDefinition>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommandDefinition {
    pub name: String,
    pub description: String,
    pub handler: String,
}

pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub path: PathBuf,
    pub tools: Vec<Arc<dyn Tool>>,
    pub hooks: Vec<HookConfig>,
}
```

2. Implement loader:
```rust
// loader.rs
pub struct PluginLoader {
    plugin_dirs: Vec<PathBuf>,
}

impl PluginLoader {
    pub fn load_plugin(&self, path: &Path) -> Result<LoadedPlugin, PluginError> {
        // 1. Read manifest.json / plugin.yaml
        // 2. Validate structure
        // 3. Load tool implementations (WASM or native)
        // 4. Register hooks
    }
    
    pub fn load_all(&self) -> Result<Vec<LoadedPlugin>, PluginError> {
        let mut plugins = Vec::new();
        
        for dir in &self.plugin_dirs {
            for entry in fs::read_dir(dir)? {
                let plugin = self.load_plugin(&entry?.path())?;
                plugins.push(plugin);
            }
        }
        
        Ok(plugins)
    }
}
```

3. Implement registry:
```rust
// registry.rs
pub struct PluginRegistry {
    plugins: HashMap<String, LoadedPlugin>,
}

impl PluginRegistry {
    pub fn register(&mut self, plugin: LoadedPlugin);
    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>>;
    pub fn get_all_tools(&self) -> Vec<Arc<dyn Tool>>;
    pub fn get_hooks(&self) -> Vec<&HookConfig>;
}
```

4. Integrate with ToolRegistry and HookRegistry

5. Write tests

**Verification:** `cargo test --package hcode-engine plugins`

---

### Phase 2 Milestone

- [ ] Hooks execute on tool/query events
- [ ] Skills load from files
- [ ] SkillTool executes skills
- [ ] Plugins load and register tools/hooks
- [ ] All tests pass

---

## Phase 3: Network Tools (P1)

**Goal:** Enable web access capabilities.

### Task 3.1: WebFetchTool

**Files:**
- Create: `hcode-tools/src/web_fetch/mod.rs`
- Create: `hcode-tools/src/web_fetch/schema.rs`
- Create: `hcode-tools/src/web_fetch/fetcher.rs`
- Test: `hcode-tools/tests/web_fetch_tool_test.rs`

**Dependencies:** Add `reqwest` to workspace (already exists)

**Steps:**

1. Create schema:
```rust
pub struct WebFetchInput {
    pub url: String,
    pub method: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
    pub timeout: Option<u64>,
    pub follow_redirects: Option<bool>,
}

pub struct WebFetchOutput {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub final_url: String,
}
```

2. Implement fetcher with content extraction:
```rust
pub async fn fetch(input: WebFetchInput) -> Result<WebFetchOutput, ToolError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(input.timeout.unwrap_or(30000)))
        .redirect(if input.follow_redirects.unwrap_or(true) {
            reqwest::redirect::Policy::limited(10)
        } else {
            reqwest::redirect::Policy::none()
        })
        .build()?;
    
    let mut request = client.request(
        Method::from_str(&input.method.unwrap_or("GET".to_string()))?,
        &input.url,
    );
    
    if let Some(headers) = &input.headers {
        for (k, v) in headers {
            request = request.header(k, v);
        }
    }
    
    if let Some(body) = &input.body {
        request = request.body(body.clone());
    }
    
    let response = request.send().await?;
    
    // Extract text content, strip HTML if needed
    let body = response.text().await?;
    let content = extract_text_content(&body);
    
    Ok(WebFetchOutput {
        status: response.status().as_u16(),
        headers: response.headers().iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect(),
        body: content,
        final_url: response.url().to_string(),
    })
}
```

3. Write tests

**Verification:** `cargo test --package hcode-tools web_fetch`

---

### Task 3.2: WebSearchTool

**Files:**
- Create: `hcode-tools/src/web_search/mod.rs`
- Create: `hcode-tools/src/web_search/schema.rs`
- Create: `hcode-tools/src/web_search/searcher.rs`
- Test: `hcode-tools/tests/web_search_tool_test.rs`

**Dependencies:** Task 3.1

**Steps:**

1. Create schema:
```rust
pub struct WebSearchInput {
    pub query: String,
    pub engine: Option<String>, // "duckduckgo", "google" (via API key)
    pub limit: Option<usize>,
}

pub struct WebSearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

pub struct WebSearchOutput {
    pub results: Vec<WebSearchResult>,
    pub query: String,
    pub total: usize,
}
```

2. Implement searcher:
```rust
pub async fn search(input: WebSearchInput) -> Result<WebSearchOutput, ToolError> {
    match input.engine.as_deref() {
        Some("duckduckgo") | None => search_duckduckgo(&input.query, input.limit).await,
        Some("google") => search_google(&input.query, input.limit).await,
        _ => Err(ToolError::InvalidInput("Unknown search engine".to_string())),
    }
}

async fn search_duckduckgo(query: &str, limit: Option<usize>) -> Result<WebSearchOutput, ToolError> {
    // Use DuckDuckGo HTML search or API
    // Parse results
}

async fn search_google(query: &str, limit: Option<usize>) -> Result<WebSearchOutput, ToolError> {
    // Use Google Custom Search API if key available
}
```

3. Write tests

**Verification:** `cargo test --package hcode-tools web_search`

---

### Phase 3 Milestone

- [ ] WebFetchTool fetches URLs
- [ ] WebSearchTool searches web
- [ ] Content extraction works
- [ ] All tests pass

---

## Phase 4: Task & Communication Tools (P1)

**Goal:** Enable background tasks and interactive communication.

### Task 4.1: TodoWriteTool

**Files:**
- Create: `hcode-tools/src/todo/mod.rs`
- Create: `hcode-tools/src/todo/schema.rs`
- Create: `hcode-tools/src/todo/manager.rs`
- Test: `hcode-tools/tests/todo_tool_test.rs`

**Dependencies:** None

**Steps:**

1. Create schema:
```rust
pub struct TodoWriteInput {
    pub todos: Vec<TodoItem>,
}

pub struct TodoItem {
    pub content: String,
    pub status: String, // "pending", "in_progress", "completed"
    pub priority: Option<String>, // "high", "medium", "low"
}

pub struct TodoWriteOutput {
    pub todos: Vec<TodoItem>,
    pub updated: usize,
}
```

2. Implement manager with persistence:
```rust
pub struct TodoManager {
    todos: RwLock<Vec<TodoItem>>,
    storage_path: PathBuf,
}

impl TodoManager {
    pub async fn update(&self, todos: Vec<TodoItem>) -> usize;
    pub async fn list(&self) -> Vec<TodoItem>;
    pub async fn save(&self) -> Result<(), TodoError>;
    pub async fn load(&self) -> Result<(), TodoError>;
}
```

3. Write tests

**Verification:** `cargo test --package hcode-tools todo`

---

### Task 4.2: AskUserQuestionTool

**Files:**
- Create: `hcode-tools/src/ask/mod.rs`
- Create: `hcode-tools/src/ask/schema.rs`
- Test: `hcode-tools/tests/ask_tool_test.rs`

**Dependencies:** None

**Steps:**

1. Create schema:
```rust
pub struct AskUserQuestionInput {
    pub question: String,
    pub options: Option<Vec<String>>,
    pub allow_custom: Option<bool>,
}

pub struct AskUserQuestionOutput {
    pub answer: String,
    pub selected_option: Option<usize>,
}
```

2. Implement with async channel:
```rust
impl Tool for AskUserQuestionTool {
    async fn call(&self, input: Value, context: ToolContext) -> Result<ToolResult, ToolError> {
        let params: AskUserQuestionInput = serde_json::from_value(input)?;
        
        // Send question to UI via event channel
        let response = context.ask_channel
            .ok_or(ToolError::NotAvailable)?
            .ask(params.question, params.options).await?;
        
        Ok(ToolResult::success(json!(response)))
    }
}
```

3. Write tests

**Verification:** `cargo test --package hcode-tools ask`

---

### Task 4.3: TaskCreateTool / TaskListTool / TaskGetTool / TaskUpdateTool

**Files:**
- Create: `hcode-tools/src/task/mod.rs`
- Create: `hcode-tools/src/task/schema.rs`
- Create: `hcode-tools/src/task/manager.rs`
- Test: `hcode-tools/tests/task_tools_test.rs`

**Dependencies:** Phase 1 (Coordinator)

**Steps:**

1. Create unified task management
2. Implement CRUD operations
3. Add scheduling capability

**Verification:** `cargo test --package hcode-tools task`

---

### Phase 4 Milestone

- [ ] TodoWriteTool manages todos
- [ ] AskUserQuestionTool prompts users
- [ ] Task tools manage background tasks
- [ ] All tests pass

---

## Phase 5: Intelligence Tools (P2)

**Goal:** Add code intelligence capabilities.

### Task 5.1: LSPTool

**Files:**
- Create: `hcode-tools/src/lsp/mod.rs`
- Create: `hcode-tools/src/lsp/schema.rs`
- Create: `hcode-tools/src/lsp/client.rs`
- Create: `hcode-tools/src/lsp/manager.rs`
- Test: `hcode-tools/tests/lsp_tool_test.rs`

**Dependencies:** Add `lsp-types` crate

**Steps:**

1. Create schema:
```rust
pub struct LspInput {
    pub action: String, // "definition", "references", "hover", "completion", "rename"
    pub file_path: String,
    pub line: usize,
    pub column: usize,
    pub new_name: Option<String>, // For rename
}

pub struct LspOutput {
    pub action: String,
    pub results: Vec<LspResult>,
}

pub struct LspResult {
    pub file_path: String,
    pub line: usize,
    pub column: usize,
    pub text: Option<String>,
}
```

2. Implement LSP client:
```rust
pub struct LspClient {
    process: Child,
    stdin: Box<dyn Write>,
    stdout: BufReader<Box<dyn Read>>,
    request_id: AtomicU64,
}

impl LspClient {
    pub async fn initialize(&mut self, root_path: &Path) -> Result<(), LspError>;
    pub async fn goto_definition(&mut self, file: &Path, line: usize, col: usize) -> Result<Vec<Location>, LspError>;
    pub async fn find_references(&mut self, file: &Path, line: usize, col: usize) -> Result<Vec<Location>, LspError>;
    pub async fn hover(&mut self, file: &Path, line: usize, col: usize) -> Result<Option<Hover>, LspError>;
    pub async fn completion(&mut self, file: &Path, line: usize, col: usize) -> Result<Vec<CompletionItem>, LspError>;
}
```

3. Implement manager for multiple language servers:
```rust
pub struct LspManager {
    clients: HashMap<String, Arc<Mutex<LspClient>>>,
}

impl LspManager {
    pub async fn get_client(&mut self, language: &str, root: &Path) -> Result<Arc<Mutex<LspClient>>, LspError>;
}
```

4. Write tests

**Verification:** `cargo test --package hcode-tools lsp`

---

### Task 5.2: ToolSearchTool

**Files:**
- Create: `hcode-tools/src/tool_search/mod.rs`
- Create: `hcode-tools/src/tool_search/schema.rs`
- Create: `hcode-tools/src/tool_search/searcher.rs`
- Test: `hcode-tools/tests/tool_search_tool_test.rs`

**Dependencies:** None

**Steps:**

1. Implement tool discovery and recommendation based on task description

**Verification:** `cargo test --package hcode-tools tool_search`

---

### Task 5.3: BriefTool

**Files:**
- Create: `hcode-tools/src/brief/mod.rs`
- Test: `hcode-tools/tests/brief_tool_test.rs`

**Dependencies:** None

**Steps:**

1. Implement tool to generate brief summaries of long outputs

**Verification:** `cargo test --package hcode-tools brief`

---

### Phase 5 Milestone

- [ ] LSPTool provides code intelligence
- [ ] ToolSearchTool recommends tools
- [ ] BriefTool summarizes content
- [ ] All tests pass

---

## Phase 6: Additional Providers (P2)

**Goal:** Support multiple LLM providers.

### Task 6.1: OpenAI Provider

**Files:**
- Create: `hcode-provider/src/openai/mod.rs`
- Create: `hcode-provider/src/openai/client.rs`
- Create: `hcode-provider/src/openai/types.rs`
- Create: `hcode-provider/src/openai/stream.rs`
- Test: `hcode-provider/tests/openai_test.rs`

**Dependencies:** None

**Steps:**

1. Implement OpenAI client with streaming
2. Map OpenAI types to hcode-types
3. Support GPT-4, GPT-4o, o1, o3 models

**Verification:** `cargo test --package hcode-provider openai`

---

### Task 6.2: OpenRouter Provider

**Files:**
- Create: `hcode-provider/src/openrouter/mod.rs`
- Test: `hcode-provider/tests/openrouter_test.rs`

**Dependencies:** Task 6.1

**Steps:**

1. Implement OpenRouter client (OpenAI-compatible API)

**Verification:** `cargo test --package hcode-provider openrouter`

---

### Task 6.3: Azure Provider

**Files:**
- Create: `hcode-provider/src/azure/mod.rs`
- Test: `hcode-provider/tests/azure_test.rs`

**Dependencies:** Task 6.1

**Steps:**

1. Implement Azure OpenAI client with AD auth support

**Verification:** `cargo test --package hcode-provider azure`

---

### Phase 6 Milestone

- [ ] OpenAI provider works
- [ ] OpenRouter provider works
- [ ] Azure provider works
- [ ] Provider registry supports all

---

## Phase 7: Commands & UX (P3)

**Goal:** Add slash commands and improved CLI.

### Task 7.1: Command System

**Files:**
- Create: `hcode/src/commands/mod.rs`
- Create: `hcode/src/commands/commit.rs`
- Create: `hcode/src/commands/review.rs`
- Create: `hcode/src/commands/compact.rs`
- Create: `hcode/src/commands/clear.rs`
- Create: `hcode/src/commands/doctor.rs`
- Create: `hcode/src/commands/init.rs`
- Test: `hcode/tests/commands_test.rs`

**Dependencies:** None

**Steps:**

1. Define command trait:
```rust
#[async_trait]
pub trait Command: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn aliases(&self) -> Vec<&str>;
    
    async fn execute(&self, args: Vec<String>, context: CommandContext) -> Result<CommandResult, CommandError>;
}
```

2. Implement command registry:
```rust
pub struct CommandRegistry {
    commands: HashMap<String, Arc<dyn Command>>,
}

impl CommandRegistry {
    pub fn register(&mut self, command: Arc<dyn Command>);
    pub fn get(&self, name: &str) -> Option<Arc<dyn Command>>;
    pub fn list(&self) -> Vec<&str>;
}
```

3. Implement core commands:
   - `/commit` - Generate commit message
   - `/review` - Review current changes
   - `/compact` - Trigger compaction
   - `/clear` - Clear conversation
   - `/doctor` - Diagnose issues
   - `/init` - Initialize project

**Verification:** `cargo test --package hcode commands`

---

### Task 7.2: Improved CLI

**Files:**
- Modify: `hcode/src/interactive.rs`
- Create: `hcode/src/ui/mod.rs`
- Create: `hcode/src/ui/progress.rs`
- Create: `hcode/src/ui/formatting.rs`

**Dependencies:** None

**Steps:**

1. Add progress indicators
2. Improve output formatting
3. Add syntax highlighting
4. Support markdown rendering

**Verification:** Manual testing

---

### Phase 7 Milestone

- [ ] Slash commands work
- [ ] CLI improvements visible
- [ ] All tests pass

---

## Final Verification

After all phases:

```bash
# Build
cargo build --workspace --release

# Test
cargo test --workspace

# Run
./target/release/hcode run -p "Test all features"
```

---

## Notes

### Dependencies to Add

```toml
# Cargo.toml workspace
lsp-types = "0.95"
uuid = { version = "1", features = ["v4", "serde"] }
parking_lot = "0.12"
```

### Breaking Changes

- ToolContext will need optional coordinator/skill_executor references
- QueryEngine will need hook integration

### Future Enhancements (Post-Phase 7)

- Voice integration
- Vim mode
- Computer Use
- GitHub integration
- Telemetry
- Plugin marketplace