//! Workflow hooks - Run commands on success or failure
//!
//! Provides --on-success and --on-failure hooks for command execution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

/// Hook configuration
#[derive(Debug, Clone, Default)]
pub struct HooksConfig {
    /// Command to run on success
    pub on_success: Option<String>,
    /// Command to run on failure
    pub on_failure: Option<String>,
}

/// Result of running a hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    /// Which hook was run
    pub hook: String,
    /// Whether the hook succeeded
    pub success: bool,
    /// Hook command that was run
    pub command: String,
    /// Output from the hook
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Error from the hook if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl HooksConfig {
    /// Create a new hooks config from command line args
    pub const fn from_args(on_success: Option<String>, on_failure: Option<String>) -> Self {
        Self {
            on_success,
            on_failure,
        }
    }

    /// Check if any hooks are configured
    pub const fn has_hooks(&self) -> bool {
        self.on_success.is_some() || self.on_failure.is_some()
    }

    /// Run the appropriate hook based on result
    pub async fn run_hook(&self, success: bool) -> Option<HookResult> {
        let (hook_name, hook_cmd) = if success {
            ("on_success", &self.on_success)
        } else {
            ("on_failure", &self.on_failure)
        };

        if let Some(cmd) = hook_cmd.as_ref() {
            Some(run_hook_command(hook_name, cmd).await)
        } else {
            None
        }
    }
}

/// Run a hook command
async fn run_hook_command(hook_name: &str, command: &str) -> HookResult {
    // Run the command through the shell
    let result = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", command]).output().await
    } else {
        Command::new("sh").args(["-c", command]).output().await
    };

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            if output.status.success() {
                HookResult {
                    hook: hook_name.to_string(),
                    success: true,
                    command: command.to_string(),
                    output: if stdout.is_empty() {
                        None
                    } else {
                        Some(stdout)
                    },
                    error: None,
                }
            } else {
                HookResult {
                    hook: hook_name.to_string(),
                    success: false,
                    command: command.to_string(),
                    output: if stdout.is_empty() {
                        None
                    } else {
                        Some(stdout)
                    },
                    error: Some(if stderr.is_empty() {
                        format!(
                            "Hook exited with code: {}",
                            output.status.code().unwrap_or(-1)
                        )
                    } else {
                        stderr
                    }),
                }
            }
        }
        Err(e) => HookResult {
            hook: hook_name.to_string(),
            success: false,
            command: command.to_string(),
            output: None,
            error: Some(format!("Failed to execute hook: {e}")),
        },
    }
}

/// Wrapper to execute a command with hooks
#[allow(dead_code)]
pub async fn with_hooks<F, Fut>(hooks: &HooksConfig, f: F) -> Result<()>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let result = f().await;
    let success = result.is_ok();

    // Run the appropriate hook
    // Hook results are tracked in HookResult and can be handled by caller if needed
    let _ = hooks.run_hook(success).await;

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hooks_config_default() {
        let config = HooksConfig::default();
        assert!(!config.has_hooks());
    }

    #[test]
    fn test_hooks_config_with_success() {
        let config = HooksConfig::from_args(Some("echo success".to_string()), None);
        assert!(config.has_hooks());
    }

    #[test]
    fn test_hooks_config_with_failure() {
        let config = HooksConfig::from_args(None, Some("echo failed".to_string()));
        assert!(config.has_hooks());
    }

    #[test]
    fn test_hook_result_serialization() -> Result<()> {
        let result = HookResult {
            hook: "on_success".to_string(),
            success: true,
            command: "echo test".to_string(),
            output: Some("test\n".to_string()),
            error: None,
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"hook\":\"on_success\""));
        Ok(())
    }

    #[tokio::test]
    async fn test_run_hook_echo() {
        let result = run_hook_command("test", "echo hello").await;
        assert!(result.success);
        assert!(result.output.is_some());
    }

    #[tokio::test]
    async fn test_run_hook_false() {
        let result = run_hook_command("test", "false").await;
        assert!(!result.success);
    }
}
