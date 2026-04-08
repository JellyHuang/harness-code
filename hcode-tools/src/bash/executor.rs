//! Bash command executor.

use super::schema::{BashInput, BashOutput, DEFAULT_TIMEOUT_MS, MAX_TIMEOUT_MS};
use crate::{ToolContext, ToolError};
use hcode_types::ToolResult;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Detect Git Bash path on Windows.
/// Returns the path to bash.exe if found, None otherwise.
#[cfg(target_os = "windows")]
fn find_git_bash() -> Option<std::path::PathBuf> {
    // Common Git Bash installation paths
    let common_paths = [
        r"C:\Program Files\Git\bin\bash.exe",
        r"C:\Program Files (x86)\Git\bin\bash.exe",
    ];

    for path in &common_paths {
        let bash_path = std::path::PathBuf::from(path);
        if bash_path.exists() {
            return Some(bash_path);
        }
    }

    // Try to expand environment variables in paths
    let env_paths = [
        r"%PROGRAMFILES%\Git\bin\bash.exe",
        r"%PROGRAMFILES(X86)%\Git\bin\bash.exe",
    ];

    for path in &env_paths {
        if let Ok(expanded) = shellexpand::env(path) {
            let bash_path = std::path::PathBuf::from(expanded.as_ref());
            if bash_path.exists() {
                return Some(bash_path);
            }
        }
    }

    // Also check if bash is in PATH
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(';') {
            let bash_exe = std::path::PathBuf::from(dir).join("bash.exe");
            if bash_exe.exists() {
                return Some(bash_exe);
            }
        }
    }

    None
}

/// Determine which shell to use and build the appropriate command.
///
/// Strategy:
/// - On Unix: use `sh -c`
/// - On Windows:
///   1. Prefer Git Bash if available (for true bash commands)
///   2. Fallback to cmd.exe /C
#[cfg(target_os = "windows")]
fn build_command(command: &str, workdir: &std::path::Path) -> Command {
    // Try to find Git Bash
    if let Some(bash_path) = find_git_bash() {
        let mut cmd = Command::new(&bash_path);
        cmd.args(["-c", command]);
        cmd.current_dir(workdir);
        // Set MSYSTEM to enable POSIX path conversion
        cmd.env("MSYSTEM", "MINGW64");
        cmd.env("CHERE_INVOKING", "1"); // Preserve current directory
        return cmd;
    }

    // Fallback to cmd.exe
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", command]);
    cmd.current_dir(workdir);
    cmd
}

/// Build shell command on Unix systems.
#[cfg(not(target_os = "windows"))]
fn build_command(command: &str, workdir: &std::path::Path) -> Command {
    let mut cmd = Command::new("sh");
    cmd.args(["-c", command]);
    cmd.current_dir(workdir);
    cmd
}

/// Execute a bash command.
///
/// On Windows, this will:
/// 1. Try to use Git Bash if installed (for true bash commands like `ls`, `grep`, etc.)
/// 2. Fallback to cmd.exe if Git Bash is not available
///
/// On Unix, uses the standard `sh -c` approach.
pub async fn execute(input: BashInput, context: ToolContext) -> Result<ToolResult, ToolError> {
    // Validate timeout
    let timeout_ms = input.timeout.unwrap_or(DEFAULT_TIMEOUT_MS);
    if timeout_ms > MAX_TIMEOUT_MS {
        return Err(ToolError::InvalidInput(format!(
            "Timeout exceeds maximum of {}ms",
            MAX_TIMEOUT_MS
        )));
    }

    // Determine working directory
    let workdir = input
        .workdir
        .map(|p| p.into())
        .unwrap_or(context.working_dir.clone());

    // Build command using platform-specific logic
    let mut cmd = build_command(&input.command, &workdir);

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
                })
                .unwrap(),
            ))
        }
        Ok(Err(e)) => Err(ToolError::Execution(format!(
            "Failed to execute command: {}",
            e
        ))),
        Err(_) => {
            // Timeout elapsed
            Ok(ToolResult::success(
                serde_json::to_value(BashOutput {
                    stdout: String::new(),
                    stderr: format!("Command timed out after {}ms", timeout_ms),
                    exit_code: -1,
                    timed_out: true,
                })
                .unwrap(),
            ))
        }
    }
}
