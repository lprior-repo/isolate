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
use zjj_core::templates::storage;
use zjj_core::zellij::{LayoutConfig, LayoutTemplate};
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::commands::zjj_data_dir;
use crate::json::{
    TemplateCreateOutput, TemplateDeleteOutput, TemplateInfo, TemplateListOutput,
    TemplateShowOutput,
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

/// Run the template list command
///
/// # Errors
///
/// Returns error if unable to read templates directory or list templates
pub fn run_list(format: OutputFormat) -> Result<()> {
    let data_dir = zjj_data_dir()?;
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
        for template in &templates {
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
pub fn run_create(options: &CreateOptions) -> Result<()> {
    let data_dir = zjj_data_dir()?;
    let templates_base = storage::templates_dir(&data_dir)?;

    // Check if template already exists
    if storage::template_exists(&options.name, &templates_base)? {
        anyhow::bail!("Template '{}' already exists", options.name);
    }

    // Get layout content based on source
    let layout_content = match &options.source {
        TemplateSource::Builtin(template_type) => {
            generate_builtin_layout(template_type, &options.name)?
        }
        TemplateSource::FromFile(file_path) => {
            std::fs::read_to_string(file_path).with_context(|| {
                format!("Failed to read template file: {file_path}")
            })?
        }
    };

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
fn generate_builtin_layout(template_type: &LayoutTemplate, name: &str) -> Result<String> {
    // Create a minimal config for layout generation
    let config = LayoutConfig::new(
        name.to_string(),
        std::path::PathBuf::from("/path/to/workspace"),
    );

    // Generate KDL content
    let kdl = zjj_core::zellij::generate_template_kdl(&config, *template_type)?;
    Ok(kdl)
}

/// Run the template show command
///
/// # Errors
///
/// Returns error if template not found or unable to read template
pub fn run_show(name: &str, format: OutputFormat) -> Result<()> {
    let data_dir = zjj_data_dir()?;
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
pub fn run_delete(name: &str, force: bool, format: OutputFormat) -> Result<()> {
    let data_dir = zjj_data_dir()?;
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
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    SystemTime::UNIX_EPOCH
        .checked_add(Duration::from_secs(timestamp as u64))
        .and_then(|time| {
            time.duration_since(UNIX_EPOCH)
                .ok()
                .map(|d| {
                    let days = d.as_secs() / 86400;
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
                })
        })
        .unwrap_or_else(|| "unknown".to_string())
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
    use tempfile::TempDir;

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
            .map(|d| d.as_secs() as i64)
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
}
