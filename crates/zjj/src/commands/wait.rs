//! Wait command - blocking primitives for AI agents
//!
//! Provides commands that block until conditions are met:
//! - `zjj wait session-exists <name>` - Wait for session to exist
//! - `zjj wait session-unlocked <name>` - Wait for session to be unlocked
//! - `zjj wait healthy` - Wait for system to be healthy

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::Serialize;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use super::get_session_db;

/// Options for wait command
#[derive(Debug, Clone)]
pub struct WaitOptions {
    pub condition: WaitCondition,
    pub timeout: Duration,
    pub poll_interval: Duration,
    pub format: OutputFormat,
}

/// Wait condition types
#[derive(Debug, Clone)]
pub enum WaitCondition {
    /// Wait for session to exist
    SessionExists(String),
    /// Wait for session to be unlocked (not in use by another agent)
    SessionUnlocked(String),
    /// Wait for system to be healthy
    Healthy,
    /// Wait for session to reach a specific status
    SessionStatus { name: String, status: String },
}

/// Wait result output
#[derive(Debug, Clone, Serialize)]
pub struct WaitOutput {
    /// Whether the condition was met
    pub success: bool,
    /// The condition that was waited for
    pub condition: String,
    /// How long we waited (in milliseconds)
    pub elapsed_ms: u64,
    /// Whether we timed out
    pub timed_out: bool,
    /// Current state when condition was met or timed out
    pub final_state: Option<String>,
}

/// Run the wait command
pub fn run(options: &WaitOptions) -> Result<()> {
    let start = Instant::now();
    let mut last_state = None;

    loop {
        // Check if condition is met
        let (met, state) = check_condition(&options.condition)?;
        last_state = state;

        if met {
            let output = WaitOutput {
                success: true,
                condition: format_condition(&options.condition),
                elapsed_ms: start.elapsed().as_millis() as u64,
                timed_out: false,
                final_state: last_state,
            };
            return output_result(&output, options.format);
        }

        // Check timeout
        if start.elapsed() >= options.timeout {
            let output = WaitOutput {
                success: false,
                condition: format_condition(&options.condition),
                elapsed_ms: start.elapsed().as_millis() as u64,
                timed_out: true,
                final_state: last_state,
            };
            return output_result(&output, options.format);
        }

        // Wait before next poll
        std::thread::sleep(options.poll_interval);
    }
}

/// Check if a condition is met
fn check_condition(condition: &WaitCondition) -> Result<(bool, Option<String>)> {
    match condition {
        WaitCondition::SessionExists(name) => {
            let db = get_session_db().ok();
            if let Some(db) = db {
                match db.get_blocking(name) {
                    Ok(Some(session)) => {
                        Ok((true, Some(format!("status:{}", session.status))))
                    }
                    Ok(None) => Ok((false, Some("not_found".to_string()))),
                    Err(_) => Ok((false, Some("error".to_string()))),
                }
            } else {
                Ok((false, Some("db_unavailable".to_string())))
            }
        }

        WaitCondition::SessionUnlocked(name) => {
            let db = get_session_db().ok();
            if let Some(db) = db {
                match db.get_blocking(name) {
                    Ok(Some(session)) => {
                        // Check if session is locked (has an active agent)
                        let locked = session
                            .metadata
                            .as_ref()
                            .and_then(|m| m.get("locked_by"))
                            .is_some();
                        Ok((!locked, Some(format!("locked:{}", locked))))
                    }
                    Ok(None) => {
                        // Session doesn't exist - consider it "unlocked"
                        Ok((true, Some("not_found".to_string())))
                    }
                    Err(_) => Ok((false, Some("error".to_string()))),
                }
            } else {
                Ok((false, Some("db_unavailable".to_string())))
            }
        }

        WaitCondition::Healthy => {
            // Check if system is healthy
            let jj_ok = crate::cli::is_command_available("jj");
            let zellij_ok = crate::cli::is_command_available("zellij");
            let db_ok = get_session_db().is_ok();

            let healthy = jj_ok && zellij_ok && db_ok;
            let state = format!(
                "jj:{},zellij:{},db:{}",
                if jj_ok { "ok" } else { "missing" },
                if zellij_ok { "ok" } else { "missing" },
                if db_ok { "ok" } else { "error" }
            );

            Ok((healthy, Some(state)))
        }

        WaitCondition::SessionStatus { name, status } => {
            let db = get_session_db().ok();
            if let Some(db) = db {
                match db.get_blocking(name) {
                    Ok(Some(session)) => {
                        let current_status = session.status.to_string();
                        let met = current_status == *status;
                        Ok((met, Some(format!("status:{}", current_status))))
                    }
                    Ok(None) => Ok((false, Some("not_found".to_string()))),
                    Err(_) => Ok((false, Some("error".to_string()))),
                }
            } else {
                Ok((false, Some("db_unavailable".to_string())))
            }
        }
    }
}

/// Format condition for display
fn format_condition(condition: &WaitCondition) -> String {
    match condition {
        WaitCondition::SessionExists(name) => format!("session-exists:{}", name),
        WaitCondition::SessionUnlocked(name) => format!("session-unlocked:{}", name),
        WaitCondition::Healthy => "healthy".to_string(),
        WaitCondition::SessionStatus { name, status } => {
            format!("session-status:{}={}", name, status)
        }
    }
}

/// Output the result
fn output_result(output: &WaitOutput, format: OutputFormat) -> Result<()> {
    if format.is_json() {
        let envelope = SchemaEnvelope::new("wait-response", "single", output);
        let json_str = serde_json::to_string_pretty(&envelope)
            .context("Failed to serialize wait output")?;
        println!("{json_str}");
    } else {
        if output.success {
            println!("Condition met: {}", output.condition);
        } else if output.timed_out {
            println!("Timeout: {} not met after {}ms", output.condition, output.elapsed_ms);
        } else {
            println!("Failed: {}", output.condition);
        }
        if let Some(ref state) = output.final_state {
            println!("Final state: {}", state);
        }
    }

    // Exit with appropriate code
    if output.success {
        Ok(())
    } else {
        anyhow::bail!("Wait condition not met")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_condition_session_exists() {
        let cond = WaitCondition::SessionExists("test".to_string());
        assert_eq!(format_condition(&cond), "session-exists:test");
    }

    #[test]
    fn test_format_condition_healthy() {
        let cond = WaitCondition::Healthy;
        assert_eq!(format_condition(&cond), "healthy");
    }

    #[test]
    fn test_wait_output_serializes() {
        let output = WaitOutput {
            success: true,
            condition: "healthy".to_string(),
            elapsed_ms: 100,
            timed_out: false,
            final_state: Some("jj:ok,zellij:ok".to_string()),
        };

        let json = serde_json::to_string(&output);
        assert!(json.is_ok());
    }

    #[test]
    fn test_wait_output_timeout() {
        let output = WaitOutput {
            success: false,
            condition: "session-exists:test".to_string(),
            elapsed_ms: 30000,
            timed_out: true,
            final_state: Some("not_found".to_string()),
        };

        assert!(!output.success);
        assert!(output.timed_out);
    }
}
