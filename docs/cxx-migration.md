# C++ Migration Outline

## Overview

This document outlines the planned migration of core `pkg/*` modules from Go to C++. The migration preserves the module boundaries defined in the current Go Workspace structure.

## Migration Goals

1. **Performance**: Native C++ for compute-intensive operations
2. **Portability**: Compile to WebAssembly for browser deployment
3. **Interoperability**: FFI bindings for Python, Node.js, Rust
4. **Independence**: Each module remains independently usable

## Module Migration Mapping

| Go Module | C++ Target | Priority | Notes |
|-----------|------------|----------|-------|
| `pkg/protocol` | `libhcode_protocol` | High | No dependencies, stable API |
| `pkg/llm` | `libhcode_llm` | High | HTTP client, provider abstraction |
| `pkg/tools` | `libhcode_tools` | Medium | Tool implementations |
| `pkg/agent` | `libhcode_agent` | Medium | Definitions, registry |
| `pkg/core` | `libhcode_core` | Low | Composition layer |

## Proposed Directory Structure

```
hcode-cpp/
├── CMakeLists.txt
├── thirdparty/              # External dependencies
│   ├── httplib/            # HTTP client
│   ├── nlohmann-json/      # JSON parsing
│   └── pugixml/            # XML parsing
│
├── include/
│   ├── protocol/
│   │   ├── notification.hpp
│   │   └── message.hpp
│   ├── llm/
│   │   ├── provider.hpp
│   │   └── providers/
│   │       ├── anthropic.hpp
│   │       └── openai.hpp
│   ├── tools/
│   │   ├── registry.hpp
│   │   └── builtin.hpp
│   └── agent/
│       ├── definition.hpp
│       └── registry.hpp
│
├── src/
│   ├── protocol/
│   │   ├── notification.cpp
│   │   └── message.cpp
│   ├── llm/
│   │   ├── provider.cpp
│   │   └── providers/
│   │       ├── anthropic.cpp
│   │       └── openai.cpp
│   ├── tools/
│   │   ├── registry.cpp
│   │   └── builtin/
│   │       ├── bash.cpp
│   │       ├── read.cpp
│   │       └── edit.cpp
│   └── agent/
│       ├── definition.cpp
│       └── registry.cpp
│
├── tests/
│   ├── protocol/
│   ├── llm/
│   └── tools/
│
└── bindings/
    ├── python/            # pybind11 bindings
    ├── node/              # napi bindings
    └── wasm/              # Emscripten output
```

## Phase 1: Protocol Module (Priority: High)

### Rationale
- No external dependencies
- Stable, simple API
- Foundation for other modules

### API Example

```cpp
// include/protocol/notification.hpp
namespace hcode::protocol {

enum class TaskStatus {
    Completed,
    Failed,
    Killed,
    InProgress
};

struct TokenUsage {
    int input_tokens;
    int output_tokens;
    int total_tokens;
};

class TaskNotification {
public:
    TaskNotification(std::string task_id, TaskStatus status,
                     std::string summary, std::string result);
    
    TaskNotification& with_usage(int input, int output);
    TaskNotification& with_metadata(std::string key, std::string value);
    
    std::string marshal() const;
    static TaskNotification unmarshal(const std::string& xml);
    
    // Getters
    const std::string& task_id() const;
    TaskStatus status() const;
    bool is_terminal() const;
    
private:
    std::string task_id_;
    TaskStatus status_;
    std::string summary_;
    std::string result_;
    TokenUsage usage_;
    int64_t timestamp_;
    std::map<std::string, std::string> metadata_;
};

} // namespace hcode::protocol
```

## Phase 2: LLM Module (Priority: High)

### Dependencies
- `httplib` for HTTP
- `nlohmann/json` for JSON

### API Example

```cpp
// include/llm/provider.hpp
namespace hcode::llm {

struct Message {
    std::string role;
    std::string content;
};

struct Tool {
    std::string name;
    std::string description;
    nlohmann::json parameters;
};

struct Response {
    std::string content;
    std::vector<ToolCall> tool_calls;
    std::optional<Usage> usage;
};

class Provider {
public:
    virtual ~Provider() = default;
    
    virtual Response complete(
        const std::vector<Message>& messages,
        const std::vector<Tool>& tools
    ) = 0;
    
    virtual std::string name() const = 0;
};

class AnthropicProvider : public Provider {
public:
    AnthropicProvider(std::string api_key, std::string model);
    
    Response complete(
        const std::vector<Message>& messages,
        const std::vector<Tool>& tools
    ) override;
    
    std::string name() const override { return "anthropic"; }
};

} // namespace hcode::llm
```

## Phase 3: Tools Module (Priority: Medium)

### Platform Considerations

