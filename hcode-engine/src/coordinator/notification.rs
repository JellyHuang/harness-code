//! XML notification formatting.

use super::worker_registry::{WorkerNotification, WorkerStatus};

/// Format a worker notification as XML.
pub fn format_xml_notification(notification: &WorkerNotification) -> String {
    let status = notification.event.status();
    let summary = notification.event.summary();
    
    // Escape XML special characters
    let summary = escape_xml(&summary);
    
    format!(
        r#"<task-notification>
  <task-id>{}</task-id>
  <status>{}</status>
  <summary>{}</summary>
  <timestamp>{}</timestamp>
</task-notification>"#,
        notification.worker_id,
        status,
        summary,
        notification.timestamp.to_rfc3339()
    )
}

/// Format a worker status update as XML.
pub fn format_status_xml(worker_id: &str, status: &WorkerStatus, result: Option<&str>, error: Option<&str>) -> String {
    let result_xml = result
        .map(|r| format!("\n  <result>{}</result>", escape_xml(r)))
        .unwrap_or_default();
    
    let error_xml = error
        .map(|e| format!("\n  <error>{}</error>", escape_xml(e)))
        .unwrap_or_default();
    
    format!(
        r#"<task-notification>
  <task-id>{}</task-id>
  <status>{}</status>{}{}
</task-notification>"#,
        worker_id,
        status,
        result_xml,
        error_xml
    )
}

/// Escape XML special characters.
pub fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_format_notification() {
        let notification = WorkerNotification {
            worker_id: "test-123".to_string(),
            event: super::NotificationEvent::Started,
            timestamp: Utc::now(),
        };

        let xml = format_xml_notification(&notification);
        assert!(xml.contains("<task-id>test-123</task-id>"));
        assert!(xml.contains("<status>started</status>"));
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("<script>"), "&lt;script&gt;");
        assert_eq!(escape_xml("a & b"), "a &amp; b");
    }
}