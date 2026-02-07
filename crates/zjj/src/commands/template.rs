//! Template management commands
//!
//! This module provides commands for managing Zellij layout templates:
//! - list: Show all available templates
//! - create: Create a new template from file or builtin
//! - show: Display template details
//! - delete: Remove a template
//! - use: Apply a template to current session (future)

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

use std::io::{self, Write};

use anyhow::{Context, Result};
use zjj_core::{
    json::SchemaEnvelope,
    kdl_validation,
    templates::storage,
    zellij::{LayoutConfig, LayoutTemplate},
    Error, OutputFormat,
};

use crate::{
    commands::zjj_data_dir,
    json::{
        TemplateCreateOutput, TemplateDeleteOutput, TemplateInfo, TemplateListOutput,
        TemplateShowOutput,
    },
};

/// Options for template create command
#[derive(Debug, Clone)]
pub struct CreateOptions {
    /// Template name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Source: either a builtin template name or file path
    pub source: TemplateSource,
    /// Output format
    pub format: OutputFormat,
}

/// Template source for creation
#[derive(Debug, Clone)]
pub enum TemplateSource {
    /// Use a builtin template
    Builtin(LayoutTemplate),
    /// Import from a KDL file
    FromFile(String),
}

/// Validate a template name
///
/// Template names must:
/// - Not be empty
/// - Not exceed 64 characters
/// - Only contain ASCII alphanumeric characters, dashes, and underscores
/// - Start with a letter (a-z, A-Z)
///
/// # Errors
///
/// Returns `Error::ValidationError` if the template name is invalid
fn validate_template_name(name: &str) -> Result<(), Error> {
    if name.is_empty() {
        return Err(Error::ValidationError(
            "Template name cannot be empty".into(),
        ));
    }

    // Check for non-ASCII characters first (prevents unicode bypasses)
    if !name.is_ascii() {
        return Err(Error::ValidationError(
            "Template name must contain only ASCII characters (a-z, A-Z, 0-9, -, _)".into(),
        ));
    }

    if name.len() > 64 {
        return Err(Error::ValidationError(
            "Template name cannot exceed 64 characters".into(),
        ));
    }

    // Only allow ASCII alphanumeric, dash, and underscore
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(Error::ValidationError(
            "Invalid template name: Template name can only contain ASCII alphanumeric characters, dashes, and underscores"
                .into(),
        ));
    }

    // Must start with a letter (not dash, underscore, or digit)
    if let Some(first) = name.chars().next() {
        if !first.is_ascii_alphabetic() {
            return Err(Error::ValidationError(
                "Invalid template name: Template name must start with a letter (a-z, A-Z)".into(),
            ));
        }
    }

    Ok(())
}

/// Run the template list command
///
/// # Errors
///
/// Returns error if unable to read templates directory or list templates
pub async fn run_list(format: OutputFormat) -> Result<()> {
    let data_dir = zjj_data_dir().await?;
    let templates_base = storage::templates_dir(&data_dir)?;

    let templates = storage::list_templates(&templates_base)?;

    if format.is_json() {
        let template_infos: Vec<TemplateInfo> = templates
            .iter()
            .map(|t| TemplateInfo {
                name: t.name.as_str().to_string(),
                description: t.metadata.description.clone(),
                created_at: t.metadata.created_at,
                updated_at: t.metadata.updated_at,
            })
            .collect();

        let output = TemplateListOutput {
            count: template_infos.len(),
            templates: template_infos,
        };

        let envelope = SchemaEnvelope::new("template-list-response", "array", output);
        let json_str = serde_json::to_string_pretty(&envelope)?;
        writeln!(std::io::stdout(), "{json_str}")?;
    } else if templates.is_empty() {
        println!("No templates found.");
        println!("Use 'zjj template create <name>' to create a template.");
    } else {
        println!("Available templates:");
        for template in templates {
            if let Some(desc) = &template.metadata.description {
                println!("  {} - {}", template.name, desc);
            } else {
                println!("  {}", template.name);
            }
        }
    }

    Ok(())
}

