//! XML serialization for task notifications.

use hcode_types::{TaskNotification, TaskStatus};

/// XML tag constants matching cc-haha-main.
pub const TASK_NOTIFICATION_TAG: &str = "task-notification";
pub const TASK_ID_TAG: &str = "task-id";
pub const STATUS_TAG: &str = "status";
pub const SUMMARY_TAG: &str = "summary";
pub const RESULT_TAG: &str = "result";
pub const USAGE_TAG: &str = "usage";
pub const TOTAL_TOKENS_TAG: &str = "total-tokens";
pub const TOOL_USES_TAG: &str = "tool-uses";
pub const DURATION_MS_TAG: &str = "duration-ms";

/// Serialize TaskNotification to XML.
pub fn task_notification_to_xml(notification: &TaskNotification) -> String {
    let mut result = String::new();

    result.push_str(&format!("<{}>\n", TASK_NOTIFICATION_TAG));
    result.push_str(&format!(
        "  <{}>{}</{}>\n",
        TASK_ID_TAG,
        escape_xml(&notification.task_id),
        TASK_ID_TAG
    ));
    result.push_str(&format!(
        "  <{}>{}</{}>\n",
        STATUS_TAG,
        status_to_string(&notification.status),
        STATUS_TAG
    ));
    result.push_str(&format!(
        "  <{}>{}</{}>\n",
        SUMMARY_TAG,
        escape_xml(&notification.summary),
        SUMMARY_TAG
    ));

    if let Some(ref r) = notification.result {
        result.push_str(&format!(
            "  <{}>{}</{}>\n",
            RESULT_TAG,
            escape_xml(r),
            RESULT_TAG
        ));
    }

    if let Some(ref usage) = notification.usage {
        result.push_str(&format!("  <{}>\n", USAGE_TAG));
        if let Some(tokens) = usage.total_tokens {
            result.push_str(&format!(
                "    <{}>{}</{}>\n",
                TOTAL_TOKENS_TAG, tokens, TOTAL_TOKENS_TAG
            ));
        }
        if let Some(tools) = usage.tool_uses {
            result.push_str(&format!(
                "    <{}>{}</{}>\n",
                TOOL_USES_TAG, tools, TOOL_USES_TAG
            ));
        }
        result.push_str(&format!("  </{}>\n", USAGE_TAG));
    }

    if let Some(duration) = notification.duration_ms {
        result.push_str(&format!(
            "  <{}>{}</{}>\n",
            DURATION_MS_TAG, duration, DURATION_MS_TAG
        ));
    }

    result.push_str(&format!("</{}>", TASK_NOTIFICATION_TAG));
    result
}

/// Parse TaskNotification from XML.
pub fn task_notification_from_xml(xml: &str) -> Option<TaskNotification> {
    // Simple XML parsing - in production, use proper XML parser
    let task_id = extract_tag(xml, TASK_ID_TAG)?;
    let status_str = extract_tag(xml, STATUS_TAG)?;
    let summary = extract_tag(xml, SUMMARY_TAG)?;

    let status = match status_str.as_str() {
        "completed" => TaskStatus::Completed,
        "failed" => TaskStatus::Failed,
        "killed" => TaskStatus::Killed,
        "in_progress" => TaskStatus::InProgress,
        _ => TaskStatus::Completed,
    };

    let result = extract_tag(xml, RESULT_TAG);
    let duration_ms = extract_tag(xml, DURATION_MS_TAG).and_then(|s| s.parse().ok());

    Some(TaskNotification {
        task_id,
        status,
        summary,
        result,
        usage: None,
        duration_ms,
    })
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);

    let start = xml.find(&start_tag)? + start_tag.len();
    let end = xml.find(&end_tag)?;

    Some(xml[start..end].to_string())
}

fn status_to_string(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
        TaskStatus::Killed => "killed",
        TaskStatus::InProgress => "in_progress",
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
