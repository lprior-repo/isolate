//! Validation-focused tests for configuration
//!
//! Tests for configuration validation rules, range checks, and placeholder substitution.

#[cfg(test)]
mod validation_tests {
    use crate::config::Config;

    // Test 6: Placeholder substitution
    #[test]
    fn test_placeholder_substitution() {
        let config = Config {
            workspace_dir: "../{repo}__ws".to_string(),
            ..Default::default()
        };
        let result = config.substitute_placeholders();
        assert!(result.is_ok());
        if let Ok(result_config) = result {
            // The repo name will be "zjj" since we're in the zjj directory
            assert!(result_config.workspace_dir.contains("__ws"));
            assert!(!result_config.workspace_dir.contains("{repo}"));
        }
    }

    // Test 7: Invalid debounce - debounce_ms = 5 → Error
    #[test]
    fn test_invalid_debounce_ms_too_low() {
        let mut config = Config::default();
        config.watch.debounce_ms = 5;

        let result = config.validate();
        assert!(result.is_err());

        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(error_msg.contains("10-5000"));
        }
    }

    // Test 8: Invalid refresh - refresh_ms = 50000 → Error
    #[test]
    fn test_invalid_refresh_ms_too_high() {
        let mut config = Config::default();
        config.dashboard.refresh_ms = 50000;

        let result = config.validate();
        assert!(result.is_err());

        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(error_msg.contains("100-10000"));
        }
    }

    #[test]
    fn test_validation_debounce_ms_valid() {
        let mut config = Config::default();
        config.watch.debounce_ms = 100;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_debounce_ms_min() {
        let mut config = Config::default();
        config.watch.debounce_ms = 10;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_debounce_ms_max() {
        let mut config = Config::default();
        config.watch.debounce_ms = 5000;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_refresh_ms_valid() {
        let mut config = Config::default();
        config.dashboard.refresh_ms = 1000;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_refresh_ms_min() {
        let mut config = Config::default();
        config.dashboard.refresh_ms = 100;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_refresh_ms_max() {
        let mut config = Config::default();
        config.dashboard.refresh_ms = 10000;
        assert!(config.validate().is_ok());
    }
}
