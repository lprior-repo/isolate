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
    if format.is_json() {
        let output = AddOutput {
            name: name.to_string(),
            workspace_path: workspace_path.to_string(),
            zellij_tab: zellij_tab.to_string(),
            status: format!("Created session '{name}' ({mode})"),
        };
        let envelope = SchemaEnvelope::new("add-response", "single", output);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope)
                .unwrap_or_else(|_| r#"{"error": "serialization failed"}"#.to_string())
        );
    } else {
        println!("Created session '{name}' (workspace at {workspace_path})");
    }
}
