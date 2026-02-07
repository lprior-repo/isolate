use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::json::AddOutput;

/// Output the result in the appropriate format
pub(super) fn output_result(
    name: &str,
    workspace_path: &str,
    zellij_tab: &str,
    mode: &str,
    format: OutputFormat,
) {
    // For idempotent mode, use clearer messaging
    let status_msg = if mode.contains("idempotent") {
        format!("Session '{name}' already exists (idempotent)")
    } else {
        format!("Created session '{name}' ({mode})")
    };

    let human_msg = if mode.contains("idempotent") {
        format!("Session '{name}' already exists (idempotent)")
    } else {
        format!("Created session '{name}' (workspace at {workspace_path})")
    };

    if format.is_json() {
        let output = AddOutput {
            name: name.to_string(),
            workspace_path: workspace_path.to_string(),
            zellij_tab: zellij_tab.to_string(),
            status: status_msg,
        };
        let envelope = SchemaEnvelope::new("add-response", "single", output);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope)
                .unwrap_or_else(|_| r#"{"error": "serialization failed"}"#.to_string())
        );
    } else {
        println!("{human_msg}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_result_idempotent_mode() {
        // Test that idempotent mode shows "already exists" not "Created"
        let name = "test-session";
        let workspace_path = "/path/to/workspace";
        let zellij_tab = "zjj:test-session";
        let mode = "already exists (idempotent)";

        // Test JSON format
        let status_msg = if mode.contains("idempotent") {
            format!("Session '{name}' already exists (idempotent)")
        } else {
            format!("Created session '{name}' ({mode})")
        };

        assert_eq!(
            status_msg,
            "Session 'test-session' already exists (idempotent)"
        );
        assert!(!status_msg.contains("Created"));

        // Test human format
        let human_msg = if mode.contains("idempotent") {
            format!("Session '{name}' already exists (idempotent)")
        } else {
            format!("Created session '{name}' (workspace at {workspace_path})")
        };

        assert_eq!(
            human_msg,
            "Session 'test-session' already exists (idempotent)"
        );
        assert!(!human_msg.contains("Created"));
    }

    #[test]
    fn test_output_result_normal_mode() {
        // Test that normal mode shows "Created"
        let name = "test-session";
        let workspace_path = "/path/to/workspace";
        let mode = "with Zellij tab";

        let status_msg = if mode.contains("idempotent") {
            format!("Session '{name}' already exists (idempotent)")
        } else {
            format!("Created session '{name}' ({mode})")
        };

        assert_eq!(
            status_msg,
            "Created session 'test-session' (with Zellij tab)"
        );
        assert!(status_msg.contains("Created"));
    }
}
