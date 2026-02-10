//! Configuration viewing and editing command

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use zjj_core::{config::Config, json::SchemaEnvelope, OutputFormat};

use crate::json::{ConfigSetOutput, ConfigValueOutput};

/// File lock timeout - maximum time to wait for acquiring a file lock
const LOCK_TIMEOUT: Duration = Duration::from_secs(5);

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug)]
pub struct ConfigOptions {
    pub key: Option<String>,
    pub value: Option<String>,
    pub global: bool,
    pub format: OutputFormat,
}

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
    // Preserve error type for proper exit code mapping
    let config = zjj_core::config::load_config()
        .await
        .map_err(anyhow::Error::new)?;

    match (options.key, options.value) {
        // No key, no value: Show all config
        (None, None) => {
            show_all_config(&config, options.global, options.format)?;
        }

        // Key, no value: Show specific value
        (Some(key), None) => {
            zjj_core::config::validate_key(&key)?;
            show_config_value(&config, &key, options.format)?;
        }

        // Key + value: Set value
        (Some(key), Some(value)) => {
            zjj_core::config::validate_key(&key)?;
            let config_path = if options.global {
                global_config_path()?
            } else {
                project_config_path()?
            };
            set_config_value(&config_path, &key, &value).await?;

            if options.format.is_json() {
                let output = ConfigSetOutput {
                    key: key.clone(),
                    value: value.clone(),
                    scope: if options.global {
                        "global".to_string()
                    } else {
                        "project".to_string()
                    },
                };
                let envelope = SchemaEnvelope::new("config-set", "single", output);
                let output_json = serde_json::to_string_pretty(&envelope)
                    .unwrap_or_else(|_| r#"{"error": "serialization failed"}"#.to_string());
                println!("{output_json}");
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

// ═══════════════════════════════════════════════════════════════════════════
// VIEW OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Show all configuration
fn show_all_config(config: &Config, global_only: bool, format: OutputFormat) -> Result<()> {
    if format.is_json() {
        let envelope = SchemaEnvelope::new("config-response", "single", config.clone());
        let output_json = serde_json::to_string_pretty(&envelope)
            .unwrap_or_else(|_| r#"{"error": "serialization failed"}"#.to_string());
        println!("{output_json}");
        return Ok(());
    }

    // Serialize config to TOML
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
        println!("  4. Environment: ZJJ_* variables");
    }

    Ok(())
}

/// Show a specific config value
fn show_config_value(config: &Config, key: &str, format: OutputFormat) -> Result<()> {
    if format.is_json() {
        let json_val =
            serde_json::to_value(config).context("Failed to serialize config for value lookup")?;
        let parts: Vec<&str> = key.split('.').collect();
        let current = parts.iter().try_fold(&json_val, |current_value, &part| {
            current_value.get(part).ok_or_else(|| {
                anyhow::Error::new(zjj_core::Error::ValidationError(format!(
                    "Config key '{key}' not found"
                )))
            })
        })?;

        let output = ConfigValueOutput {
            key: key.to_string(),
            value: current.clone(),
        };
        let envelope = SchemaEnvelope::new("config-get", "single", output);
        let output_json = serde_json::to_string_pretty(&envelope)
            .unwrap_or_else(|_| r#"{"error": "serialization failed"}"#.to_string());
        println!("{output_json}");
        return Ok(());
    }

    let value = get_nested_value(config, key)?;
    println!("{key} = {value}");
    Ok(())
}

/// Get a nested value from config using dot notation
fn get_nested_value(config: &Config, key: &str) -> Result<String> {
    // Convert config to JSON for easy nested access
    let json =
        serde_json::to_value(config).context("Failed to serialize config for value lookup")?;

    let parts: Vec<&str> = key.split('.').collect();

    // Navigate through nested keys using functional fold pattern
    let current = parts.iter().try_fold(&json, |current_value, &part| {
        current_value.get(part).ok_or_else(|| {
            anyhow::Error::new(zjj_core::Error::ValidationError(format!(
                "Config key '{key}' not found. Use 'zjj config' to see all keys."
            )))
        })
    })?;

    // Format value based on type
    Ok(match current {
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => s.clone(),
        JsonValue::Array(arr) => {
            // Format as TOML array: ["a", "b"]
            let items: Vec<String> = arr
                .iter()
                .map(|v| format!("\"{}\"", v.as_str().map_or("", |s| s)))
                .collect();
            format!("[{}]", items.join(", "))
        }
        _ => serde_json::to_string_pretty(current)
            .context("Failed to format complex config value")?,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// SET OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Set a config value in the specified config file with file locking to prevent data loss
/// from concurrent writes
async fn set_config_value(config_path: &Path, key: &str, value: &str) -> Result<()> {
    use fs4::tokio::AsyncFileExt;
    use tokio::fs::OpenOptions;

    // Create parent directory if needed
    if let Some(parent) = config_path.parent() {
        tokio::fs::create_dir_all(parent).await.context(format!(
            "Failed to create config directory {}",
            parent.display()
        ))?;
    }

    // Open file with read and write access, creating if it doesn't exist
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(config_path)
        .await
        .context(format!(
            "Failed to open config file {}",
            config_path.display()
        ))?;

    // Try to acquire exclusive lock with timeout
    let mut acquired = false;
    let start = std::time::Instant::now();
    while start.elapsed() < LOCK_TIMEOUT {
        if file.try_lock_exclusive().is_ok() {
            acquired = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    if !acquired {
        anyhow::bail!(
            "Timeout waiting for file lock on {} after {} seconds. \
             Another process may be holding the lock.",
            config_path.display(),
            LOCK_TIMEOUT.as_secs()
        );
    }

    // Load existing config or create new document
    let mut doc = {
        file.seek(std::io::SeekFrom::Start(0))
            .await
            .context(format!(
                "Failed to seek config file {}",
                config_path.display()
            ))?;

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).await.context(format!(
            "Failed to read config file {}",
            config_path.display()
        ))?;

        let content = String::from_utf8(bytes).context(format!(
            "Config file contains invalid UTF-8: {}",
            config_path.display()
        ))?;

        if content.trim().is_empty() {
            toml_edit::DocumentMut::new()
        } else {
            content
                .parse::<toml_edit::DocumentMut>()
                .context("Failed to parse config file as TOML")?
        }
    };

    // Parse dot notation and set value
    let parts: Vec<&str> = key.split('.').collect();
    set_nested_value(&mut doc, &parts, value)?;

    // Write back to file (still holding the lock)
    let serialized = doc.to_string();
    file.set_len(0).await.context(format!(
        "Failed to truncate config file {}",
        config_path.display()
    ))?;
    file.seek(std::io::SeekFrom::Start(0))
        .await
        .context(format!(
            "Failed to seek config file {}",
            config_path.display()
        ))?;
    file.write_all(serialized.as_bytes())
        .await
        .context(format!(
            "Failed to write config file {}",
            config_path.display()
        ))?;
    file.flush().await.context(format!(
        "Failed to flush config file {}",
        config_path.display()
    ))?;

    // Lock is released automatically when `file` is dropped
    drop(file);

    Ok(())
}

/// Set a nested value in a TOML document using dot notation
fn set_nested_value(doc: &mut toml_edit::DocumentMut, parts: &[&str], value: &str) -> Result<()> {
    if parts.is_empty() {
        anyhow::bail!("Empty config key");
    }

    // Navigate to parent table and ensure all intermediate tables exist
    // Using fold to navigate through the path while maintaining table references
    let final_table =
        parts[..parts.len() - 1]
            .iter()
            .try_fold(doc.as_table_mut(), |current_table, &part| {
                // Ensure table exists
                if !current_table.contains_key(part) {
                    current_table[part] = toml_edit::table();
                }
                current_table[part].as_table_mut().ok_or_else(|| {
                    anyhow::Error::new(zjj_core::Error::ValidationError(format!(
                        "{part} is not a table"
                    )))
                })
            })?;

    // Set the value
    let key = parts.last().ok_or_else(|| {
        anyhow::Error::new(zjj_core::Error::ValidationError(
            "Invalid key path".to_string(),
        ))
    })?;
    let toml_value = parse_value(value)?;
    final_table[key] = toml_value;

    Ok(())
}

/// Parse a string value into a TOML value (bool, int, string, or array)
fn parse_value(value: &str) -> Result<toml_edit::Item> {
    // Try parsing as different types
    if value == "true" || value == "false" {
        let bool_val = value
            .parse::<bool>()
            .context("Failed to parse boolean value")?;
        Ok(toml_edit::value(bool_val))
    } else if let Ok(n) = value.parse::<i64>() {
        Ok(toml_edit::value(n))
    } else if value.starts_with('[') && value.ends_with(']') {
        // Parse array: ["a", "b"] or [1, 2]
        let items: Vec<&str> = value[1..value.len() - 1]
            .split(',')
            .map(|s| s.trim().trim_matches('"'))
            .collect();
        let array = items.iter().map(|s| toml_edit::Value::from(*s)).collect();
        Ok(toml_edit::Item::Value(toml_edit::Value::Array(array)))
    } else {
        // Default to string
        Ok(toml_edit::value(value))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Get path to global config file
fn global_config_path() -> Result<PathBuf> {
    directories::ProjectDirs::from("", "", "zjj")
        .map(|proj_dirs| proj_dirs.config_dir().join("config.toml"))
        .ok_or_else(|| {
            anyhow::Error::new(zjj_core::Error::IoError(
                "Failed to determine global config directory".to_string(),
            ))
        })
}

/// Get path to project config file
fn project_config_path() -> Result<PathBuf> {
    std::env::current_dir()
        .context("Failed to get current directory")
        .map(|dir| dir.join(".zjj/config.toml"))
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::TempDir;

    use super::*;

    fn setup_test_config() -> Config {
        Config::default()
    }

    fn create_temp_config(content: &str) -> Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.toml");
        let mut file = std::fs::File::create(&config_path)?;
        file.write_all(content.as_bytes())?;
        Ok((temp_dir, config_path))
    }

    // Async version for use in async test contexts
    #[allow(dead_code)]
    async fn create_temp_config_async(content: &str) -> Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.toml");
        tokio::fs::write(&config_path, content).await?;
        Ok((temp_dir, config_path))
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
    fn test_parse_value_bool_true() -> Result<()> {
        let item = parse_value("true")?;
        assert_eq!(item.to_string().trim(), "true");
        Ok(())
    }

    #[test]
    fn test_parse_value_bool_false() -> Result<()> {
        let item = parse_value("false")?;
        assert_eq!(item.to_string().trim(), "false");
        Ok(())
    }

    #[test]
    fn test_parse_value_int() -> Result<()> {
        let item = parse_value("42")?;
        assert_eq!(item.to_string().trim(), "42");
        Ok(())
    }

    #[test]
    fn test_parse_value_string() -> Result<()> {
        let item = parse_value("hello")?;
        assert_eq!(item.to_string().trim(), r#""hello""#);
        Ok(())
    }

    #[test]
    fn test_parse_value_array() -> Result<()> {
        let item = parse_value(r#"["a", "b", "c"]"#)?;
        let result = item.to_string();
        assert!(result.contains('a'));
        assert!(result.contains('b'));
        assert!(result.contains('c'));
        Ok(())
    }

    #[tokio::test]
    async fn test_set_config_value_simple() -> Result<()> {
        let (_temp_dir, config_path) = create_temp_config("")?;
        set_config_value(&config_path, "workspace_dir", "../custom").await?;

        let content = tokio::fs::read_to_string(&config_path).await?;
        assert!(content.contains("workspace_dir"));
        assert!(content.contains("../custom"));
        Ok(())
    }

    #[tokio::test]
    async fn test_set_config_value_nested() -> Result<()> {
        let (_temp_dir, config_path) = create_temp_config("")?;
        set_config_value(&config_path, "zellij.use_tabs", "false").await?;

        let content = tokio::fs::read_to_string(&config_path).await?;
        assert!(content.contains("[zellij]"));
        assert!(content.contains("use_tabs = false"));
        Ok(())
    }

    #[tokio::test]
    async fn test_set_config_value_deep_nested() -> Result<()> {
        let (_temp_dir, config_path) = create_temp_config("")?;
        set_config_value(&config_path, "zellij.panes.main.command", "nvim").await?;

        let content = tokio::fs::read_to_string(&config_path).await?;
        assert!(content.contains("[zellij.panes.main]"));
        assert!(content.contains("command"));
        assert!(content.contains("nvim"));
        Ok(())
    }

    #[tokio::test]
    async fn test_set_config_value_overwrite_existing() -> Result<()> {
        let (_temp_dir, config_path) = create_temp_config(r#"workspace_dir = "../old""#)?;
        set_config_value(&config_path, "workspace_dir", "../new").await?;

        let content = tokio::fs::read_to_string(&config_path).await?;
        assert!(content.contains("../new"));
        assert!(!content.contains("../old"));
        Ok(())
    }

    #[tokio::test]
    async fn test_set_config_value_creates_parent_dir() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("subdir").join("config.toml");

        set_config_value(&config_path, "workspace_dir", "../test").await?;

        assert!(config_path.exists());
        let content = tokio::fs::read_to_string(&config_path).await?;
        assert!(content.contains("workspace_dir"));
        Ok(())
    }

    #[test]
    fn test_set_nested_value_empty_parts() {
        let mut doc = toml_edit::DocumentMut::new();
        let result = set_nested_value(&mut doc, &[], "value");
        let has_error = result
            .as_ref()
            .map_or_else(|e| e.to_string().contains("Empty config key"), |()| false);
        assert!(has_error);
    }

    #[test]
    fn test_show_config_value() -> Result<()> {
        let config = setup_test_config();
        // Just test that it doesn't panic
        show_config_value(&config, "workspace_dir", zjj_core::OutputFormat::Human)?;
        Ok(())
    }

    #[test]
    fn test_show_all_config() -> Result<()> {
        let config = setup_test_config();
        // Just test that it doesn't panic
        show_all_config(&config, false, zjj_core::OutputFormat::Human)?;
        show_all_config(&config, true, zjj_core::OutputFormat::Human)?;
        Ok(())
    }

    #[test]
    fn test_project_config_path() -> Result<()> {
        let path = project_config_path()?;
        assert!(path.ends_with("config.toml"));
        assert!(path.to_string_lossy().contains(".zjj"));
        Ok(())
    }

    // ===== PHASE 2 (RED): SchemaEnvelope Wrapping Tests =====
    // These tests FAIL initially - they verify envelope structure and format
    // Implementation in Phase 4 (GREEN) will make them pass

    #[test]
    fn test_config_json_has_envelope() -> Result<()> {
        // FAILING: Verify envelope wrapping for config command output
        use zjj_core::json::SchemaEnvelope;
        let config = setup_test_config();
        let envelope = SchemaEnvelope::new("config-response", "single", config);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert_eq!(
            parsed.get("_schema_version").and_then(|v| v.as_str()),
            Some("1.0")
        );
        assert_eq!(
            parsed.get("schema_type").and_then(|v| v.as_str()),
            Some("single")
        );
        assert!(parsed.get("success").is_some(), "Missing success field");

        Ok(())
    }

    #[test]
    fn test_config_set_wrapped() -> Result<()> {
        // FAILING: Verify envelope wrapping when setting config values
        use serde_json::json;
        use zjj_core::json::SchemaEnvelope;

        let response = json!({"success": true, "key": "test.key", "value": "test_value"});
        let envelope = SchemaEnvelope::new("config-set", "single", response);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert_eq!(
            parsed.get("schema_type").and_then(|v| v.as_str()),
            Some("single")
        );

        Ok(())
    }

    #[test]
    fn test_config_get_wrapped() -> Result<()> {
        // FAILING: Verify envelope wrapping when getting config values
        use serde_json::json;
        use zjj_core::json::SchemaEnvelope;

        let response = json!({"value": "config_value"});
        let envelope = SchemaEnvelope::new("config-get", "single", response);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert!(parsed.get("success").is_some(), "Missing success field");

        Ok(())
    }

    #[test]
    fn test_config_data_preserved() -> Result<()> {
        // FAILING: Verify config data is preserved inside envelope
        use zjj_core::json::SchemaEnvelope;

        let config = setup_test_config();
        let envelope = SchemaEnvelope::new("config-response", "single", config);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        // Verify data is flattened into envelope (not nested in a "data" field)
        let has_config_fields =
            parsed.get("workspace_dir").is_some() || parsed.get("zellij").is_some();
        assert!(
            has_config_fields,
            "Config data should be preserved in envelope"
        );

        Ok(())
    }

    // ===== Concurrency Tests (zjj-16ks) =====
    // Tests for concurrent write data loss bug fix

    #[tokio::test]
    async fn concurrent_writes_respect_file_lock() -> Result<()> {
        let (_temp_dir, config_path) = create_temp_config("")?;
        let mut tasks = Vec::new();

        // Spawn 10 concurrent writes
        for i in 0..10 {
            let path = config_path.clone();
            let task = tokio::spawn(async move {
                let key = format!("concurrent_key_{i}");
                set_config_value(&path, &key, &format!("value_{i}")).await
            });
            tasks.push(task);
        }

        // Wait for all tasks
        for task in tasks {
            task.await
                .context("Task join error")?
                .context("Task execution error")?;
        }

        // Verify all keys present in the final config
        let content = tokio::fs::read_to_string(&config_path).await?;
        for i in 0..10 {
            let key = format!("concurrent_key_{i}");
            assert!(
                content.contains(&key),
                "Key {key} not found in config after concurrent writes"
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn concurrent_writes_no_data_loss() -> Result<()> {
        let (_temp_dir, config_path) = create_temp_config("")?;
        let num_writes = 20;
        let mut tasks = Vec::new();

        // Spawn 20 concurrent writes
        for i in 0..num_writes {
            let path = config_path.clone();
            let task = tokio::spawn(async move {
                let key = format!("data_loss_test_key_{i:03}");
                set_config_value(&path, &key, &format!("value_{i}")).await
            });
            tasks.push(task);
        }

        // Wait for all tasks
        for task in tasks {
            task.await
                .context("Task join error")?
                .context("Task execution error")?;
        }

        // Count how many unique keys are in the file
        let content = tokio::fs::read_to_string(&config_path).await?;
        let mut key_count = 0;
        for i in 0..num_writes {
            let key = format!("data_loss_test_key_{i:03}");
            if content.contains(&key) {
                key_count += 1;
            }
        }

        assert_eq!(
            key_count,
            num_writes,
            "Expected {} keys, got {} (data loss: {})",
            num_writes,
            key_count,
            num_writes - key_count
        );
        Ok(())
    }

    #[tokio::test]
    async fn sequential_writes_performance() -> Result<()> {
        let (_temp_dir, config_path) = create_temp_config("")?;
        let start = std::time::Instant::now();

        for i in 0..50 {
            let key = format!("perf_key_{i}");
            set_config_value(&config_path, &key, &format!("value_{i}")).await?;
        }

        let elapsed = start.elapsed();
        // Should complete in reasonable time (file locking adds overhead)
        assert!(
            elapsed.as_secs() < 30,
            "50 writes took too long: {elapsed:?}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn concurrent_mixed_read_write() -> Result<()> {
        let (_temp_dir, config_path) = create_temp_config("")?;
        let mut write_tasks = Vec::new();
        let mut read_tasks = Vec::new();

        // First, set some initial values
        for i in 0..5 {
            let key = format!("initial_key_{i}");
            set_config_value(&config_path, &key, &format!("initial_value_{i}")).await?;
        }

        // Spawn concurrent readers and writers
        for i in 0..10 {
            let path = config_path.clone();
            if i % 2 == 0 {
                // Writer task
                let task = tokio::spawn(async move {
                    let key = format!("mixed_write_key_{i}");
                    set_config_value(&path, &key, &format!("value_{i}")).await
                });
                write_tasks.push(task);
            } else {
                // Reader task (verify file is readable)
                let task = tokio::spawn(async move { tokio::fs::read_to_string(&path).await });
                read_tasks.push(task);
            }
        }

        // All write tasks should complete without error
        for task in write_tasks {
            task.await
                .context("Writer join error")?
                .context("Writer execution error")?;
        }

        // All read tasks should complete
        for task in read_tasks {
            task.await
                .context("Reader join error")?
                .context("Reader execution error")?;
        }
        Ok(())
    }
}
