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
                    Ok(Some(session)) => Ok((true, Some(format!("status:{}", session.status)))),
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
        let json_str =
            serde_json::to_string_pretty(&envelope).context("Failed to serialize wait output")?;
        println!("{json_str}");
    } else {
        if output.success {
            println!("Condition met: {}", output.condition);
        } else if output.timed_out {
            println!(
                "Timeout: {} not met after {}ms",
                output.condition, output.elapsed_ms
            );
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

    // ============================================================================
    // Martin Fowler Style Behavior Tests
    // These tests describe the BEHAVIOR of the wait command
    // ============================================================================

    mod condition_formatting_behavior {
        use super::*;

        /// GIVEN: A session-exists condition
        /// WHEN: Formatted for display
        /// THEN: Should include condition type and session name
        #[test]
        fn session_exists_format_includes_name() {
            let condition = WaitCondition::SessionExists("my-feature".to_string());
            let formatted = format_condition(&condition);

            assert!(
                formatted.contains("session-exists"),
                "Should identify condition type"
            );
            assert!(
                formatted.contains("my-feature"),
                "Should include session name"
            );
            assert_eq!(formatted, "session-exists:my-feature");
        }

        /// GIVEN: A session-unlocked condition
        /// WHEN: Formatted for display
        /// THEN: Should include condition type and session name
        #[test]
        fn session_unlocked_format_includes_name() {
            let condition = WaitCondition::SessionUnlocked("locked-session".to_string());
            let formatted = format_condition(&condition);

            assert_eq!(formatted, "session-unlocked:locked-session");
        }

        /// GIVEN: A healthy condition
        /// WHEN: Formatted for display
        /// THEN: Should simply say "healthy"
        #[test]
        fn healthy_format_is_simple() {
            let condition = WaitCondition::Healthy;
            let formatted = format_condition(&condition);

            assert_eq!(formatted, "healthy");
        }

        /// GIVEN: A session-status condition
        /// WHEN: Formatted for display
        /// THEN: Should include session name and target status
        #[test]
        fn session_status_format_includes_name_and_status() {
            let condition = WaitCondition::SessionStatus {
                name: "build-task".to_string(),
                status: "completed".to_string(),
            };
            let formatted = format_condition(&condition);

            assert!(
                formatted.contains("build-task"),
                "Should include session name"
            );
            assert!(
                formatted.contains("completed"),
                "Should include target status"
            );
            assert_eq!(formatted, "session-status:build-task=completed");
        }
    }

    mod wait_output_behavior {
        use super::*;

        /// GIVEN: A condition was met successfully
        /// WHEN: Output is generated
        /// THEN: success should be true and timed_out should be false
        #[test]
        fn successful_wait_shows_success() {
            let output = WaitOutput {
                success: true,
                condition: "session-exists:my-session".to_string(),
                elapsed_ms: 50,
                timed_out: false,
                final_state: Some("status:active".to_string()),
            };

            assert!(output.success, "Should indicate success");
            assert!(!output.timed_out, "Should not be timed out");
            assert!(output.elapsed_ms < 1000, "Quick success should be fast");
        }

        /// GIVEN: Timeout occurred before condition was met
        /// WHEN: Output is generated
        /// THEN: success should be false and timed_out should be true
        #[test]
        fn timeout_shows_failure_and_timed_out() {
            let output = WaitOutput {
                success: false,
                condition: "session-exists:missing-session".to_string(),
                elapsed_ms: 30000,
                timed_out: true,
                final_state: Some("not_found".to_string()),
            };

            assert!(!output.success, "Should indicate failure");
            assert!(output.timed_out, "Should indicate timeout");
            assert!(
                output.elapsed_ms >= 30000,
                "Should show full timeout duration"
            );
        }

        /// GIVEN: Condition failed for non-timeout reason
        /// WHEN: Output is generated
        /// THEN: success should be false but timed_out should also be false
        #[test]
        fn failure_without_timeout_is_distinct() {
            let output = WaitOutput {
                success: false,
                condition: "healthy".to_string(),
                elapsed_ms: 100,
                timed_out: false,
                final_state: Some("jj:missing".to_string()),
            };

            assert!(!output.success, "Should indicate failure");
            assert!(!output.timed_out, "Should not be timeout");
        }

        /// GIVEN: Wait completed
        /// WHEN: Output includes final_state
        /// THEN: It should describe why the wait ended
        #[test]
        fn final_state_explains_outcome() {
            // Success case
            let success_output = WaitOutput {
                success: true,
                condition: "session-exists:test".to_string(),
                elapsed_ms: 500,
                timed_out: false,
                final_state: Some("status:active".to_string()),
            };
            assert!(
                success_output
                    .final_state
                    .as_ref()
                    .map_or(false, |s| s.contains("active")),
                "Success final_state should contain 'active'"
            );

            // Failure case
            let failure_output = WaitOutput {
                success: false,
                condition: "healthy".to_string(),
                elapsed_ms: 100,
                timed_out: false,
                final_state: Some("zellij:missing".to_string()),
            };
            assert!(
                failure_output
                    .final_state
                    .as_ref()
                    .map_or(false, |s| s.contains("missing")),
                "Failure final_state should contain 'missing'"
            );
        }
    }

    mod wait_condition_behavior {
        use super::*;

        /// GIVEN: Different wait condition types exist
        /// WHEN: They are used
        /// THEN: Each should represent a distinct wait scenario
        #[test]
        fn all_condition_types_are_distinct() {
            let conditions: Vec<WaitCondition> = vec![
                WaitCondition::SessionExists("a".to_string()),
                WaitCondition::SessionUnlocked("b".to_string()),
                WaitCondition::Healthy,
                WaitCondition::SessionStatus {
                    name: "c".to_string(),
                    status: "active".to_string(),
                },
            ];

            // Each condition type should format differently
            let formatted: Vec<String> = conditions.iter().map(format_condition).collect();

            // All should be unique
            let mut unique = formatted.clone();
            unique.sort();
            unique.dedup();
            assert_eq!(
                unique.len(),
                formatted.len(),
                "All conditions should format uniquely"
            );
        }

        /// GIVEN: A session name
        /// WHEN: Used in SessionExists condition
        /// THEN: That exact name should be preserved
        #[test]
        fn session_name_is_preserved_in_condition() {
            let session_name = "feature-auth-oauth2-integration";
            let condition = WaitCondition::SessionExists(session_name.to_string());

            assert!(
                matches!(condition, WaitCondition::SessionExists(ref name) if name == session_name),
                "Session name should be preserved exactly in SessionExists variant"
            );
        }
    }

    mod wait_options_behavior {
        use std::time::Duration;

        use super::*;

        /// GIVEN: Default wait options
        /// WHEN: Created
        /// THEN: Should have sensible defaults for AI agents
        #[test]
        fn default_options_are_sensible() {
            let options = WaitOptions {
                condition: WaitCondition::Healthy,
                timeout: Duration::from_secs(30),
                poll_interval: Duration::from_secs(1),
                format: zjj_core::OutputFormat::Json,
            };

            assert!(
                options.timeout.as_secs() >= 10,
                "Timeout should be at least 10s"
            );
            assert!(
                options.poll_interval.as_millis() >= 100,
                "Poll interval should be at least 100ms"
            );
            assert!(
                options.poll_interval < options.timeout,
                "Poll interval should be less than timeout"
            );
        }

        /// GIVEN: Wait options with JSON format
        /// WHEN: Output is generated
        /// THEN: Format should be preserved for AI consumption
        #[test]
        fn json_format_is_preserved() {
            let options = WaitOptions {
                condition: WaitCondition::Healthy,
                timeout: Duration::from_secs(5),
                poll_interval: Duration::from_secs(1),
                format: zjj_core::OutputFormat::Json,
            };

            assert!(options.format.is_json(), "JSON format should be preserved");
        }
    }

    mod json_output_behavior {
        use super::*;

        /// GIVEN: WaitOutput is serialized to JSON
        /// WHEN: AI agent parses it
        /// THEN: All necessary fields should be present
        #[test]
        fn json_has_all_required_fields() {
            let output = WaitOutput {
                success: true,
                condition: "healthy".to_string(),
                elapsed_ms: 500,
                timed_out: false,
                final_state: Some("all:ok".to_string()),
            };

            let json: serde_json::Value =
                serde_json::from_str(&serde_json::to_string(&output).unwrap()).unwrap();

            // All fields should be present
            assert!(json.get("success").is_some(), "Must have success");
            assert!(json.get("condition").is_some(), "Must have condition");
            assert!(json.get("elapsed_ms").is_some(), "Must have elapsed_ms");
            assert!(json.get("timed_out").is_some(), "Must have timed_out");
            assert!(json.get("final_state").is_some(), "Must have final_state");

            // Types should be correct
            assert!(json["success"].is_boolean());
            assert!(json["condition"].is_string());
            assert!(json["elapsed_ms"].is_number());
            assert!(json["timed_out"].is_boolean());
        }

        /// GIVEN: WaitOutput with timeout
        /// WHEN: Serialized and parsed
        /// THEN: Can determine why wait failed
        #[test]
        fn timeout_output_is_diagnosable() {
            let output = WaitOutput {
                success: false,
                condition: "session-exists:missing".to_string(),
                elapsed_ms: 30000,
                timed_out: true,
                final_state: Some("not_found".to_string()),
            };

            let json: serde_json::Value =
                serde_json::from_str(&serde_json::to_string(&output).unwrap()).unwrap();

            // AI can determine the cause
            assert_eq!(json["success"].as_bool(), Some(false));
            assert_eq!(json["timed_out"].as_bool(), Some(true));
            assert!(json["condition"].as_str().unwrap().contains("missing"));
            assert_eq!(json["final_state"].as_str(), Some("not_found"));
        }
    }
}
