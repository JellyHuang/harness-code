//! SSE (Server-Sent Events) parsing utilities.

/// Parse an SSE line into event type and data.
pub fn parse_sse_line(line: &str) -> Option<(String, String)> {
    if let Some(data) = line.strip_prefix("data: ") {
        Some(("data".to_string(), data.to_string()))
    } else if let Some(event) = line.strip_prefix("event: ") {
        Some(("event".to_string(), event.to_string()))
    } else {
        None
    }
}

/// Check if a line is an SSE event boundary.
pub fn is_event_boundary(line: &str) -> bool {
    line.is_empty()
}

/// SSE event from a stream.
#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event_type: String,
    pub data: String,
}

impl SseEvent {
    pub fn new(event_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            event_type: event_type.into(),
            data: data.into(),
        }
    }
}
