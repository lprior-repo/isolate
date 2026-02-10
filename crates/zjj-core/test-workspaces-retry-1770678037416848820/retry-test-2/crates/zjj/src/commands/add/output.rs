use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::json::AddOutput;

/// Output the result in the appropriate format
pub(super) fn output_result(
    name: &str,
    workspace_path: &str,
    zellij_tab: &str,
    mode: &str,
    created: bool,
    format: OutputFormat,
) {
    let is_replay = mode.contains("idempotent") || mode.contains("command replay");

    let status_msg = if is_replay {
        format!("Session '{name}' already exists (idempotent)")
    } else {
        format!("Created session '{name}' ({mode})")
    };

    let human_msg = if is_replay {
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
            created,
        };
        let envelope_result =
            serde_json::to_value(SchemaEnvelope::new("add-response", "single", &output));
        match envelope_result {
            Ok(mut value) => {
                if let Some(obj) = value.as_object_mut() {
                    obj.insert(
                        "schema".to_string(),
                        serde_json::Value::String("add-response".to_string()),
                    );
                    obj.insert(
                        "type".to_string(),
                        serde_json::Value::String("single".to_string()),
                    );
                    if let Ok(data_value) = serde_json::to_value(output) {
                        obj.insert("data".to_string(), data_value);
                    }
                }

                match serde_json::to_string_pretty(&value) {
                    Ok(serialized) => println!("{serialized}"),
                    Err(_) => println!("{{\"error\": \"serialization failed\"}}"),
                }
            }
            Err(_) => println!("{{\"error\": \"serialization failed\"}}"),
        }
    } else {
        println!("{human_msg}");
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::no_effect_underscore_binding)]

    #[test]
    fn test_output_result_idempotent_mode() {
        // Test that idempotent mode shows "already exists" not "Created"
        let name = "test-session";
        let workspace_path = "/path/to/workspace";
        let _zellij_tab = "zjj:test-session";
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
        let _workspace_path = "/path/to/workspace";
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

    #[test]
    fn test_output_result_command_replay_mode() {
        let name = "test-session";
        let mode = "already exists (command replay)";

        let is_replay = mode.contains("idempotent") || mode.contains("command replay");
        let status_msg = if is_replay {
            format!("Session '{name}' already exists (idempotent)")
        } else {
            format!("Created session '{name}' ({mode})")
        };

        assert_eq!(
            status_msg,
            "Session 'test-session' already exists (idempotent)"
        );
        assert!(!status_msg.contains("Created"));
    }
}
