//! Workflow hooks - Run commands on success or failure
//!
//! Provides --on-success and --on-failure hooks for command execution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::Command;

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
    pub fn from_args(on_success: Option<String>, on_failure: Option<String>) -> Self {
        Self {
            on_success,
            on_failure,
        }
    }

    /// Check if any hooks are configured
    pub fn has_hooks(&self) -> bool {
        self.on_success.is_some() || self.on_failure.is_some()
    }

    /// Run the appropriate hook based on result
    pub fn run_hook(&self, success: bool) -> Option<HookResult> {
        let (hook_name, hook_cmd) = if success {
            ("on_success", &self.on_success)
        } else {
            ("on_failure", &self.on_failure)
        };

        hook_cmd.as_ref().map(|cmd| run_hook_command(hook_name, cmd))
    }
}

/// Run a hook command
fn run_hook_command(hook_name: &str, command: &str) -> HookResult {
    // Run the command through the shell
    let result = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", command])
            .output()
    } else {
        Command::new("sh")
            .args(["-c", command])
            .output()
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
                    output: if stdout.is_empty() { None } else { Some(stdout) },
                    error: None,
                }
            } else {
                HookResult {
                    hook: hook_name.to_string(),
                    success: false,
                    command: command.to_string(),
                    output: if stdout.is_empty() { None } else { Some(stdout) },
                    error: Some(if stderr.is_empty() {
                        format!("Hook exited with code: {}", output.status.code().unwrap_or(-1))
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
pub fn with_hooks<F>(hooks: &HooksConfig, f: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    let result = f();
    let success = result.is_ok();

    // Run the appropriate hook
    if let Some(hook_result) = hooks.run_hook(success) {
        // Print hook result (could be suppressed with a flag)
        if hook_result.success {
            if let Some(output) = &hook_result.output {
                if !output.trim().is_empty() {
                    eprintln!("[hook:{}] {}", hook_result.hook, output.trim());
                }
            }
        } else {
            if let Some(error) = &hook_result.error {
                eprintln!("[hook:{} failed] {}", hook_result.hook, error.trim());
            }
        }
    }

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
    fn test_hook_result_serialization() {
        let result = HookResult {
            hook: "on_success".to_string(),
            success: true,
            command: "echo test".to_string(),
            output: Some("test\n".to_string()),
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"hook\":\"on_success\""));
    }

    #[test]
    fn test_run_hook_echo() {
        let result = run_hook_command("test", "echo hello");
        assert!(result.success);
        assert!(result.output.is_some());
    }

    #[test]
    fn test_run_hook_false() {
        let result = run_hook_command("test", "false");
        assert!(!result.success);
    }
}
