//! Config loading and viewing operations

use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use zjj_core::config::Config;

use crate::json_output::{ConfigGetOutput, ConfigViewAllOutput};

/// Show all configuration
pub fn show_all_config(config: &Config, global_only: bool, json: bool) -> Result<()> {
    if json {
        // JSON output
        let config_value =
            serde_json::to_value(config).context("Failed to serialize config to JSON")?;

        let sources = if global_only {
            None
        } else {
            // Functional approach: chain all config sources
            let sources_list = std::iter::once("Built-in defaults".to_string())
                .chain(
                    global_config_path()
                        .ok()
                        .map(|p| format!("Global: {}", p.display())),
                )
                .chain(
                    project_config_path()
                        .ok()
                        .map(|p| format!("Project: {}", p.display())),
                )
                .chain(std::iter::once("Environment: JJZ_* variables".to_string()))
                .collect::<Vec<_>>();
            Some(sources_list)
        };

        let output = ConfigViewAllOutput {
            success: true,
            config: config_value,
            sources,
            error: None,
        };

        println!(
            "{}",
            serde_json::to_string_pretty(&output).context("Failed to serialize JSON output")?
        );
    } else {
        // Human-readable TOML output
        let toml = toml::to_string_pretty(config).context("Failed to serialize config to TOML")?;

        println!(
            "Current configuration{}:",
            if global_only {
                " (global)"
            } else {
                " (merged)"
            }
        );
        println!();
        println!("{toml}");

        if !global_only {
            println!();
            println!("Config sources:");
            println!("  1. Built-in defaults");
            if let Ok(global_path) = global_config_path() {
                println!("  2. Global: {}", global_path.display());
            }
            if let Ok(project_path) = project_config_path() {
                println!("  3. Project: {}", project_path.display());
            }
            println!("  4. Environment: JJZ_* variables");
        }
    }

    Ok(())
}

/// Show a specific config value
pub fn show_config_value(config: &Config, key: &str, json: bool) -> Result<()> {
    let value = get_nested_value(config, key)?;

    if json {
        let output = ConfigGetOutput {
            success: true,
            key: key.to_string(),
            value: Some(value),
            error: None,
        };
        println!(
            "{}",
            serde_json::to_string(&output).context("Failed to serialize JSON output")?
        );
    } else {
        println!("{key} = {value}");
    }

    Ok(())
}

/// Get a nested value from config using dot notation
pub fn get_nested_value(config: &Config, key: &str) -> Result<String> {
    // Convert config to JSON for easy nested access
    let json =
        serde_json::to_value(config).context("Failed to serialize config for value lookup")?;

    let parts: im::Vector<&str> = key.split('.').collect();

    // Navigate through nested keys using functional fold pattern
    let current = parts.iter().try_fold(&json, |current_value, &part| {
        current_value.get(part).ok_or_else(|| {
            anyhow::anyhow!("Config key '{key}' not found. Use 'jjz config' to see all keys.")
        })
    })?;

    // Format value based on type
    Ok(match current {
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => s.clone(),
        JsonValue::Array(arr) => {
            // Format as TOML array: ["a", "b"]
            let items: im::Vector<String> = arr
                .iter()
                .map(|v| format!("\"{}\"", v.as_str().unwrap_or("")))
                .collect();
            format!("[{}]", items.iter().cloned().collect::<Vec<_>>().join(", "))
        }
        _ => serde_json::to_string_pretty(current)
            .context("Failed to format complex config value")?,
    })
}

/// Get path to global config file
pub fn global_config_path() -> Result<PathBuf> {
    directories::ProjectDirs::from("", "", "jjz")
        .map(|proj_dirs| proj_dirs.config_dir().join("config.toml"))
        .ok_or_else(|| anyhow::anyhow!("Failed to determine global config directory"))
}

/// Get path to project config file
pub fn project_config_path() -> Result<PathBuf> {
    std::env::current_dir()
        .context("Failed to get current directory")
        .map(|dir| dir.join(".zjj/config.toml"))
}

/// Get path to global config file (returns Option)
pub fn global_config_path_opt() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "jjz")
        .map(|proj_dirs| proj_dirs.config_dir().join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_config() -> Config {
        Config::default()
    }

    #[test]
    fn test_get_nested_value_simple() -> Result<()> {
        let config = setup_test_config();
        let value = get_nested_value(&config, "workspace_dir")?;
        assert_eq!(value, "../{repo}__workspaces");
        Ok(())
    }

    #[test]
    fn test_get_nested_value_nested() -> Result<()> {
        let config = setup_test_config();
        let value = get_nested_value(&config, "zellij.use_tabs")?;
        assert_eq!(value, "true");
        Ok(())
    }

    #[test]
    fn test_get_nested_value_deep() -> Result<()> {
        let config = setup_test_config();
        let value = get_nested_value(&config, "zellij.panes.main.command")?;
        assert_eq!(value, "claude");
        Ok(())
    }

    #[test]
    fn test_get_nested_value_not_found() {
        let config = setup_test_config();
        let result = get_nested_value(&config, "invalid.key");
        assert!(result.is_err(), "Expected an error but got Ok: {result:?}");
        if let Err(e) = result {
            assert!(e.to_string().contains("Config key 'invalid.key' not found"));
        }
    }

    #[test]
    fn test_get_nested_value_array() -> Result<()> {
        let config = setup_test_config();
        let value = get_nested_value(&config, "watch.paths")?;
        assert_eq!(value, r#"[".beads/beads.db"]"#);
        Ok(())
    }

    #[test]
    fn test_show_config_value() -> Result<()> {
        let config = setup_test_config();
        // Just test that it doesn't panic (non-JSON mode)
        show_config_value(&config, "workspace_dir", false)?;
        Ok(())
    }

    #[test]
    fn test_show_config_value_json() -> Result<()> {
        let config = setup_test_config();
        // Test JSON output doesn't panic and produces valid JSON
        show_config_value(&config, "workspace_dir", true)?;
        Ok(())
    }

    #[test]
    fn test_show_all_config() -> Result<()> {
        let config = setup_test_config();
        // Just test that it doesn't panic (non-JSON mode)
        show_all_config(&config, false, false)?;
        show_all_config(&config, true, false)?;
        Ok(())
    }

    #[test]
    fn test_show_all_config_json() -> Result<()> {
        let config = setup_test_config();
        // Test JSON output doesn't panic (both global and merged views)
        show_all_config(&config, false, true)?;
        show_all_config(&config, true, true)?;
        Ok(())
    }

    #[test]
    fn test_project_config_path() -> Result<()> {
        let path = project_config_path()?;
        assert!(path.ends_with("config.toml"));
        assert!(path.to_string_lossy().contains(".zjj"));
        Ok(())
    }

    #[test]
    fn test_global_config_path_opt() {
        let path = global_config_path_opt();
        // Should return Some on most systems
        if let Some(p) = path {
            assert!(p.ends_with("config.toml"));
        }
    }
}
