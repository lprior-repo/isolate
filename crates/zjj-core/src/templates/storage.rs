//! Template storage and management
//!
//! This module provides type-safe storage for Zellij layout templates.
//! Templates are stored in `.zjj/templates/<name>/` with:
//! - `layout.kdl` - The Zellij layout content
//! - `metadata.json` - Template metadata (`created_at`, description, etc.)

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

use std::{
    fs::File,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use fs2::FileExt;

use crate::{Error, Result};

/// A validated template name
///
/// Names must:
/// - Be 1-64 characters long
/// - Contain only ASCII alphanumeric, dash, or underscore
/// - Not start with a dash
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TemplateName(String);

impl TemplateName {
    /// Create a new template name with validation
    ///
    /// # Errors
    ///
    /// Returns error if name is invalid
    pub fn new(name: String) -> Result<Self> {
        Self::validate(&name)?;
        Ok(Self(name))
    }

    /// Validate a template name
    fn validate(name: &str) -> Result<()> {
        if name.is_empty() || name.len() > 64 {
            return Err(Error::ValidationError(
                "Template name must be 1-64 characters".to_string(),
            ));
        }

        if name.starts_with('-') {
            return Err(Error::ValidationError(
                "Template name cannot start with dash".to_string(),
            ));
        }

        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(Error::ValidationError(
                "Template name must contain only ASCII alphanumeric, dash, or underscore"
                    .to_string(),
            ));
        }

        Ok(())
    }

    /// Get the name as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for TemplateName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TemplateName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Template metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateMetadata {
    /// Template name
    pub name: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Unix timestamp when template was created
    pub created_at: i64,
    /// Unix timestamp when template was last updated
    pub updated_at: i64,
}

impl TemplateMetadata {
    /// Create new metadata with current timestamp
    #[must_use]
    pub fn new(name: String, description: Option<String>) -> Self {
        let now = current_timestamp();
        Self {
            name,
            description,
            created_at: now,
            updated_at: now,
        }
    }
}

/// A complete template with layout and metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    /// Validated template name
    pub name: TemplateName,
    /// KDL layout content
    pub layout: String,
    /// Template metadata
    pub metadata: TemplateMetadata,
}

impl Template {
    /// Create a new template
    ///
    /// # Errors
    ///
    /// Returns error if name validation fails
    pub fn new(name: String, layout: String, description: Option<String>) -> Result<Self> {
        let template_name = TemplateName::new(name.clone())?;
        let metadata = TemplateMetadata::new(name, description);

        Ok(Self {
            name: template_name,
            layout,
            metadata,
        })
    }
}

/// Get current Unix timestamp
fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0)
}

/// Get the templates directory path for a repository
///
/// # Errors
///
/// Returns error if the path cannot be constructed
pub fn templates_dir(repo_root: &Path) -> Result<PathBuf> {
    let zjj_dir = repo_root.join(".zjj");
    let templates_path = zjj_dir.join("templates");
    Ok(templates_path)
}

/// Get the directory path for a specific template
fn template_dir(templates_base: &Path, name: &TemplateName) -> PathBuf {
    templates_base.join(name.as_str())
}

/// List all available templates
///
/// # Errors
///
/// Returns error if:
/// - Templates directory doesn't exist
/// - Unable to read directory
/// - Unable to parse template metadata
pub fn list_templates(templates_base: &Path) -> Result<Vec<Template>> {
    if !templates_base.exists() {
        return Ok(Vec::new());
    }

    let entries = std::fs::read_dir(templates_base)
        .map_err(|e| Error::IoError(format!("Failed to read templates directory: {e}")))?;

    let templates = entries
        .filter_map(|entry| {
            entry
                .ok()
                .and_then(|e| load_template_from_dir(&e.path()).ok())
        })
        .collect::<Vec<_>>();

    Ok(templates)
}

