//! whereami command - Quick location query for AI agents
//!
//! Returns the current location in a simple, parseable format:
//! - `main` - On main branch
//! - `workspace:<name>` - In a workspace
//!
//! This is designed for AI agents that need to quickly orient themselves.

use anyhow::Result;
use serde::Serialize;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use super::context;

/// Output for whereami command
#[derive(Debug, Clone, Serialize)]
pub struct WhereAmIOutput {
    /// Location type: "main" or "workspace"
    pub location_type: String,
    /// Workspace name if in a workspace, None if on main
    pub workspace_name: Option<String>,
    /// Full path if in a workspace
    pub workspace_path: Option<String>,
    /// Simple one-line representation
    pub simple: String,
}

/// Options for whereami command
pub struct WhereAmIOptions {
    pub format: OutputFormat,
}

/// Run the whereami command
///
/// # Errors
///
/// Returns an error if unable to determine location
pub fn run(options: &WhereAmIOptions) -> Result<()> {
    let root = super::check_in_jj_repo()?;
    let location = context::detect_location(&root)?;

    let output = match &location {
        context::Location::Main => WhereAmIOutput {
            location_type: "main".to_string(),
            workspace_name: None,
            workspace_path: None,
            simple: "main".to_string(),
        },
        context::Location::Workspace { name, path } => WhereAmIOutput {
            location_type: "workspace".to_string(),
            workspace_name: Some(name.clone()),
            workspace_path: Some(path.clone()),
            simple: format!("workspace:{name}"),
        },
    };

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("whereami-response", "single", &output);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope)
                .unwrap_or_else(|_| r#"{"error": "serialization failed"}"#.to_string())
        );
    } else {
        // Simple output for human consumption - just the simple string
        println!("{}", output.simple);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whereami_output_main() {
        let output = WhereAmIOutput {
            location_type: "main".to_string(),
            workspace_name: None,
            workspace_path: None,
            simple: "main".to_string(),
        };

        assert_eq!(output.simple, "main");
        assert!(output.workspace_name.is_none());
    }

    #[test]
    fn test_whereami_output_workspace() {
        let output = WhereAmIOutput {
            location_type: "workspace".to_string(),
            workspace_name: Some("feature-auth".to_string()),
            workspace_path: Some("/path/to/.zjj/workspaces/feature-auth".to_string()),
            simple: "workspace:feature-auth".to_string(),
        };

        assert_eq!(output.simple, "workspace:feature-auth");
        assert_eq!(output.workspace_name, Some("feature-auth".to_string()));
    }

    #[test]
    fn test_whereami_output_serializes() {
        let output = WhereAmIOutput {
            location_type: "main".to_string(),
            workspace_name: None,
            workspace_path: None,
            simple: "main".to_string(),
        };

        let json = serde_json::to_string(&output);
        assert!(json.is_ok());
        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("\"location_type\":\"main\""));
    }
}
