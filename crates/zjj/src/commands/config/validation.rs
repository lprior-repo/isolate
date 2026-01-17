//! Configuration validation functions

use anyhow::Result;
use std::path::Path;
use zjj_core::config::Config;

use super::types::{ValidationIssue, ValidationResult};

/// Validate a config key format (zjj-audit-003)
///
/// Keys must be non-empty and follow dot notation: section.key
/// Pattern: lowercase letters/numbers/underscores, separated by dots
pub fn validate_config_key(key: &str) -> Result<()> {
    use zjj_core::contracts::Constraint;

    let trimmed = key.trim();

    if trimmed.is_empty() {
        anyhow::bail!(
            "Config key cannot be empty.\n\n\
            Use dot notation: section.key (e.g., 'zellij.use_tabs')"
        );
    }

    // Check for invalid characters and format using the contracts system
    // Must start with lowercase letter, contain only lowercase/digits/underscores,
    // and sections must be separated by single dots
    let constraint = Constraint::Regex {
        pattern: r"^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$".to_string(),
        description: "Config key must use dot notation (section.key)".to_string(),
    };

    constraint.validate_string(trimmed).map_err(|_| {
        anyhow::anyhow!(
            "Invalid config key format: '{key}'\n\n\
            Expected format: section.key (e.g., 'zellij.use_tabs')\n\
            Rules:\n\
            • Must start with a lowercase letter\n\
            • Can contain lowercase letters, numbers, underscores\n\
            • Sections separated by single dots\n\
            • No spaces or special characters"
        )
    })?;

    Ok(())
}

/// Validate size string (should be percentage or pixel value)
/// Returns a vector of validation issues found (empty if valid)
pub fn validate_size(size: &str, field: &str) -> im::Vector<ValidationIssue> {
    // Functional pipeline: collect all validation issues
    let empty_check = if size.is_empty() {
        Some(ValidationIssue {
            field: field.to_string(),
            issue: "Size value is empty".to_string(),
            suggestion: Some("Set to a percentage (e.g., '50%') or pixel value".to_string()),
        })
    } else {
        None
    };

    let is_valid = size.ends_with('%') || size.parse::<u32>().is_ok();

    // Only check format if size is not empty
    let format_check = if !size.is_empty() && !is_valid {
        Some(ValidationIssue {
            field: field.to_string(),
            issue: format!("Invalid size value '{size}'"),
            suggestion: Some(
                "Must be a percentage (e.g., '50%') or pixel value (e.g., '800')".to_string(),
            ),
        })
    } else {
        None
    };

    // Check percentage range
    let percentage_check = size
        .strip_suffix('%')
        .and_then(|pct_str| pct_str.parse::<u32>().ok())
        .filter(|&pct| pct > 100)
        .map(|pct| ValidationIssue {
            field: field.to_string(),
            issue: format!("Percentage value {pct}% is greater than 100%"),
            suggestion: Some("Set to a value between 1% and 100%".to_string()),
        });

    // Collect all issues using functional iterator chain
    [empty_check, format_check, percentage_check]
        .into_iter()
        .flatten()
        .collect()
}

/// Check if a file is readable
pub fn is_readable(path: &Path) -> bool {
    std::fs::File::open(path).is_ok()
}

