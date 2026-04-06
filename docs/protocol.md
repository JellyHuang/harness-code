# XML Notification Protocol

## Overview

The `pkg/protocol` package defines an XML-based notification protocol for inter-agent communication. This protocol is compatible with Claude Code's multi-agent architecture and designed for cross-language implementation.

## Task Notification

### Structure

```xml
<?xml version="1.0" encoding="UTF-8"?>
<task-notification>
  <task-id>agent-1234567890</task-id>
  <status>completed</status>
  <summary>Task completed successfully</summary>
  <result>Task output content here...</result>
  <usage>
    <input-tokens>1500</input-tokens>
    <output-tokens>800</output-tokens>
    <total-tokens>2300</total-tokens>
  </usage>
  <timestamp>1699876543210</timestamp>
  <metadata>
    <entry key="files-affected">src/auth.ts,src/user.ts</entry>
    <entry key="duration-seconds">45</entry>
  </metadata>
</task-notification>
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `task-id` | string | Unique identifier for the task |
| `status` | enum | `completed`, `failed`, `killed`, `in_progress` |
| `summary` | string | Human-readable summary of the task outcome |
| `result` | string | Full result content (may be truncated for display) |
| `usage` | object | Token consumption metrics |
| `timestamp` | int64 | Unix timestamp in milliseconds |
| `metadata` | object | Optional key-value metadata |

### Status Values

```go
const (
    StatusCompleted    TaskStatus = "completed"    // Task finished successfully
    StatusFailed       TaskStatus = "failed"       // Task encountered an error
    StatusKilled       TaskStatus = "killed"       // Task was terminated
    StatusInProgress   TaskStatus = "in_progress"  // Task is still running
)
```

## Agent Message

### Structure

```xml
<?xml version="1.0" encoding="UTF-8"?>
<agent-message from="coordinator" to="worker-1" timestamp="1699876543210" type="shutdown_request">
  Please shut down gracefully
</agent-message>
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `from` | string | Sender agent ID |
| `to` | string | Recipient agent ID |
| `timestamp` | int64 | Unix timestamp in milliseconds |
| `type` | string | Message type (e.g., `shutdown_request`) |
| content | string | Message body |

### Message Types

```go
const (
    MsgTypeShutdownRequest      = "shutdown_request"
    MsgTypeShutdownResponse     = "shutdown_response"
    MsgTypePlanApprovalRequest  = "plan_approval_request"
    MsgTypePlanApprovalResponse = "plan_approval_response"
    MsgTypeCheckpointRequest    = "checkpoint_request"
    MsgTypeCheckpointResponse   = "checkpoint_response"
)
```

## Usage Examples

### Creating a Notification

```go
notification := protocol.NewTaskNotification(
    "agent-123",
    protocol.StatusCompleted,
    "Research completed: found 5 auth patterns",
    "Detailed research findings...",
).WithUsage(1500, 800).
  WithMetadata("files-affected", "src/auth.ts")
```

### Marshaling to XML

```go
xmlData, err := notification.Marshal()
if err != nil {
    return err
}
fmt.Println(string(xmlData))
```

### Unmarshaling from XML

```go
parsed, err := protocol.UnmarshalNotification(xmlData)
if err != nil {
    return err
}
fmt.Printf("Task %s: %s\n", parsed.TaskID, parsed.Status)
```

## Integration with Coordinator

The Coordinator receives notifications through the message queue:

```go
func (c *Coordinator) listenForNotifications() {
    handler := func(msg *messaging.Message) error {
        notification, err := protocol.UnmarshalNotification(msg.Content)
        if err != nil {
            return err
        }
        return c.SendNotification(notification)
    }
    c.queue.Subscribe(c.ctx, c.ID, handler)
}
```

## Cross-Language Compatibility

The XML format is designed for easy implementation in other languages:

- **C++**: Use libxml2 or pugixml
- **Python**: Use xml.etree.ElementTree
- **Rust**: Use serde-xml-rs

### XML Schema (Future)

A formal XSD schema will be provided in `pkg/protocol/schema/xml.xsd` for validation.

## Design Rationale

### Why XML?

1. **Human-readable**: Easy to debug and inspect
2. **Widely supported**: Libraries available in all major languages
3. **Self-describing**: Structure is clear from the document
4. **Compatible with Claude Code**: Matches existing protocol

### Why Not JSON?

- XML provides better attribute/element distinction
- Better support for mixed content
- Consistent with Claude Code's protocol choice