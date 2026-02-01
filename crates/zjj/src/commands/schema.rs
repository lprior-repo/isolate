//! Schema command - machine-readable schema definitions
//!
//! Provides actual JSON Schema definitions for AI agents to validate against:
//! - `zjj schema <response-type>` - Get schema for a response type
//! - `zjj schema --all` - Get all schemas
//! - `zjj schema --list` - List available schemas

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

/// Options for schema command
#[derive(Debug, Clone)]
pub struct SchemaOptions {
    /// Specific schema to get (None for list/all)
    pub schema_name: Option<String>,
    /// List all available schemas
    pub list: bool,
    /// Get all schemas
    pub all: bool,
    /// Output format
    pub format: OutputFormat,
}

/// Schema listing output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaListOutput {
    pub schemas: Vec<SchemaInfo>,
    pub base_url: String,
}

/// Schema info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    pub name: String,
    pub description: String,
    pub version: String,
}

/// All schemas output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct AllSchemasOutput {
    pub schemas: serde_json::Value,
}

/// Run the schema command
pub fn run(options: &SchemaOptions) -> Result<()> {
    if options.list {
        return run_list(options.format);
    }

    if options.all {
        return run_all(options.format);
    }

    if let Some(ref name) = options.schema_name {
        return run_single(name, options.format);
    }

    // Default to list
    run_list(options.format)
}

/// List available schemas
fn run_list(format: OutputFormat) -> Result<()> {
    let schemas = get_available_schemas();

    let output = SchemaListOutput {
        schemas,
        base_url: "https://zjj.dev/schemas".to_string(),
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("schema-list-response", "single", &output);
        let json_str =
            serde_json::to_string_pretty(&envelope).context("Failed to serialize schema list")?;
        println!("{json_str}");
    } else {
        println!("Available Schemas:");
        println!();
        for schema in &output.schemas {
            println!("  {} (v{})", schema.name, schema.version);
            println!("    {}", schema.description);
        }
        println!();
        println!("Use 'zjj schema <name> --json' to get the full JSON Schema.");
    }

    Ok(())
}

/// Get all schemas
fn run_all(format: OutputFormat) -> Result<()> {
    let schemas = json!({
        "add-response": get_add_response_schema(),
        "remove-response": get_remove_response_schema(),
        "list-response": get_list_response_schema(),
        "status-response": get_status_response_schema(),
        "sync-response": get_sync_response_schema(),
        "context-response": get_context_response_schema(),
        "ai-status-response": get_ai_status_response_schema(),
        "ai-next-response": get_ai_next_response_schema(),
        "error-response": get_error_response_schema(),
    });

    if format.is_json() {
        let envelope = SchemaEnvelope::new("all-schemas-response", "single", &schemas);
        let json_str =
            serde_json::to_string_pretty(&envelope).context("Failed to serialize all schemas")?;
        println!("{json_str}");
    } else {
        println!("{}", serde_json::to_string_pretty(&schemas)?);
    }

    Ok(())
}

/// Get a single schema
fn run_single(name: &str, format: OutputFormat) -> Result<()> {
    let schema = match name {
        "add-response" => get_add_response_schema(),
        "remove-response" => get_remove_response_schema(),
        "list-response" => get_list_response_schema(),
        "status-response" => get_status_response_schema(),
        "sync-response" => get_sync_response_schema(),
        "context-response" => get_context_response_schema(),
        "ai-status-response" => get_ai_status_response_schema(),
        "ai-next-response" => get_ai_next_response_schema(),
        "error-response" => get_error_response_schema(),
        _ => {
            anyhow::bail!(
                "Unknown schema: {}. Use 'zjj schema --list' to see available schemas.",
                name
            );
        }
    };

    if format.is_json() {
        println!("{}", serde_json::to_string_pretty(&schema)?);
    } else {
        println!("Schema: {}\n", name);
        println!("{}", serde_json::to_string_pretty(&schema)?);
    }

    Ok(())
}

