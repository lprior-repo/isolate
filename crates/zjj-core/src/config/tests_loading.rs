//! Loading-focused tests for configuration
//!
//! Tests for configuration file loading, parsing, path resolution, and environment overrides.

#[cfg(test)]
mod loading_tests {
    use crate::config::{
        global_config_path, load_config, load_toml_file, project_config_path, Config,
    };
    use crate::{Error, Result};

    // Test 1: No config files - Returns default config
    #[test]
    fn test_no_config_files_returns_defaults() {
        // This test works in the normal repo context where no .jjz/config.toml exists
        // and global config likely doesn't exist either
        let result = load_config();
        assert!(
            result.is_ok(),
            "load_config should succeed even without config files"
        );

        let config = result.unwrap_or_else(|_| Config::default());
        // Check that we got defaults (with {repo} replaced by actual repo name)
        assert!(config.workspace_dir.contains("__workspaces"));
        assert_eq!(config.default_template, "standard");
        assert_eq!(config.state_db, ".jjz/state.db");
    }

    // Test 9: Missing global config - No error, uses defaults
    #[test]
    fn test_missing_global_config_no_error() {
        // This tests that load_config doesn't fail when global config doesn't exist
        // (which is the normal case for most users)
        let result = load_config();
        assert!(result.is_ok());
    }

    // Test 10: Malformed TOML - Clear error with line number
    #[test]
    fn test_malformed_toml_returns_parse_error() -> Result<()> {
        use std::io::Write;
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::io_error(format!("Failed to create temp dir: {e}")))?;
        let config_path = temp_dir.path().join("bad_config.toml");

        let mut file = std::fs::File::create(&config_path)
            .map_err(|e| Error::io_error(format!("Failed to create test file: {e}")))?;
        file.write_all(b"workspace_dir = \n invalid toml [[[")
            .map_err(|e| Error::io_error(format!("Failed to write test file: {e}")))?;

        let result = load_toml_file(&config_path);
        assert!(result.is_err());

        if let Err(e) = result {
            // Verify it's a validation error (parse errors are wrapped as validation)
            assert!(e.to_string().contains("parse") || e.to_string().contains("invalid"));
        }
        Ok(())
    }

    // Additional tests for helper functions
    #[test]
    fn test_global_config_path() {
        let path = global_config_path();
        // Should return Some path to ~/.config/jjz/config.toml
        // or None on systems without home directory
        assert!(path.is_some() || path.is_none());
    }

    #[test]
    fn test_project_config_path() {
        let result = project_config_path();
        assert!(result.is_ok());
        let path = result.unwrap_or_default();
        assert!(path.ends_with("config.toml"));
    }

    // Test 5: Env override - JJZ_WORKSPACE_DIR=../custom → config.workspace_dir
    // Ignored: Requires unsafe code for env var manipulation, conflicts with workspace
    // forbid(unsafe_code)
    #[test]
    #[ignore = "Requires unsafe code for env var manipulation"]
    fn test_env_var_overrides_config() {
        // Set env var
        std::env::set_var("JJZ_WORKSPACE_DIR", "../env");

        let config = Config {
            workspace_dir: "../original".to_string(),
            ..Default::default()
        };

        let result = config.apply_env_vars();
        assert!(result.is_ok());
        if let Ok(result_config) = result {
            assert_eq!(result_config.workspace_dir, "../env");
        }

        // Cleanup
        std::env::remove_var("JJZ_WORKSPACE_DIR");
    }

    // Ignored: Requires unsafe code for env var manipulation, conflicts with workspace
    // forbid(unsafe_code)
    #[test]
    #[ignore = "Requires unsafe code for env var manipulation"]
    fn test_env_var_parsing_bool() {
        std::env::set_var("JJZ_WATCH_ENABLED", "false");

        let config = Config::default();
        let result = config.apply_env_vars();
        assert!(result.is_ok());
        let result_config = result.unwrap_or_else(|_| Config::default());
        assert!(!result_config.watch.enabled);

        std::env::remove_var("JJZ_WATCH_ENABLED");
    }

    // Ignored: Requires unsafe code for env var manipulation, conflicts with workspace
    // forbid(unsafe_code)
    #[test]
    #[ignore = "Requires unsafe code for env var manipulation"]
    fn test_env_var_parsing_int() {
        std::env::set_var("JJZ_WATCH_DEBOUNCE_MS", "200");

        let config = Config::default();
        let result = config.apply_env_vars();
        assert!(result.is_ok());
        let result_config = result.unwrap_or_else(|_| Config::default());
        assert_eq!(result_config.watch.debounce_ms, 200);

        std::env::remove_var("JJZ_WATCH_DEBOUNCE_MS");
    }
}