/// Run the template create command
///
/// # Errors
///
/// Returns error if unable to create template or write to storage
pub async fn run_create(options: &CreateOptions) -> Result<()> {
    // Validate template name first
    validate_template_name(&options.name)
        .map_err(|e| anyhow::Error::new(e).context("Invalid template name"))?;

    let data_dir = zjj_data_dir().await?;
    let templates_base = storage::templates_dir(&data_dir)?;

    // Check if template already exists
    if storage::template_exists(&options.name, &templates_base)? {
        anyhow::bail!("Template '{}' already exists", options.name);
    }

    // Get layout content based on source
    let layout_content = match &options.source {
        TemplateSource::Builtin(template_type) => {
            generate_builtin_layout(*template_type, &options.name)?
        }
        TemplateSource::FromFile(file_path) => {
            // Read file as bytes first to detect UTF-8 issues
            let bytes = tokio::fs::read(file_path).await.map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    anyhow::Error::new(e).context(format!("Template file not found: {file_path}"))
                } else {
                    anyhow::Error::new(e)
                }
            })?;

            // Validate UTF-8 and convert to string
            String::from_utf8(bytes).map_err(|e| {
                let valid_up_to = e.utf8_error().valid_up_to();
                anyhow::anyhow!(
                    "Template files must be valid UTF-8 text. \
                     File '{file_path}' contains invalid UTF-8 data at byte {}. \
                     This may be a binary file. Please provide a text-based KDL layout file.",
                    valid_up_to
                )
            })?
        }
    };

    // Validate KDL syntax (for both builtin and file sources)
    kdl_validation::validate_kdl_syntax(&layout_content)
        .map_err(|e| anyhow::anyhow!("Invalid KDL syntax in template '{}': {}", options.name, e))?;

    // Create template
    let template = storage::Template::new(
        options.name.clone(),
        layout_content,
        options.description.clone(),
    )?;

    // Save to storage
    storage::save_template(&template, &templates_base)?;

    if options.format.is_json() {
        let output = TemplateCreateOutput {
            name: options.name.clone(),
            message: format!("Created template '{}'", options.name),
        };
        let envelope = SchemaEnvelope::new("template-create-response", "single", output);
        let json_str = serde_json::to_string_pretty(&envelope)?;
        writeln!(std::io::stdout(), "{json_str}")?;
    } else {
        println!("Created template '{}'", options.name);
    }

    Ok(())
}

/// Generate layout content for a builtin template
fn generate_builtin_layout(template_type: LayoutTemplate, name: &str) -> Result<String> {
    // Create a minimal config for layout generation
    let config = LayoutConfig::new(
        name.to_string(),
        std::path::PathBuf::from("/path/to/workspace"),
    );

    // Generate KDL content
    let kdl = zjj_core::zellij::generate_template_kdl(&config, template_type)?;
    Ok(kdl)
}

/// Run the template show command
///
/// # Errors
///
/// Returns error if template not found or unable to read template
pub async fn run_show(name: &str, format: OutputFormat) -> Result<()> {
    let data_dir = zjj_data_dir().await?;
    let templates_base = storage::templates_dir(&data_dir)?;

    let template = storage::load_template(name, &templates_base)?;

    if format.is_json() {
        let output = TemplateShowOutput {
            name: template.name.as_str().to_string(),
            description: template.metadata.description.clone(),
            created_at: template.metadata.created_at,
            updated_at: template.metadata.updated_at,
            layout: template.layout,
        };
        let envelope = SchemaEnvelope::new("template-show-response", "single", output);
        let json_str = serde_json::to_string_pretty(&envelope)?;
        writeln!(std::io::stdout(), "{json_str}")?;
    } else {
        println!("Template: {}", template.name);
        if let Some(desc) = &template.metadata.description {
            println!("Description: {desc}");
        }
        println!(
            "Created: {}",
            format_timestamp(template.metadata.created_at)
        );
        println!(
            "Updated: {}",
            format_timestamp(template.metadata.updated_at)
        );
        println!("\nLayout:");
        println!("{}", template.layout);
    }

    Ok(())
}