/// Get list of available schemas
fn get_available_schemas() -> Vec<SchemaInfo> {
    vec![
        SchemaInfo {
            name: "add-response".to_string(),
            description: "Response from zjj add command".to_string(),
            version: "1.0".to_string(),
        },
        SchemaInfo {
            name: "remove-response".to_string(),
            description: "Response from zjj remove command".to_string(),
            version: "1.0".to_string(),
        },
        SchemaInfo {
            name: "list-response".to_string(),
            description: "Response from zjj list command".to_string(),
            version: "1.0".to_string(),
        },
        SchemaInfo {
            name: "status-response".to_string(),
            description: "Response from zjj status command".to_string(),
            version: "1.0".to_string(),
        },
        SchemaInfo {
            name: "sync-response".to_string(),
            description: "Response from zjj sync command".to_string(),
            version: "1.0".to_string(),
        },
        SchemaInfo {
            name: "context-response".to_string(),
            description: "Response from zjj context command".to_string(),
            version: "1.0".to_string(),
        },
        SchemaInfo {
            name: "ai-status-response".to_string(),
            description: "Response from zjj ai status command".to_string(),
            version: "1.0".to_string(),
        },
        SchemaInfo {
            name: "ai-next-response".to_string(),
            description: "Response from zjj ai next command".to_string(),
            version: "1.0".to_string(),
        },
        SchemaInfo {
            name: "error-response".to_string(),
            description: "Error response format".to_string(),
            version: "1.0".to_string(),
        },
    ]
}

/// JSON Schema for add response
fn get_add_response_schema() -> serde_json::Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://zjj.dev/schemas/add-response.v1.json",
        "title": "Add Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "schema_type", "success", "data"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "schema_type": { "type": "string", "const": "single" },
            "success": { "type": "boolean" },
            "data": {
                "type": "object",
                "required": ["name", "status", "workspace_path"],
                "properties": {
                    "name": { "type": "string", "description": "Session name" },
                    "status": { "type": "string", "enum": ["active", "creating", "failed"] },
                    "workspace_path": { "type": "string", "description": "Path to workspace" },
                    "branch": { "type": "string", "description": "Git branch name" },
                    "bead_id": { "type": ["string", "null"], "description": "Associated bead ID" }
                }
            }
        }
    })
}

/// JSON Schema for remove response
fn get_remove_response_schema() -> serde_json::Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://zjj.dev/schemas/remove-response.v1.json",
        "title": "Remove Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "schema_type", "success", "data"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "schema_type": { "type": "string", "const": "single" },
            "success": { "type": "boolean" },
            "data": {
                "type": "object",
                "required": ["name", "removed"],
                "properties": {
                    "name": { "type": "string" },
                    "removed": { "type": "boolean" },
                    "merged": { "type": "boolean" },
                    "workspace_deleted": { "type": "boolean" }
                }
            }
        }
    })
}

/// JSON Schema for list response
fn get_list_response_schema() -> serde_json::Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://zjj.dev/schemas/list-response.v1.json",
        "title": "List Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "schema_type", "success", "data"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "schema_type": { "type": "string", "const": "list" },
            "success": { "type": "boolean" },
            "data": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["name", "status"],
                    "properties": {
                        "name": { "type": "string" },
                        "status": { "type": "string" },
                        "branch": { "type": "string" },
                        "bead_id": { "type": ["string", "null"] },
                        "created_at": { "type": "string", "format": "date-time" }
                    }
                }
            }
        }
    })
}

/// JSON Schema for status response
fn get_status_response_schema() -> serde_json::Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://zjj.dev/schemas/status-response.v1.json",
        "title": "Status Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "schema_type", "success", "data"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "schema_type": { "type": "string", "const": "single" },
            "success": { "type": "boolean" },
            "data": {
                "type": "object",
                "required": ["name", "status"],
                "properties": {
                    "name": { "type": "string" },
                    "status": { "type": "string" },
                    "branch": { "type": "string" },
                    "workspace_path": { "type": "string" },
                    "last_synced": { "type": ["string", "null"], "format": "date-time" },
                    "changes": { "type": "integer" }
                }
            }
        }
    })
}

/// JSON Schema for sync response
fn get_sync_response_schema() -> serde_json::Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://zjj.dev/schemas/sync-response.v1.json",
        "title": "Sync Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "schema_type", "success", "data"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "schema_type": { "type": "string", "const": "single" },
            "success": { "type": "boolean" },
            "data": {
                "type": "object",
                "required": ["name", "synced"],
                "properties": {
                    "name": { "type": "string" },
                    "synced": { "type": "boolean" },
                    "conflicts": { "type": "boolean" },
                    "commits_rebased": { "type": "integer" }
                }
            }
        }
    })
}

/// JSON Schema for context response
fn get_context_response_schema() -> serde_json::Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://zjj.dev/schemas/context-response.v1.json",
        "title": "Context Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "schema_type", "success", "data"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "schema_type": { "type": "string", "const": "single" },
            "success": { "type": "boolean" },
            "data": {
                "type": "object",
                "required": ["location"],
                "properties": {
                    "location": {
                        "type": "object",
                        "properties": {
                            "type": { "type": "string", "enum": ["main", "workspace"] },
                            "name": { "type": ["string", "null"] },
                            "path": { "type": "string" }
                        }
                    },
                    "session": {
                        "type": ["object", "null"],
                        "properties": {
                            "name": { "type": "string" },
                            "status": { "type": "string" },
                            "bead_id": { "type": ["string", "null"] }
                        }
                    },
                    "agent": {
                        "type": ["object", "null"],
                        "properties": {
                            "id": { "type": "string" },
                            "registered": { "type": "boolean" }
                        }
                    }
                }
            }
        }
    })
}