/// Validate configuration and return validation result
#[allow(clippy::too_many_lines)]
#[allow(clippy::arithmetic_side_effects)]
pub fn validate_configuration(config: Option<&Config>) -> ValidationResult {
    let mut issues = im::Vector::new();
    let mut warnings = im::Vector::new();

    if let Some(config) = config {
        // Check workspace_dir is not empty
        if config.workspace_dir.is_empty() {
            issues.push_back(ValidationIssue {
                field: "workspace_dir".to_string(),
                issue: "Workspace directory cannot be empty".to_string(),
                suggestion: Some("Set to '../{repo}__workspaces' or custom path".to_string()),
            });
        }

        // Check for absolute paths in workspace_dir (warn, not error)
        if config.workspace_dir.starts_with('/') {
            warnings.push_back(ValidationIssue {
                field: "workspace_dir".to_string(),
                issue: "Using absolute path for workspace_dir".to_string(),
                suggestion: Some(
                    "Consider using relative path like '../{repo}__workspaces'".to_string(),
                ),
            });
        }

        // Validate debounce_ms range
        if config.watch.debounce_ms < 10 || config.watch.debounce_ms > 5000 {
            issues.push_back(ValidationIssue {
                field: "watch.debounce_ms".to_string(),
                issue: format!("Value {} is out of range", config.watch.debounce_ms),
                suggestion: Some("Must be between 10 and 5000 milliseconds".to_string()),
            });
        }

        // Validate refresh_ms range
        if config.dashboard.refresh_ms < 100 || config.dashboard.refresh_ms > 10000 {
            issues.push_back(ValidationIssue {
                field: "dashboard.refresh_ms".to_string(),
                issue: format!("Value {} is out of range", config.dashboard.refresh_ms),
                suggestion: Some("Must be between 100 and 10000 milliseconds".to_string()),
            });
        }

        // Validate template
        let valid_templates = ["minimal", "standard", "full"];
        if !valid_templates.contains(&config.default_template.as_str()) {
            warnings.push_back(ValidationIssue {
                field: "default_template".to_string(),
                issue: format!("Unknown template '{}'", config.default_template),
                suggestion: Some("Valid templates: minimal, standard, full".to_string()),
            });
        }

        // Validate pane sizes (should be percentages or pixel values)
        // Collect all size validation issues using functional iterator chain
        let size_validations = [
            (&config.zellij.panes.main.size, "zellij.panes.main.size"),
            (&config.zellij.panes.beads.size, "zellij.panes.beads.size"),
            (&config.zellij.panes.status.size, "zellij.panes.status.size"),
            (
                &config.zellij.panes.float.height,
                "zellij.panes.float.height",
            ),
        ];

        warnings = size_validations
            .iter()
            .fold(warnings, |acc, (size, field)| {
                acc + validate_size(size, field)
            });

        // Validate pane commands are not empty
        if config.zellij.panes.main.command.is_empty() {
            warnings.push_back(ValidationIssue {
                field: "zellij.panes.main.command".to_string(),
                issue: "Main pane command is empty".to_string(),
                suggestion: Some("Set to a command like 'claude' or 'bash'".to_string()),
            });
        }

        // Check watch paths
        if config.watch.enabled && config.watch.paths.is_empty() {
            warnings.push_back(ValidationIssue {
                field: "watch.paths".to_string(),
                issue: "Watch is enabled but no paths are configured".to_string(),
                suggestion: Some("Add paths to watch, e.g., [\".beads/beads.db\"]".to_string()),
            });
        }

        // Check dashboard columns
        if config.dashboard.columns.is_empty() {
            warnings.push_back(ValidationIssue {
                field: "dashboard.columns".to_string(),
                issue: "No dashboard columns configured".to_string(),
                suggestion: Some("Add columns like [\"name\", \"status\", \"branch\"]".to_string()),
            });
        }
    }

    ValidationResult {
        valid: issues.is_empty(),
        issues,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_size_percentage() {
        let warnings = validate_size("50%", "test.field");
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_size_pixel() {
        let warnings = validate_size("800", "test.field");
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_size_empty() {
        let warnings = validate_size("", "test.field");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].issue.contains("empty"));
    }

    #[test]
    fn test_validate_size_invalid() {
        let warnings = validate_size("invalid", "test.field");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].issue.contains("Invalid size"));
    }

    #[test]
    fn test_validate_size_percentage_over_100() {
        let warnings = validate_size("150%", "test.field");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].issue.contains("greater than 100%"));
    }

    #[test]
    fn test_is_readable_nonexistent() {
        let path = Path::new("/nonexistent/file/path.toml");
        assert!(!is_readable(path));
    }

    #[test]
    fn test_validation_issue_structure() {
        let issue = ValidationIssue {
            field: "test.field".to_string(),
            issue: "Something wrong".to_string(),
            suggestion: Some("Fix it this way".to_string()),
        };

        assert_eq!(issue.field, "test.field");
        assert_eq!(issue.issue, "Something wrong");
        assert_eq!(issue.suggestion, Some("Fix it this way".to_string()));
    }
}
