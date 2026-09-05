# HCode 项目开发计划

## 项目概述

本文档对比 **HCode** (Rust实现) 与 **cc-haha** (TypeScript/Claude Code泄露源码) 的功能差异，列出待完成的功能模块。

---

## 一、项目架构对比

### 1.1 当前 HCode 项目结构 (Rust)

```
hcode-rust/
├── hcode-types/         # 核心类型定义
│   └── message.rs       # 消息系统 (~350行) ✅ 完成
├── hcode-protocol/      # 协议层
│   ├── xml.rs          # XML通知解析 ✅ 完成
│   ├── sse.rs          # SSE事件处理 ✅ 完成
│   └── stream_event.rs # 流事件定义 ✅ 完成
├── hcode-provider/      # LLM Provider抽象
│   ├── anthropic/      # Anthropic API ✅ 完成
│   ├── openai/         # OpenAI API ✅ 完成
│   └── registry.rs     # Provider注册 ✅ 完成
├── hcode-tools/         # 工具系统
│   ├── bash/           # Bash工具 ⚠️ 部分完成 (缺安全沙箱)
│   ├── file_read/     # 读取文件 ✅ 完成
│   ├── file_write/    # 写入文件 ✅ 完成
│   ├── file_edit/     # 编辑文件 ✅ 完成
│   ├── glob_tool/     # Glob搜索 ✅ 完成
│   ├── grep_tool/     # Grep搜索 ✅ 完成
│   ├── web_fetch/     # Web抓取 ✅ 完成
│   ├── web_search/     # Web搜索 ✅ 完成
│   ├── agent/         # Agent工具 ✅ 完成
│   ├── send_message/  # 发送消息 ✅ 完成
│   ├── task_stop/     # 停止任务 ✅ 完成
│   ├── task_output/   # 任务输出 ✅ 完成
│   ├── ask/           # 询问用户 ✅ 完成
│   ├── todo/          # Todo工具 ⚠️ 部分完成
│   ├── skill/         # Skill系统 ⚠️ 部分完成
│   ├── lsp/           # LSP支持 ⚠️ 架构完成
│   └── config_tool/   # 配置工具 ⚠️ 部分完成
├── hcode-permission/   # 权限系统 ✅ 基础完成
├── hcode-engine/       # 执行引擎
│   ├── query_engine.rs # 查询引擎 ⚠️ 架构完成，核心逻辑存在
│   ├── state.rs       # 状态机 ✅ 完成
│   ├── coordinator/  # 协调器 ✅ 架构完成
│   ├── worker/        # Worker实现 ✅ 架构完成
│   ├── compact/       # 压缩模块 ✅ 完成
│   ├── stop_hooks/    # 停止钩子 ✅ 完成
│   ├── error_recovery/# 错误恢复 ✅ 完成
│   ├── budget.rs      # 预算跟踪 ✅ 完成
│   └── tool_orchestration/ # 工具编排 ✅ 完成
├── hcode-config/       # 配置系统 ✅ 完成
├── hcode-session/     # 会话管理 ✅ 完成
├── hcode-mcp/         # MCP支持 ⚠️ 基础完成
└── hcode/            # CLI主程序 ✅ 基础完成
```

### 1.2 cc-haha 项目结构 (TypeScript)

```
cc-haha/
├── src/
│   ├── entrypoints/    # 入口点
│   ├── main.tsx       # TUI主逻辑 (804KB)
│   ├── QueryEngine.ts # 查询引擎 (46KB)
│   ├── query.ts       # 查询核心 (68KB)
│   ├── Tool.ts        # 工具基类 (29KB)
│   ├── tools/         # Agent工具 (~30个)
│   │   ├── BashTool/  # Bash工具 (160KB + 安全模块 300KB)
│   │   ├── Chrome/    # Chrome控制
│   │   ├── Desktop/   # 桌面控制
│   │   ├── Config/    # 配置
│   │   ├── LSP/       # LSP支持
│   │   ├── MCP/       # MCP工具
│   │   └── ...        # 更多工具
│   ├── commands/      # 斜杠命令 (~60+个)
│   ├── skills/        # Skill系统
│   ├── services/      # 服务层
│   ├── hooks/         # React hooks
│   └── screens/       # UI界面
```

---

## 二、Bash Command 实现分析 (重点)