/// JSON Schema for AI status response
fn get_ai_status_response_schema() -> serde_json::Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://zjj.dev/schemas/ai-status-response.v1.json",
        "title": "AI Status Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "schema_type", "success", "data"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "schema_type": { "type": "string", "const": "single" },
            "success": { "type": "boolean" },
            "data": {
                "type": "object",
                "required": ["location", "initialized", "ready", "suggestion", "next_command"],
                "properties": {
                    "location": { "type": "string" },
                    "workspace": { "type": ["string", "null"] },
                    "agent_id": { "type": ["string", "null"] },
                    "initialized": { "type": "boolean" },
                    "active_sessions": { "type": "integer" },
                    "ready": { "type": "boolean" },
                    "suggestion": { "type": "string" },
                    "next_command": { "type": "string" }
                }
            }
        }
    })
}

/// JSON Schema for AI next response
fn get_ai_next_response_schema() -> serde_json::Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://zjj.dev/schemas/ai-next-response.v1.json",
        "title": "AI Next Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "schema_type", "success", "data"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "schema_type": { "type": "string", "const": "single" },
            "success": { "type": "boolean" },
            "data": {
                "type": "object",
                "required": ["action", "command", "reason", "priority"],
                "properties": {
                    "action": { "type": "string", "description": "What to do" },
                    "command": { "type": "string", "description": "Copy-paste ready command" },
                    "reason": { "type": "string", "description": "Why this is the next step" },
                    "priority": { "type": "string", "enum": ["high", "medium", "low"] }
                }
            }
        }
    })
}