/// Run the template delete command
///
/// # Errors
///
/// Returns error if template not found or unable to delete
pub async fn run_delete(name: &str, force: bool, format: OutputFormat) -> Result<()> {
    let data_dir = zjj_data_dir().await?;
    let templates_base = storage::templates_dir(&data_dir)?;

    // Confirm deletion unless --force
    if !force && !confirm_deletion(name)? {
        if format.is_json() {
            let output = TemplateDeleteOutput {
                name: name.to_string(),
                message: "Deletion cancelled".to_string(),
            };
            let envelope = SchemaEnvelope::new("template-delete-response", "single", output);
            let json_str = serde_json::to_string(&envelope)?;
            writeln!(std::io::stdout(), "{json_str}")?;
        } else {
            writeln!(std::io::stdout(), "Deletion cancelled")?;
        }
        return Ok(());
    }

    storage::delete_template(name, &templates_base)?;

    if format.is_json() {
        let output = TemplateDeleteOutput {
            name: name.to_string(),
            message: format!("Deleted template '{name}'"),
        };
        let envelope = SchemaEnvelope::new("template-delete-response", "single", output);
        let json_str = serde_json::to_string(&envelope)?;
        writeln!(std::io::stdout(), "{json_str}")?;
    } else {
        println!("Deleted template '{name}'");
    }

    Ok(())
}

/// Prompt user for confirmation
fn confirm_deletion(name: &str) -> Result<bool> {
    write!(io::stdout(), "Delete template '{name}'? [y/N] ")?;
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    let response = response.trim().to_lowercase();
    Ok(response == "y" || response == "yes")
}

/// Format Unix timestamp as human-readable string
fn format_timestamp(timestamp: i64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let timestamp_secs = u64::try_from(timestamp.max(0)).unwrap_or_default();
    let ago_secs = now_secs.saturating_sub(timestamp_secs);
    let days = ago_secs / 86400;

    if days == 0 {
        "today".to_string()
    } else if days == 1 {
        "yesterday".to_string()
    } else if days < 7 {
        format!("{days} days ago")
    } else if days < 30 {
        format!("{} weeks ago", days / 7)
    } else if days < 365 {
        format!("{} months ago", days / 30)
    } else {
        format!("{} years ago", days / 365)
    }
}