### 2.1 HCode (Rust) 当前实现

**文件结构：**
```
hcode-tools/src/bash/
├── mod.rs      (44行)   # 工具定义
├── executor.rs (153行)  # 命令执行
├── sandbox.rs  (13行)   # 沙箱检查 (空实现!)
├── schema.rs   (74行)   # 输入输出Schema
```

**核心实现特点：**
1. **executor.rs (153行)**
   - 使用 `tokio::process::Command` 执行命令
   - Windows上自动检测Git Bash
   - 支持timeout机制
   - 返回stdout/stderr/exit_code

2. **sandbox.rs (13行)**
   - 占位符实现 `is_sandbox_enabled()` 返回 `false`
   - `validate_command()` 无任何检查

3. **schema.rs (74行)**
   - `BashInput`: command, workdir, timeout, run_in_background
   - `BashOutput`: stdout, stderr, exit_code, timed_out

### 2.2 cc-haha (TypeScript) 实现

**文件结构：**
```
tools/BashTool/
├── BashTool.tsx           (160KB)   # 主实现
├── bashSecurity.ts        (102KB)   # 安全检查
├── bashPermissions.ts      (98KB)    # 权限管理
├── pathValidation.ts      (43KB)    # 路径验证
├── readOnlyValidation.ts  (68KB)    # 只读验证
├── sedValidation.ts       (21KB)   # sed命令验证
├── bashCommandHelpers.ts  (8KB)     # 命令帮助
├── shouldUseSandbox.ts    (5KB)     # 沙箱决策
├── UI.tsx                 (25KB)   # UI渲染
└── utils.ts               (5KB)    # 工具函数
```

**cc-haha 完整功能：**

1. **命令语义分析** (~200行)
   - 搜索命令识别: find, grep, rg, ag, ack, locate, which
   - 读取命令识别: cat, head, tail, less, more, wc, stat, file
   - 列表命令识别: ls, tree, du
   - 静默命令识别: mv, cp, rm, mkdir, chmod, chown, touch, ln

2. **安全沙箱系统** (~300KB代码)
   - bashSecurity.ts: 完整的安全检查逻辑
   - bashPermissions.ts: 权限规则系统
   - 危险命令拦截 (rm -rf等)

3. **路径验证** (43KB)
   - 项目边界检查
   - 路径遍历防护
   - 符号链接处理

4. **只读约束** (68KB)
   - 只读模式下禁止写入
   - 配置文件检查

5. **sed命令特殊处理** (21KB)
   - sed编辑命令解析
   - 安全性验证

6. **后台任务支持**
   - LocalShellTask集成
   - 进度显示
   - 任务队列管理

### 2.3 Bash工具差距分析

| 功能 | cc-haha | HCode | 状态 |
|------|---------|-------|------|
| 命令执行 | ✅ | ✅ | 完成 |
| 超时控制 | ✅ | ✅ | 完成 |
| 工作目录 | ✅ | ✅ | 完成 |
| 后台执行 | ✅ | ⚠️ | 架构存在，需完善 |
| 安全沙箱 | ✅ | ❌ | 未实现 |
| 路径验证 | ✅ | ❌ | 未实现 |
| 权限控制 | ✅ | ❌ | 未实现 |
| 只读检查 | ✅ | ❌ | 未实现 |
| 命令语义分析 | ✅ | ❌ | 未实现 |
| sed验证 | ✅ | ❌ | 未实现 |
| UI显示 | ✅ | ❌ | 无复杂UI |

---

## 三、功能对比与待完成项

### 3.1 核心查询引擎 (QueryEngine)

| 功能 | cc-haha | HCode | 状态 |
|------|---------|-------|------|
| 消息提交 | ✅ | ✅ | 完成 |
| 流式API调用 | ✅ | ✅ | 完成 |
| 状态机转换 | ✅ | ✅ | 完成 |
| Tool执行 | ✅ | ✅ | 完成 |
| Stop Hooks | ✅ | ✅ | 完成 |
| 错误恢复 | ✅ | ✅ | 完成 |
| 预算跟踪 | ✅ | ✅ | 完成 |
| 对话压缩 | ✅ | ✅ | 完成 |

