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
        let json_str = serde_json::to_string_pretty(&envelope)
            .context("Failed to serialize schema list")?;
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
        let json_str = serde_json::to_string_pretty(&envelope)
            .context("Failed to serialize all schemas")?;
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
}