/// Run the template use command (applies template to current session)
///
/// Note: This is a placeholder for future implementation
///
/// # Errors
///
/// Returns error - not yet implemented
#[allow(dead_code)]
pub fn run_use(_name: &str, _format: OutputFormat) -> Result<()> {
    anyhow::bail!("Template 'use' command is not yet implemented.\nTo use a template, create a session with: zjj add <session-name> -t <template-name>");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_options_construction() {
        let opts = CreateOptions {
            name: "test".to_string(),
            description: Some("Test template".to_string()),
            source: TemplateSource::Builtin(LayoutTemplate::Minimal),
            format: OutputFormat::Human,
        };

        assert_eq!(opts.name, "test");
        assert_eq!(opts.description, Some("Test template".to_string()));
    }

    #[test]
    fn test_format_timestamp() {
        // Test with current timestamp
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
            .unwrap_or(0);

        let formatted = format_timestamp(now);
        assert_eq!(formatted, "today");

        // Test with old timestamp (1 day ago)
        let yesterday = now - 86400;
        let formatted = format_timestamp(yesterday);
        assert_eq!(formatted, "yesterday");

        // Test with very old timestamp (1 year ago)
        let year_ago = now - (365 * 86400);
        let formatted = format_timestamp(year_ago);
        assert!(formatted.contains("year"));
    }

    #[test]
    fn test_template_source_types() {
        let builtin = TemplateSource::Builtin(LayoutTemplate::Standard);
        let from_file = TemplateSource::FromFile("/path/to/file.kdl".to_string());

        assert!(matches!(builtin, TemplateSource::Builtin(_)));
        assert!(matches!(from_file, TemplateSource::FromFile(_)));
    }

    #[tokio::test]
    async fn test_binary_file_error_message() {
        // Create a temporary binary file
        let temp_dir = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(_) => {
                // Skip test if tempfile fails
                return;
            }
        };

        let binary_file_path = temp_dir.path().join("binary.kdl");
        let binary_content = vec![0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD];

        // Write binary content
        if std::fs::write(&binary_file_path, binary_content).is_err() {
            // Skip test if write fails
            return;
        }

        // Try to create a template from the binary file
        let opts = CreateOptions {
            name: "test_binary".to_string(),
            description: None,
            source: TemplateSource::FromFile(binary_file_path.to_string_lossy().to_string()),
            format: OutputFormat::Human,
        };

        let result = run_create(&opts).await;

        // Verify it fails
        assert!(result.is_err());

        // Check that error message mentions UTF-8 requirement
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("UTF-8") || error_msg.contains("utf-8"),
            "Error message should mention UTF-8 requirement. Got: {}",
            error_msg
        );
        assert!(
            error_msg.contains("binary") || error_msg.contains("text"),
            "Error message should mention binary vs text. Got: {}",
            error_msg
        );
    }

    // Tests for template name validation

    #[test]
    fn test_validate_template_name_empty() {
        let result = validate_template_name("");
        assert!(result.is_err());
        if let Err(Error::ValidationError(msg)) = result {
            assert!(msg.contains("empty") || msg.contains("Empty"));
        } else {
            panic!("Expected ValidationError, got: {:?}", result);
        }
    }

    #[test]
    fn test_validate_template_name_too_long() {
        let long_name = "a".repeat(65);
        let result = validate_template_name(&long_name);
        assert!(result.is_err());
        if let Err(Error::ValidationError(msg)) = result {
            assert!(msg.contains("64"));
        } else {
            panic!("Expected ValidationError, got: {:?}", result);
        }
    }

    #[test]
    fn test_validate_template_name_non_ascii() {
        let result = validate_template_name("template-🚀");
        assert!(result.is_err());
        if let Err(Error::ValidationError(msg)) = result {
            assert!(msg.contains("ASCII"));
        } else {
            panic!("Expected ValidationError, got: {:?}", result);
        }
    }

    #[test]
    fn test_validate_template_name_starts_with_digit() {
        let result = validate_template_name("123template");
        assert!(result.is_err());
        if let Err(Error::ValidationError(msg)) = result {
            assert!(msg.contains("start with a letter"));
        } else {
            panic!("Expected ValidationError, got: {:?}", result);
        }
    }

    #[test]
    fn test_validate_template_name_starts_with_dash() {
        let result = validate_template_name("-template");
        assert!(result.is_err());
        if let Err(Error::ValidationError(msg)) = result {
            assert!(msg.contains("start with a letter"));
        } else {
            panic!("Expected ValidationError, got: {:?}", result);
        }
    }

    #[test]
    fn test_validate_template_name_starts_with_underscore() {
        let result = validate_template_name("_template");
        assert!(result.is_err());
        if let Err(Error::ValidationError(msg)) = result {
            assert!(msg.contains("start with a letter"));
        } else {
            panic!("Expected ValidationError, got: {:?}", result);
        }
    }

    #[test]
    fn test_validate_template_name_invalid_characters() {
        let result = validate_template_name("template name");
        assert!(result.is_err());
        if let Err(Error::ValidationError(msg)) = result {
            assert!(msg.contains("alphanumeric") || msg.contains("character"));
        } else {
            panic!("Expected ValidationError, got: {:?}", result);
        }
    }

    #[test]
    fn test_validate_template_name_valid() {
        let valid_names = vec![
            "template",
            "my_template",
            "my-template",
            "my_template-123",
            "Template",
            "TEMPLATE",
            "a",
            "abc123",
        ];

        for name in valid_names {
            let result = validate_template_name(name);
            assert!(
                result.is_ok(),
                "Expected '{name}' to be valid, got: {:?}",
                result
            );
        }
    }

    #[test]
    fn test_validate_template_name_max_length() {
        let max_name = "a".repeat(64);
        let result = validate_template_name(&max_name);
        assert!(result.is_ok(), "64-character name should be valid");
    }

    #[test]
    fn test_validate_template_name_special_characters() {
        let invalid_names = vec!["template.name", "template@name", "template$name"];

        for name in invalid_names {
            let result = validate_template_name(name);
            assert!(
                result.is_err(),
                "Expected '{name}' to be invalid, but it was accepted",
            );
        }
    }
}
