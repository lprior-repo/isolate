//! Schema generators for command outputs.

use super::types::{JsonSchema, PropertySchema};

/// Generate schema for list command output
pub fn list_output_schema() -> JsonSchema {
    JsonSchema::object("ListOutput")
        .with_description("List of all sessions")
        .with_property(
            "sessions",
            &PropertySchema::array(PropertySchema::string().with_description("Session name"))
                .with_description("Array of session names"),
        )
}

/// Generate schema for status command output
pub fn status_output_schema() -> JsonSchema {
    JsonSchema::object("StatusOutput")
        .with_description("Detailed session status information")
        .with_property(
            "name",
            &PropertySchema::string()
                .required()
                .with_description("Session name"),
        )
        .with_property(
            "status",
            &PropertySchema::string()
                .required()
                .with_enum(vec![
                    "creating".to_string(),
                    "active".to_string(),
                    "paused".to_string(),
                    "completed".to_string(),
                    "failed".to_string(),
                ])
                .with_description("Current session status"),
        )
        .with_property(
            "workspace_path",
            &PropertySchema::string()
                .required()
                .with_description("Absolute path to workspace directory"),
        )
        .with_property(
            "branch",
            &PropertySchema::string().with_description("Associated branch name"),
        )
        .with_property(
            "created_at",
            &PropertySchema::string()
                .required()
                .with_format("date-time")
                .with_description("Creation timestamp in ISO 8601 format"),
        )
}

/// Generate schema for config command output
pub fn config_output_schema() -> JsonSchema {
    JsonSchema::object("ConfigOutput")
        .with_description("Configuration view or update result")
        .with_property(
            "key",
            &PropertySchema::string().with_description("Configuration key in dot notation"),
        )
        .with_property(
            "value",
            &PropertySchema::string().with_description("Configuration value"),
        )
        .with_property(
            "scope",
            &PropertySchema::string()
                .with_enum(vec!["global".to_string(), "project".to_string()])
                .with_description("Configuration scope"),
        )
}

/// Generate schema for diff command output
pub fn diff_output_schema() -> JsonSchema {
    JsonSchema::object("DiffOutput")
        .with_description("Diff between session and main branch")
        .with_property(
            "session_name",
            &PropertySchema::string()
                .required()
                .with_description("Session name"),
        )
        .with_property(
            "base",
            &PropertySchema::string()
                .required()
                .with_description("Base commit or branch"),
        )
        .with_property(
            "head",
            &PropertySchema::string()
                .required()
                .with_description("Head commit or workspace"),
        )
        .with_property(
            "files_changed",
            &PropertySchema::integer().with_description("Number of files changed"),
        )
        .with_property(
            "insertions",
            &PropertySchema::integer().with_description("Number of lines inserted"),
        )
        .with_property(
            "deletions",
            &PropertySchema::integer().with_description("Number of lines deleted"),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_output_schema() {
        let schema = list_output_schema();
        assert_eq!(schema.title, Some("ListOutput".to_string()));
        assert!(schema.properties.is_some());
    }

    #[test]
    fn test_status_output_schema() {
        let schema = status_output_schema();
        assert_eq!(schema.title, Some("StatusOutput".to_string()));
        assert!(schema.required.is_some());

        let required = schema.required.unwrap_or_default();
        assert!(required.contains(&"name".to_string()));
        assert!(required.contains(&"status".to_string()));
    }
}