```cpp
// Cross-platform process execution
#ifdef _WIN32
    // Use CreateProcess on Windows
#else
    // Use fork/exec on Unix
#endif
```

### API Example

```cpp
// include/tools/registry.hpp
namespace hcode::tools {

class BaseTool {
public:
    virtual ~BaseTool() = default;
    
    virtual ToolInfo info() const = 0;
    virtual ToolResponse run(const std::string& input) = 0;
};

class Registry {
public:
    void register_tool(std::unique_ptr<BaseTool> tool);
    BaseTool* get(const std::string& name) const;
    std::vector<std::string> list() const;
    ToolResponse execute(const std::string& name, const std::string& input);
    
private:
    std::map<std::string, std::unique_ptr<BaseTool>> tools_;
};

} // namespace hcode::tools
```

## Phase 4: Agent Module (Priority: Medium)

### Definition

```cpp
// include/agent/definition.hpp
namespace hcode::agent {

enum class PermissionMode {
    ReadOnly,
    AcceptEdits,
    Plan
};

struct Definition {
    std::string agent_type;
    std::string name;
    std::string description;
    PermissionMode permission_mode;
    std::vector<std::string> allowed_tools;
    std::vector<std::string> disallowed_tools;
    
    bool is_tool_allowed(const std::string& tool_name) const;
};

class Registry {
public:
    void load_from_directory(const std::filesystem::path& dir);
    void register_definition(Definition def);
    const Definition* get(const std::string& type) const;
    std::vector<std::string> list() const;
    
private:
    std::map<std::string, Definition> agents_;
};

} // namespace hcode::agent
```

## Interoperability Layer

### Go CGO Bindings

```go
/*
#cgo LDFLAGS: -lhcode_protocol -lhcode_llm
#include <hcode/protocol/notification.hpp>
#include <hcode/llm/provider.hpp>
*/
import "C"

// Go wrapper for C++ library
```

### Python Bindings (pybind11)

```python
# bindings/python/hcode.cpp
#include <pybind11/pybind11.h>
#include <hcode/protocol/notification.hpp>

namespace py = pybind11;

PYBIND11_MODULE(hcode, m) {
    py::enum_<hcode::protocol::TaskStatus>(m, "TaskStatus")
        .value("Completed", hcode::protocol::TaskStatus::Completed)
        .value("Failed", hcode::protocol::TaskStatus::Failed);
    
    py::class_<hcode::protocol::TaskNotification>(m, "TaskNotification")
        .def(py::init<std::string, TaskStatus, std::string, std::string>())
        .def("marshal", &hcode::protocol::TaskNotification::marshal)
        .def("is_terminal", &hcode::protocol::TaskNotification::is_terminal);
}
```

## Build System

### CMakeLists.txt

```cmake
cmake_minimum_required(VERSION 3.20)
project(hcode VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Dependencies
find_package(htplib REQUIRED)
find_package(nlohmann_json REQUIRED)

# Libraries
add_subdirectory(src/protocol)  # libhcode_protocol
add_subdirectory(src/llm)       # libhcode_llm
add_subdirectory(src/tools)     # libhcode_tools
add_subdirectory(src/agent)     # libhcode_agent

# Bindings (optional)
option(BUILD_PYTHON_BINDINGS "Build Python bindings" OFF)
if(BUILD_PYTHON_BINDINGS)
    add_subdirectory(bindings/python)
endif()
```

## Timeline Estimate

| Phase | Module | Duration | Dependencies |
|-------|--------|----------|--------------|
| 1 | pkg/protocol | 2-3 weeks | None |
| 2 | pkg/llm | 3-4 weeks | httplib, JSON |
| 3 | pkg/tools | 4-5 weeks | Platform-specific |
| 4 | pkg/agent | 2-3 weeks | YAML parser |
| 5 | Bindings | 2-3 weeks | pybind11, napi |
| 6 | Integration | 2-3 weeks | Go CGO |

**Total Estimate**: 15-21 weeks for core migration

## Testing Strategy

1. **Unit Tests**: Each C++ module has corresponding test suite
2. **Integration Tests**: Cross-language compatibility tests
3. **Performance Benchmarks**: Compare Go vs C++ performance
4. **Fuzzing**: Protocol parsing security testing

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| API divergence | Maintain shared protocol specification |
| Memory safety | Use RAII, smart pointers, sanitizers |
| Build complexity | Provide Conan/vcpkg recipes |
| Platform differences | CI on Linux, macOS, Windows |

## Conclusion

The C++ migration preserves the modular architecture while enabling:
- Native performance for critical paths
- WebAssembly deployment for browser-based tools
- Language-agnostic protocol layer
- Gradual migration without breaking existing Go implementation