**🔴 优先级P0 - 核心引擎 (已完成)**
- [x] QueryEngine 核心流式逻辑 (~1150行)
- [x] SSE 事件处理
- [x] 工具执行完整流程
- [x] 错误恢复机制

---

### 3.2 工具系统 (Tools)

#### 已完成 ✅
- Bash (基础执行)
- FileRead
- FileWrite
- FileEdit
- Glob
- Grep
- WebFetch
- WebSearch
- Agent (子Agent)
- SendMessage
- TaskStop
- TaskOutput
- AskUser

#### 缺失功能 ❌
| 工具 | cc-haha | 说明 |
|------|---------|------|
| Chrome | ✅ | Chrome控制 |
| Desktop | ✅ | 桌面控制 (Computer Use) |
| Brief | ✅ | 摘要工具 |
| EnterPlanMode | ✅ | 进入计划模式 |
| ExitPlanMode | ✅ | 退出计划模式 |
| EnterWorktree | ✅ | 进入git worktree |
| ExitWorktree | ✅ | 退出git worktree |
| Memory | ✅ | 记忆系统 |
| MCP Tools | ✅ | MCP工具 |

#### 需要完善 ⚠️
| 工具 | 当前状态 | 需完善 |
|------|---------|--------|
| Bash | 基础执行 | 需实现安全沙箱、路径验证、sed验证 |
| Todo | 部分完成 | 需完善 |
| Skill | 部分完成 | 需完善执行器 |
| LSP | 架构 | 需完善功能 |
| Config | 部分 | 需完善 |

**🔴 优先级P1 - 工具扩展**
- [ ] 实现Bash完整安全沙箱 (~300行)
- [ ] 实现路径验证 (~200行)
- [ ] 实现只读约束检查 (~100行)
- [ ] 实现sed命令验证 (~100行)
- [ ] 完善TodoWrite工具
- [ ] 完善LSP工具
- [ ] 集成MCP Tools

---

### 3.3 斜杠命令 (Slash Commands)

#### cc-haha 命令列表 (~60+个)
```
/add-dir          /agents           /ant-trace        /autofix-pr
/backfill-sessions /branch          /break-cache      /bridge
/buddy            /bughunter        /chrome           /clear
/color            /commit           /compact         /config
/context          /copy             /cost            /ctx_viz
/debug-tool-call  /desktop         /diff            /doctor
/effort           /env              /exit            /export
/extra-usage     /fast             /feedback        /files
/good-claude    /heapdump         /help            /hooks
/ide              /install-github-app /install-slack-app /issue
/keybindings     /login            /logout          /mcp
/memory          /mobile           /model           /oauth-refresh
/onboarding      /output-style     /passes          /perf-issue
/permissions     /plan             /plugin          /pr_comments
/privacy-settings /rate-limit-options /release-notes /reload-plugins
/remote-env       /remote-setup    /rename          /reset-limits
/resume          /review           /rewind          /sandbox-toggle
/session         /share            /skills          /stats
/status          /stickers         /summary         /tag
/tasks           /teleport         /terminalSetup   /theme
/thinkback       /thinkback-play   /upgrade         /usage
/vim             /voice            /btw
```

#### 当前 HCode 实现 (~5个)
| 命令 | 状态 |
|------|------|
| /help | ✅ |
| /exit | ✅ |
| /clear | ✅ |
| /compact | ✅ (架构) |
| /doctor | ✅ |
| /mcp | ⚠️ (CLI存在，需完善) |

**🔴 优先级P2 - 命令系统**
- [ ] /config - 配置查看/编辑
- [ ] /commit - Git提交
- [ ] /branch - 分支管理
- [ ] /diff - 显示差异
- [ ] /status - Git状态
- [ ] /agents - 列出Agent
- [ ] /session - 会话管理
- [ ] /memory - 记忆管理
- [ ] /plan - 计划模式
- [ ] /review - 代码审查
- [ ] /stats - 使用统计

---

### 3.4 Skills 系统

#### cc-haha 技能 (~15个)
- batch.ts
- claudeApi.ts
- debug.ts
- keybindings.ts
- loop.ts
- remember.ts
- scheduleRemoteAgents.ts
- simplify.ts
- skillify.ts
- stuck.ts
- updateConfig.ts
- verify.ts

#### HCode 当前状态
- [x] skill/loader.rs - 技能加载器
- [x] skill/schema.rs - 技能定义
- [ ] skill/executor.rs - 技能执行器