/// Load a template from its directory
fn load_template_from_dir(dir: &Path) -> Result<Template> {
    let metadata_path = dir.join("metadata.json");
    let layout_path = dir.join("layout.kdl");

    // Read metadata
    let metadata_content = std::fs::read_to_string(&metadata_path)
        .map_err(|e| Error::IoError(format!("Failed to read template metadata: {e}")))?;

    let metadata: TemplateMetadata = serde_json::from_str(&metadata_content)
        .map_err(|e| Error::ValidationError(format!("Invalid template metadata: {e}")))?;

    // Read layout
    let layout = std::fs::read_to_string(&layout_path)
        .map_err(|e| Error::IoError(format!("Failed to read template layout: {e}")))?;

    let name = TemplateName::new(metadata.name.clone())?;

    Ok(Template {
        name,
        layout,
        metadata,
    })
}

/// Load a specific template by name
///
/// # Errors
///
/// Returns error if:
/// - Template doesn't exist
/// - Unable to read template files
/// - Invalid template metadata
pub fn load_template(name: &str, templates_base: &Path) -> Result<Template> {
    let template_name = TemplateName::new(name.to_string())?;
    let dir = template_dir(templates_base, &template_name);

    if !dir.exists() {
        return Err(Error::NotFound(format!("Template '{name}' not found")));
    }

    load_template_from_dir(&dir)
}

/// Save a template to storage
///
/// # Errors
///
/// Returns error if:
/// - Unable to create template directory
/// - Unable to acquire file lock
/// - Unable to write template files
pub fn save_template(template: &Template, templates_base: &Path) -> Result<()> {
    let dir = template_dir(templates_base, &template.name);

    // Create template directory
    std::fs::create_dir_all(&dir)
        .map_err(|e| Error::IoError(format!("Failed to create template directory: {e}")))?;

    // Acquire exclusive lock on templates directory before writing
    // Lock is automatically released when lock_file goes out of scope
    let lock_path = templates_base.join(".template.lock");
    let lock_file = File::options()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| Error::IoError(format!("Failed to create lock file: {e}")))?;

    lock_file
        .lock_exclusive()
        .map_err(|e| Error::IoError(format!("Failed to acquire template lock: {e}")))?;

    // Write metadata
    let metadata_path = dir.join("metadata.json");
    let metadata_json = serde_json::to_string_pretty(&template.metadata)
        .map_err(|e| Error::IoError(format!("Failed to serialize template metadata: {e}")))?;

    std::fs::write(&metadata_path, metadata_json)
        .map_err(|e| Error::IoError(format!("Failed to write template metadata: {e}")))?;

    // Write layout
    let layout_path = dir.join("layout.kdl");
    std::fs::write(&layout_path, &template.layout)
        .map_err(|e| Error::IoError(format!("Failed to write template layout: {e}")))?;

    // Lock automatically released here when lock_file is dropped
    Ok(())
}

/// Delete a template
///
/// # Errors
///
/// Returns error if:
/// - Template doesn't exist
/// - Unable to acquire file lock
/// - Unable to remove template directory
pub fn delete_template(name: &str, templates_base: &Path) -> Result<()> {
    let template_name = TemplateName::new(name.to_string())?;
    let dir = template_dir(templates_base, &template_name);

    if !dir.exists() {
        return Err(Error::NotFound(format!("Template '{name}' not found")));
    }

    // Acquire exclusive lock on templates directory before deleting
    // Lock is automatically released when lock_file goes out of scope
    let lock_path = templates_base.join(".template.lock");
    let lock_file = File::options()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| Error::IoError(format!("Failed to create lock file: {e}")))?;

    lock_file
        .lock_exclusive()
        .map_err(|e| Error::IoError(format!("Failed to acquire template lock: {e}")))?;

    std::fs::remove_dir_all(&dir)
        .map_err(|e| Error::IoError(format!("Failed to delete template: {e}")))?;

    // Lock automatically released here when lock_file is dropped
    Ok(())
}