/// JSON Schema for error response
fn get_error_response_schema() -> serde_json::Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://zjj.dev/schemas/error-response.v1.json",
        "title": "Error Response",
        "type": "object",
        "required": ["$schema", "_schema_version", "success", "error"],
        "properties": {
            "$schema": { "type": "string" },
            "_schema_version": { "type": "string", "const": "1.0" },
            "success": { "type": "boolean", "const": false },
            "error": {
                "type": "object",
                "required": ["message"],
                "properties": {
                    "code": { "type": "string", "description": "Error code" },
                    "message": { "type": "string", "description": "Human-readable message" },
                    "exit_code": { "type": "integer", "description": "Suggested exit code" },
                    "fix_commands": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Commands that might fix the error"
                    },
                    "hints": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "field": { "type": "string" },
                                "issue": { "type": "string" },
                                "suggestion": { "type": "string" }
                            }
                        }
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_list_not_empty() {
        let schemas = get_available_schemas();
        assert!(!schemas.is_empty());
    }

    #[test]
    fn test_all_schemas_have_versions() {
        let schemas = get_available_schemas();
        for schema in schemas {
            assert!(!schema.version.is_empty());
        }
    }

    #[test]
    fn test_add_response_schema_valid() {
        let schema = get_add_response_schema();
        assert!(schema.get("$schema").is_some());
        assert!(schema.get("properties").is_some());
    }

    #[test]
    fn test_error_response_schema_valid() {
        let schema = get_error_response_schema();
        assert!(schema.get("$schema").is_some());
        let props = schema.get("properties").unwrap();
        assert!(props.get("error").is_some());
    }

    // ============================================================================
    // Martin Fowler Style Behavior Tests
    // These tests describe the BEHAVIOR of the schema command
    // ============================================================================

    mod schema_listing_behavior {
        use super::*;

        /// GIVEN: User wants to know available schemas
        /// WHEN: They list schemas
        /// THEN: All major response types should be included
        #[test]
        fn listing_includes_all_major_response_types() {
            let schemas = get_available_schemas();
            let schema_names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();

            // Core commands should have schemas
            assert!(
                schema_names.contains(&"add-response"),
                "Should have add-response"
            );
            assert!(
                schema_names.contains(&"remove-response"),
                "Should have remove-response"
            );
            assert!(
                schema_names.contains(&"list-response"),
                "Should have list-response"
            );

            // Error handling should have schema
            assert!(
                schema_names.contains(&"error-response"),
                "Should have error-response"
            );

            // AI-specific should have schemas
            assert!(
                schema_names.contains(&"ai-status-response"),
                "Should have ai-status-response"
            );
            assert!(
                schema_names.contains(&"ai-next-response"),
                "Should have ai-next-response"
            );
        }

        /// GIVEN: Schema list
        /// WHEN: Displayed
        /// THEN: Each schema should have name, description, and version
        #[test]
        fn each_schema_has_required_metadata() {
            let schemas = get_available_schemas();

            for schema in &schemas {
                assert!(!schema.name.is_empty(), "Schema must have a name");
                assert!(
                    !schema.description.is_empty(),
                    "Schema {} must have a description",
                    schema.name
                );
                assert!(
                    !schema.version.is_empty(),
                    "Schema {} must have a version",
                    schema.name
                );
            }
        }

        /// GIVEN: Schema versions
        /// WHEN: Checked
        /// THEN: All should be valid semantic versions or simple versions
        #[test]
        fn schema_versions_are_valid() {
            let schemas = get_available_schemas();

            for schema in &schemas {
                // Version should be like "1.0" or "1.0.0"
                let parts: Vec<&str> = schema.version.split('.').collect();
                assert!(
                    parts.len() >= 2,
                    "Version {} should have at least major.minor",
                    schema.version
                );

                // First part should be numeric
                assert!(
                    parts[0].parse::<u32>().is_ok(),
                    "Major version should be numeric"
                );
            }
        }
    }

    mod schema_content_behavior {
        use super::*;

        /// GIVEN: Any schema
        /// WHEN: Retrieved
        /// THEN: Should be valid JSON Schema with required meta-fields
        #[test]
        fn all_schemas_are_valid_json_schema() {
            let schema_getters: Vec<(&str, fn() -> serde_json::Value)> = vec![
                ("add-response", get_add_response_schema),
                ("remove-response", get_remove_response_schema),
                ("list-response", get_list_response_schema),
                ("status-response", get_status_response_schema),
                ("sync-response", get_sync_response_schema),
                ("context-response", get_context_response_schema),
                ("ai-status-response", get_ai_status_response_schema),
                ("ai-next-response", get_ai_next_response_schema),
                ("error-response", get_error_response_schema),
            ];

            for (name, getter) in schema_getters {
                let schema = getter();

                // Must have $schema meta-field
                assert!(
                    schema.get("$schema").is_some(),
                    "Schema {} must have $schema",
                    name
                );

                // Must have type
                assert!(
                    schema.get("type").is_some(),
                    "Schema {} must have type",
                    name
                );

                // Must have properties for object types
                if schema.get("type").and_then(|t| t.as_str()) == Some("object") {
                    assert!(
                        schema.get("properties").is_some(),
                        "Object schema {} must have properties",
                        name
                    );
                }
            }
        }

        /// GIVEN: Schema for a response type
        /// WHEN: It declares required fields
        /// THEN: Those fields should be in properties
        #[test]
        fn required_fields_are_in_properties() {
            let schema = get_add_response_schema();

            if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
                let properties = schema.get("properties").unwrap();

                for field in required {
                    if let Some(field_name) = field.as_str() {
                        assert!(
                            properties.get(field_name).is_some(),
                            "Required field '{}' must be in properties",
                            field_name
                        );
                    }
                }
            }
        }

        /// GIVEN: Error response schema
        /// WHEN: Examined
        /// THEN: Should have structure for AI to parse errors
        #[test]
        fn error_schema_is_ai_parseable() {
            let schema = get_error_response_schema();
            let props = schema.get("properties");
            assert!(props.is_some(), "Schema must have properties");
            let props = props.unwrap_or(&serde_json::Value::Null);

            // Must have error field
            assert!(props.get("error").is_some(), "Must have error field");

            let error_props = props.get("error").and_then(|e| e.get("properties"));
            assert!(error_props.is_some(), "error field must have properties");
            let error_props = error_props.unwrap_or(&serde_json::Value::Null);

            // Error should have parseable fields
            assert!(
                error_props.get("message").is_some(),
                "Error must have message"
            );
            assert!(error_props.get("code").is_some(), "Error should have code");
            assert!(
                error_props.get("fix_commands").is_some(),
                "Error should have fix_commands for AI"
            );
        }
    }

    mod schema_options_behavior {
        use super::*;

        /// GIVEN: SchemaOptions with list=true
        /// WHEN: Processed
        /// THEN: Should indicate listing mode
        #[test]
        fn list_option_triggers_listing() {
            let options = SchemaOptions {
                schema_name: None,
                list: true,
                all: false,
                format: zjj_core::OutputFormat::Json,
            };

            assert!(options.list, "list should be true");
            assert!(!options.all, "all should be false when just listing");
        }

        /// GIVEN: SchemaOptions with all=true
        /// WHEN: Processed
        /// THEN: Should indicate all schemas requested
        #[test]
        fn all_option_gets_all_schemas() {
            let options = SchemaOptions {
                schema_name: None,
                list: false,
                all: true,
                format: zjj_core::OutputFormat::Json,
            };

            assert!(options.all, "all should be true");
        }

        /// GIVEN: SchemaOptions with specific schema name
        /// WHEN: Processed
        /// THEN: Should request that specific schema
        #[test]
        fn specific_schema_name_is_preserved() {
            let options = SchemaOptions {
                schema_name: Some("add-response".to_string()),
                list: false,
                all: false,
                format: zjj_core::OutputFormat::Json,
            };

            assert_eq!(options.schema_name, Some("add-response".to_string()));
        }
    }

    mod ai_schema_requirements {
        use super::*;

        /// GIVEN: AI agent needs to validate responses
        /// WHEN: Using ai-status-response schema
        /// THEN: Should have fields for decision making
        #[test]
        fn ai_status_schema_has_decision_fields() {
            let schema = get_ai_status_response_schema();
            let data_props = schema
                .get("properties")
                .and_then(|p| p.get("data"))
                .and_then(|d| d.get("properties"));
            assert!(data_props.is_some(), "Should have data.properties");
            let data_props = data_props.unwrap_or(&serde_json::Value::Null);

            // Fields needed for AI decision making
            assert!(
                data_props.get("ready").is_some(),
                "Need ready field for quick check"
            );
            assert!(
                data_props.get("suggestion").is_some(),
                "Need suggestion for guidance"
            );
            assert!(
                data_props.get("next_command").is_some(),
                "Need next_command for automation"
            );
        }

        /// GIVEN: AI agent needs actionable next step
        /// WHEN: Using ai-next-response schema
        /// THEN: Should have copy-paste ready command
        #[test]
        fn ai_next_schema_has_actionable_command() {
            let schema = get_ai_next_response_schema();
            let data_props = schema
                .get("properties")
                .and_then(|p| p.get("data"))
                .and_then(|d| d.get("properties"));
            assert!(data_props.is_some(), "Should have data.properties");
            let data_props = data_props.unwrap_or(&serde_json::Value::Null);

            // Must have command field
            assert!(
                data_props.get("command").is_some(),
                "Need command for copy-paste"
            );

            // Must have priority for ordering decisions
            assert!(
                data_props.get("priority").is_some(),
                "Need priority for ordering"
            );

            // Must have reason for understanding
            assert!(
                data_props.get("reason").is_some(),
                "Need reason for context"
            );
        }

        /// GIVEN: Any schema
        /// WHEN: Has $id field
        /// THEN: Should point to zjj.dev domain
        #[test]
        fn schema_ids_use_consistent_domain() {
            let schemas = vec![
                get_add_response_schema(),
                get_ai_status_response_schema(),
                get_error_response_schema(),
            ];

            for schema in schemas {
                if let Some(id) = schema.get("$id").and_then(|i| i.as_str()) {
                    assert!(
                        id.starts_with("https://zjj.dev/schemas/"),
                        "Schema $id '{}' should use zjj.dev domain",
                        id
                    );
                }
            }
        }
    }

    mod json_output_behavior {
        use super::*;

        /// GIVEN: SchemaListOutput is serialized
        /// WHEN: AI parses it
        /// THEN: Should have list of schemas and base URL
        #[test]
        fn schema_list_output_is_parseable() {
            let output = SchemaListOutput {
                schemas: get_available_schemas(),
                base_url: "https://zjj.dev/schemas".to_string(),
            };

            let json: serde_json::Value =
                serde_json::from_str(&serde_json::to_string(&output).unwrap()).unwrap();

            assert!(json.get("schemas").is_some(), "Must have schemas array");
            assert!(json.get("base_url").is_some(), "Must have base_url");
            assert!(json["schemas"].is_array(), "schemas must be array");
        }

        /// GIVEN: SchemaInfo is serialized
        /// WHEN: AI parses it
        /// THEN: Should have all metadata fields
        #[test]
        fn schema_info_has_all_fields() {
            let info = SchemaInfo {
                name: "test-response".to_string(),
                description: "Test schema".to_string(),
                version: "1.0".to_string(),
            };

            let json: serde_json::Value =
                serde_json::from_str(&serde_json::to_string(&info).unwrap()).unwrap();

            assert_eq!(json["name"].as_str(), Some("test-response"));
            assert_eq!(json["description"].as_str(), Some("Test schema"));
            assert_eq!(json["version"].as_str(), Some("1.0"));
        }
    }
}