**🔴 优先级P2 - Skills**
- [ ] 实现 SkillExecutor
- [ ] 迁移 cc-haha 内置技能
- [ ] 实现自定义技能加载

---

### 3.5 MCP 支持

| 功能 | cc-haha | HCode |
|------|---------|-------|
| MCP Client | ✅ | ⚠️ 基础完成 |
| MCP Servers | ✅ | ❌ |
| MCP Tools | ✅ | ❌ |

**🔴 优先级P1 - MCP**
- [ ] 完善 hcode-mcp 客户端
- [ ] 实现 MCP Server 发现与连接
- [ ] MCP Tools 到 HCode Tools 映射

---

### 3.6 UI/交互界面

#### cc-haha 特点
- 完整 Ink TUI 界面 (React + Ink)
- 彩色终端输出
- 实时流式显示
- 动画效果
- 进度提示

#### HCode 当前
- 基础 CLI 输出
- 简单交互
- 无复杂 UI

**🔴 优先级P3 - UI增强**
- [ ] 彩色输出 (ansi_term)
- [ ] 进度条显示
- [ ] 动画效果
- [ ] 更好的错误提示

---

### 3.7 Provider 支持

#### HCode 已完成 ✅
- Anthropic (Claude)
- OpenAI (GPT)

#### cc-haha 支持
- Anthropic 官方
- MiniMax
- OpenRouter
- 自定义端点

**🔴 优先级P2 - Provider扩展**
- [ ] 添加 OpenRouter 支持
- [ ] 添加 Azure OpenAI 支持
- [ ] 添加 Bedrock 支持
- [ ] 添加自定义端点配置

---

## 四、实现优先级排序

### Phase 1: Bash安全增强 (P0) ⚠️ 重要
```
1. Bash安全沙箱
   - 实现 sandbox.rs 核心逻辑
   - 危险命令拦截
   - 命令白名单

2. 路径验证
   - 项目边界检查
   - 路径遍历防护

3. 只读约束
   - 只读模式检查

4. sed命令验证
   - 解析sed命令
   - 安全性检查
```

### Phase 2: 工具与命令 (P1)
```
5. Todo 工具完善
6. Config 工具
7. LSP 工具完善
8. MCP 集成
```

### Phase 3: 扩展功能 (P2)
```
9. 斜杠命令补全 (/config, /commit, /diff等)
10. Skills 系统完善
11. Provider 扩展
```

### Phase 4: UI增强 (P3)
```
12. 彩色输出
13. 进度显示
14. TUI 改进
```

---

## 五、技术实现参考

### 5.1 Bash安全参考实现

**目录**: `thirdparty/cc-haha-main/src/tools/BashTool/`

关键文件：
- `bashSecurity.ts` - 安全检查逻辑
- `bashPermissions.ts` - 权限规则
- `pathValidation.ts` - 路径验证
- `readOnlyValidation.ts` - 只读检查
- `sedValidation.ts` - sed验证
- `shouldUseSandbox.ts` - 沙箱决策

### 5.2 命令参考

**目录**: `thirdparty/cc-haha-main/src/commands/`

---

## 六、开发估算

| 模块 | 预计行数 | 预计时间 |
|------|---------|---------|
| Bash安全沙箱 | ~500行 | 3-5天 |
| 路径验证 | ~200行 | 1-2天 |
| 工具扩展 | ~500行 | 3-5天 |
| 命令系统 | ~400行 | 2-3天 |
| Skills | ~300行 | 2天 |
| MCP | ~300行 | 2-3天 |
| UI增强 | ~200行 | 1-2天 |

**总计**: ~2400行代码，约14-22天

---

## 七、下一步行动

### 立即执行
1. ✅ 阅读本文档
2. ✅ 确认优先级
3. 开始实现 Phase 1: Bash安全沙箱

### 推荐实现顺序
1. 完善 bash/sandbox.rs 核心逻辑
2. 添加危险命令拦截
3. 添加路径验证
4. 添加只读检查
5. 添加sed验证
6. 完善其他工具

---

*文档生成时间: 2025*
*对比版本: HCode (Rust) vs cc-haha (TypeScript/Claude Code)*
*本版本更新了Bash工具的详细实现分析*