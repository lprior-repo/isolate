//! Configuration viewing and editing command

pub mod defaults;
pub mod loading;
pub mod types;
pub mod validation;

use anyhow::{Context, Result};

use crate::json_output::ConfigSetOutput;

pub use defaults::set_config_value;
pub use loading::{
    global_config_path, global_config_path_opt, project_config_path,
    show_all_config, show_config_value,
};
pub use types::ConfigOptions;
pub use validation::{is_readable, validate_config_key};

/// Execute the config command
///
/// # Errors
///
/// Returns error if:
/// - Config file cannot be read or parsed
/// - Config key is not found
/// - Config value cannot be set
/// - Invalid arguments provided
pub async fn run(options: ConfigOptions) -> Result<()> {
    // Yield to make function legitimately async
    tokio::task::yield_now().await;

    // Wrap the entire function to handle errors in JSON mode
    run_internal(&options).inspect_err(|e| {
        if options.json {
            // Output error as JSON and still return the error
            output_error_json(&e.to_string());
        }
    })
}

/// Internal implementation of the config command
fn run_internal(options: &ConfigOptions) -> Result<()> {
    // Handle validate flag first
    if options.validate {
        return run_validate_internal(options.json);
    }

    // Validate key if provided (zjj-audit-003)
    if let Some(key) = &options.key {
        validate_config_key(key)?;
    }

    let config = zjj_core::config::load_config()
        .map_err(|e| anyhow::anyhow!("Failed to load config: {e}"))?;

    match (&options.key, &options.value) {
        // No key, no value: Show all config
        (None, None) => {
            show_all_config(&config, options.global, options.json)?;
        }

        // Key, no value: Show specific value
        (Some(key), None) => {
            show_config_value(&config, key, options.json)?;
        }

        // Key + value: Set value
        (Some(key), Some(value)) => {
            let config_path = if options.global {
                global_config_path()?
            } else {
                project_config_path()?
            };
            set_config_value(&config_path, key, value)?;

            if options.json {
                let output = ConfigSetOutput {
                    success: true,
                    key: key.clone(),
                    value: value.clone(),
                    scope: Some(if options.global {
                        "global".to_string()
                    } else {
                        "project".to_string()
                    }),
                    error: None,
                };
                println!(
                    "{}",
                    serde_json::to_string(&output).unwrap_or_else(|_| format!(
                        r#"{{"success":true,"key":"{key}","value":"{value}"}}"#
                    ))
                );
            } else {
                println!("✓ Set {key} = {value}");
                if options.global {
                    println!("  (in global config)");
                } else {
                    println!("  (in project config)");
                }
            }
        }

        // Value without key: Invalid
        (None, Some(_)) => {
            anyhow::bail!("Cannot set value without key");
        }
    }

    Ok(())
}

/// Validate configuration and report issues
fn run_validate_internal(json: bool) -> Result<()> {
    // Try to load config and collect validation errors
    let config_result = zjj_core::config::load_config().ok();

    // Check if config files exist and are readable
    let mut validation = validation::validate_configuration(config_result.as_ref());

    if let Some(global_path) = global_config_path_opt() {
        if global_path.exists() && !is_readable(&global_path) {
            validation.issues.push_back(types::ValidationIssue {
                field: "global_config".to_string(),
                issue: format!("Cannot read global config file: {}", global_path.display()),
                suggestion: Some("Check file permissions".to_string()),
            });
        }
    }

    if let Ok(project_path) = project_config_path() {
        if project_path.exists() && !is_readable(&project_path) {
            validation.issues.push_back(types::ValidationIssue {
                field: "project_config".to_string(),
                issue: format!(
                    "Cannot read project config file: {}",
                    project_path.display()
                ),
                suggestion: Some("Check file permissions".to_string()),
            });
        }
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&validation)
                .context("Failed to serialize validation result")?
        );
    } else {
        print_validation_result(&validation);
    }

    if validation.valid {
        Ok(())
    } else {
        anyhow::bail!("Configuration validation failed")
    }
}

/// Print validation result in human-readable format
fn print_validation_result(result: &types::ValidationResult) {
    if result.valid && result.warnings.is_empty() {
        println!("✓ Configuration is valid");
        return;
    }

    if !result.issues.is_empty() {
        println!("✗ Configuration validation failed\n");
        println!("Issues:");
        result.issues.iter().for_each(|issue| {
            println!("  • {}: {}", issue.field, issue.issue);
            if let Some(suggestion) = &issue.suggestion {
                println!("    → {suggestion}");
            }
        });
        println!();
    }

    if !result.warnings.is_empty() {
        println!("Warnings:");
        result.warnings.iter().for_each(|warning| {
            println!("  ⚠ {}: {}", warning.field, warning.issue);
            if let Some(suggestion) = &warning.suggestion {
                println!("    → {suggestion}");
            }
        });
        println!();
    }

    if result.valid {
        println!("✓ Configuration is valid (with warnings)");
    }
}

/// Helper to output error as JSON
fn output_error_json(error_msg: &str) {
    let output = serde_json::json!({
        "success": false,
        "error": error_msg,
    });
    // Use fallback if serialization somehow fails
    match serde_json::to_string(&output) {
        Ok(json) => println!("{json}"),
        Err(_) => println!(
            r#"{{"success":false,"error":"{}"}}"#,
            error_msg.replace('"', "\\\"")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_options_with_validate() {
        let options = ConfigOptions {
            key: None,
            value: None,
            global: false,
            json: false,
            validate: true,
        };

        assert!(options.validate);
        assert!(!options.global);
        assert!(!options.json);
    }

    #[test]
    fn test_validation_result_serialization() -> Result<()> {
        let result = types::ValidationResult {
            valid: true,
            issues: im::Vector::new(),
            warnings: im::vector![types::ValidationIssue {
                field: "test".to_string(),
                issue: "test issue".to_string(),
                suggestion: Some("test suggestion".to_string()),
            }],
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("valid"));
        assert!(json.contains("test issue"));
        Ok(())
    }
}
