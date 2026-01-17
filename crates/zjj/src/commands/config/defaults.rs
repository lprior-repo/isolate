//! Configuration value setting and parsing operations

use anyhow::{Context, Result};
use std::path::Path;

/// Set a config value in the specified config file
pub fn set_config_value(config_path: &Path, key: &str, value: &str) -> Result<()> {
    // Load existing config or create new
    let mut doc = if config_path.exists() {
        let content = std::fs::read_to_string(config_path).context(format!(
            "Failed to read config file {}",
            config_path.display()
        ))?;
        content
            .parse::<toml_edit::DocumentMut>()
            .context("Failed to parse config file as TOML")?
    } else {
        // Create parent directory if needed
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).context(format!(
                "Failed to create config directory {}",
                parent.display()
            ))?;
        }
        toml_edit::DocumentMut::new()
    };

    // Parse dot notation and set value
    let parts: im::Vector<&str> = key.split('.').collect();
    set_nested_value(&mut doc, &parts.iter().copied().collect::<Vec<_>>(), value)?;

    // Write back to file
    std::fs::write(config_path, doc.to_string()).context(format!(
        "Failed to write config file {}",
        config_path.display()
    ))?;

    Ok(())
}

/// Set a nested value in a TOML document using dot notation
pub fn set_nested_value(
    doc: &mut toml_edit::DocumentMut,
    parts: &[&str],
    value: &str,
) -> Result<()> {
    if parts.is_empty() {
        anyhow::bail!("Empty config key");
    }

    // Navigate to parent table and ensure all intermediate tables exist
    // Using fold to navigate through the path while maintaining table references
    let final_table = parts[..parts.len().saturating_sub(1)].iter().try_fold(
        doc.as_table_mut(),
        |current_table, &part| {
            // Ensure table exists
            if !current_table.contains_key(part) {
                current_table[part] = toml_edit::table();
            }
            current_table[part]
                .as_table_mut()
                .ok_or_else(|| anyhow::anyhow!("{part} is not a table"))
        },
    )?;

    // Set the value
    let key = parts
        .last()
        .ok_or_else(|| anyhow::anyhow!("Invalid key path"))?;
    let toml_value = parse_value(value)?;
    final_table[key] = toml_value;

    Ok(())
}

/// Parse a string value into a TOML value (bool, int, string, or array)
pub fn parse_value(value: &str) -> Result<toml_edit::Item> {
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
        let items: im::Vector<&str> = value[1..value.len().saturating_sub(1)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_temp_config(content: &str) -> Result<(TempDir, std::path::PathBuf)> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.toml");
        let mut file = std::fs::File::create(&config_path)?;
        file.write_all(content.as_bytes())?;
        Ok((temp_dir, config_path))
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

    #[test]
    fn test_set_config_value_simple() -> Result<()> {
        let (_temp_dir, config_path) = create_temp_config("")?;
        set_config_value(&config_path, "workspace_dir", "../custom")?;

        let content = std::fs::read_to_string(&config_path)?;
        assert!(content.contains("workspace_dir"));
        assert!(content.contains("../custom"));
        Ok(())
    }

    #[test]
    fn test_set_config_value_nested() -> Result<()> {
        let (_temp_dir, config_path) = create_temp_config("")?;
        set_config_value(&config_path, "zellij.use_tabs", "false")?;

        let content = std::fs::read_to_string(&config_path)?;
        assert!(content.contains("[zellij]"));
        assert!(content.contains("use_tabs = false"));
        Ok(())
    }

    #[test]
    fn test_set_config_value_deep_nested() -> Result<()> {
        let (_temp_dir, config_path) = create_temp_config("")?;
        set_config_value(&config_path, "zellij.panes.main.command", "nvim")?;

        let content = std::fs::read_to_string(&config_path)?;
        assert!(content.contains("[zellij.panes.main]"));
        assert!(content.contains("command"));
        assert!(content.contains("nvim"));
        Ok(())
    }

    #[test]
    fn test_set_config_value_overwrite_existing() -> Result<()> {
        let (_temp_dir, config_path) = create_temp_config(r#"workspace_dir = "../old""#)?;
        set_config_value(&config_path, "workspace_dir", "../new")?;

        let content = std::fs::read_to_string(&config_path)?;
        assert!(content.contains("../new"));
        assert!(!content.contains("../old"));
        Ok(())
    }

    #[test]
    fn test_set_config_value_creates_parent_dir() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("subdir").join("config.toml");

        set_config_value(&config_path, "workspace_dir", "../test")?;

        assert!(config_path.exists());
        let content = std::fs::read_to_string(&config_path)?;
        assert!(content.contains("workspace_dir"));
        Ok(())
    }

    #[test]
    fn test_set_nested_value_empty_parts() {
        let mut doc = toml_edit::DocumentMut::new();
        let result = set_nested_value(&mut doc, &[], "value");
        let has_error = result
            .as_ref()
            .map(|()| false)
            .unwrap_or_else(|e| e.to_string().contains("Empty config key"));
        assert!(has_error);
    }
}
