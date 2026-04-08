//! Hook executor.

use super::events::{HookConfig, HookEvent, HookResult};
use super::registry::HookRegistry;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// Hook executor.
pub struct HookExecutor {
    registry: Arc<HookRegistry>,
}

impl HookExecutor {
    /// Create a new hook executor.
    pub fn new(registry: Arc<HookRegistry>) -> Self {
        Self { registry }
    }

    /// Execute hooks for an event.
    pub async fn execute(&self, event: HookEvent) -> HookResult {
        let event_name = event.event_name();
        let hooks = self.registry.get_hooks(event_name);
        
        if hooks.is_empty() {
            return HookResult {
                success: true,
                output: None,
                error: None,
                block: false,
            };
        }
        
        let event_json = serde_json::to_string(&event).unwrap_or_default();
        
        for hook in hooks {
            match self.run_hook(hook, &event_json).await {
                Ok(result) => {
                    if result.block {
                        return result;
                    }
                }
                Err(e) => {
                    return HookResult {
                        success: false,
                        output: None,
                        error: Some(e),
                        block: false,
                    };
                }
            }
        }
        
        HookResult {
            success: true,
            output: None,
            error: None,
            block: false,
        }
    }

    /// Run a single hook.
    async fn run_hook(&self, hook: &HookConfig, input: &str) -> Result<HookResult, String> {
        let duration = Duration::from_millis(hook.timeout);
        
        let result = timeout(
            duration,
            async {
                // Build command
                let mut cmd = if cfg!(target_os = "windows") {
                    let mut c = Command::new("cmd");
                    c.args(["/C", &hook.command]);
                    c
                } else {
                    let mut c = Command::new("sh");
                    c.args(["-c", &hook.command]);
                    c
                };
                
                // Set stdin
                cmd.stdin(std::process::Stdio::piped());
                cmd.stdout(std::process::Stdio::piped());
                cmd.stderr(std::process::Stdio::piped());
                
                // Spawn process
                let mut child = cmd.spawn()
                    .map_err(|e| format!("Failed to spawn hook: {}", e))?;
                
                // Write input to stdin
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    write!(stdin, "{}", input)
                        .map_err(|e| format!("Failed to write to hook stdin: {}", e))?;
                }
                
                // Wait for completion
                let output = child.wait_with_output()
                    .await
                    .map_err(|e| format!("Hook execution failed: {}", e))?;
                
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                
                if output.status.success() {
                    // Parse stdout for block signal
                    let block = stdout.contains("BLOCK") || stdout.contains("block: true");
                    
                    Ok(HookResult {
                        success: true,
                        output: Some(stdout),
                        error: None,
                        block,
                    })
                } else {
                    Err(format!("Hook failed: {}", stderr))
                }
            }
        ).await;
        
        match result {
            Ok(Ok(r)) => Ok(r),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(format!("Hook timed out after {}ms", hook.timeout)),
        }
    }
}