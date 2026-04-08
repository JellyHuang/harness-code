//! Bash command executor.

use super::schema::{BashInput, BashOutput, DEFAULT_TIMEOUT_MS, MAX_TIMEOUT_MS};
use crate::{ToolContext, ToolError};
use hcode_types::ToolResult;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Execute a bash command.
pub async fn execute(input: BashInput, context: ToolContext) -> Result<ToolResult, ToolError> {
    // Validate timeout
    let timeout_ms = input.timeout.unwrap_or(DEFAULT_TIMEOUT_MS);
    if timeout_ms > MAX_TIMEOUT_MS {
        return Err(ToolError::InvalidInput(
            format!("Timeout exceeds maximum of {}ms", MAX_TIMEOUT_MS)
        ));
    }

    // Determine working directory
    let workdir = input.workdir
        .map(|p| p.into())
        .unwrap_or(context.working_dir.clone());

    // Build command
    // On Windows, use cmd.exe /C; on Unix, use sh -c
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", &input.command]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", &input.command]);
        c
    };

    cmd.current_dir(&workdir);

    // Execute with timeout
    let duration = Duration::from_millis(timeout_ms);
    let result = timeout(duration, cmd.output()).await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(-1);

            Ok(ToolResult::success(
                serde_json::to_value(BashOutput {
                    stdout,
                    stderr,
                    exit_code,
                    timed_out: false,
                }).unwrap()
            ))
        }
        Ok(Err(e)) => {
            Err(ToolError::Execution(format!("Failed to execute command: {}", e)))
        }
        Err(_) => {
            // Timeout elapsed
            Ok(ToolResult::success(
                serde_json::to_value(BashOutput {
                    stdout: String::new(),
                    stderr: format!("Command timed out after {}ms", timeout_ms),
                    exit_code: -1,
                    timed_out: true,
                }).unwrap()
            ))
        }
    }
}