/// Check if a template exists
///
/// # Errors
///
/// Returns error if name validation fails
pub fn template_exists(name: &str, templates_base: &Path) -> Result<bool> {
    let template_name = TemplateName::new(name.to_string())?;
    let dir = template_dir(templates_base, &template_name);
    Ok(dir.exists())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_template_name_validation() {
        // Valid names
        assert!(TemplateName::new("minimal".to_string()).is_ok());
        assert!(TemplateName::new("my-template".to_string()).is_ok());
        assert!(TemplateName::new("template_123".to_string()).is_ok());

        // Invalid names
        assert!(TemplateName::new(String::new()).is_err());
        assert!(TemplateName::new("-starts-with-dash".to_string()).is_err());
        assert!(TemplateName::new("has space".to_string()).is_err());
        assert!(TemplateName::new("has/slash".to_string()).is_err());
        assert!(TemplateName::new("a".repeat(65)).is_err());
    }

    #[test]
    fn test_template_creation() -> Result<()> {
        let template = Template::new(
            "test".to_string(),
            "layout { }".to_string(),
            Some("Test template".to_string()),
        )?;

        assert_eq!(template.name.as_str(), "test");
        assert_eq!(template.layout, "layout { }");
        assert_eq!(
            template.metadata.description,
            Some("Test template".to_string())
        );
        assert!(template.metadata.created_at > 0);
        assert_eq!(template.metadata.created_at, template.metadata.updated_at);

        Ok(())
    }

    #[test]
    fn test_save_and_load_template() -> Result<()> {
        let temp_dir = TempDir::new()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
        let templates_base = temp_dir.path();

        let template = Template::new(
            "test".to_string(),
            "layout { pane }".to_string(),
            Some("Test template".to_string()),
        )?;

        // Save template
        save_template(&template, templates_base)?;

        // Load template
        let loaded = load_template("test", templates_base)?;

        assert_eq!(loaded.name, template.name);
        assert_eq!(loaded.layout, template.layout);
        assert_eq!(loaded.metadata.name, template.metadata.name);
        assert_eq!(loaded.metadata.description, template.metadata.description);

        Ok(())
    }

    #[test]
    fn test_list_templates() -> Result<()> {
        let temp_dir = TempDir::new()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
        let templates_base = temp_dir.path();

        // Empty list initially
        let initial_templates = list_templates(templates_base)?;
        assert_eq!(initial_templates.len(), 0);

        // Create some templates
        let tmpl1 = Template::new("first".to_string(), "layout { }".to_string(), None)?;
        let tmpl2 = Template::new("second".to_string(), "layout { }".to_string(), None)?;

        save_template(&tmpl1, templates_base)?;
        save_template(&tmpl2, templates_base)?;

        // List should now have 2 templates
        let final_templates = list_templates(templates_base)?;
        assert_eq!(final_templates.len(), 2);

        Ok(())
    }

    #[test]
    fn test_delete_template() -> Result<()> {
        let temp_dir = TempDir::new()
            .map_err(|e| Error::IoError(format!("Failed to create temp dir: {e}")))?;
        let templates_base = temp_dir.path();

        let template = Template::new("test".to_string(), "layout { }".to_string(), None)?;
        save_template(&template, templates_base)?;

        assert!(template_exists("test", templates_base)?);

        delete_template("test", templates_base)?;

        assert!(!template_exists("test", templates_base)?);

        Ok(())
    }

    #[test]
    fn test_template_not_found() {
        let temp_dir = TempDir::new()
            .ok()
            .ok_or_else(|| Error::IoError("Failed to create temp dir".to_string()));
        if let Ok(temp) = temp_dir {
            let result = load_template("nonexistent", temp.path());
            assert!(result.is_err());
            assert!(matches!(result, Err(Error::NotFound(_))));
        }
    }
}
