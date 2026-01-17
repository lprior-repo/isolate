//! Defaults-focused tests for configuration
//!
//! Tests for default configuration values, initialization, and merging behavior.

#[cfg(test)]
mod defaults_tests {
    use crate::config::Config;

    // Test 2: Global only - Loads global, merges with defaults
    #[test]
    fn test_global_only_merges_with_defaults() {
        // For this test, we're testing the merge logic directly, not the file loading
        let base = Config::default();
        let override_config = Config {
            workspace_dir: "../custom".to_string(),
            ..Default::default()
        };

        let result = base.merge(override_config);

        assert_eq!(result.workspace_dir, "../custom");
        assert_eq!(result.default_template, "standard"); // Should still have default
    }

    // Test 3: Project only - Loads project, merges with defaults
    #[test]
    fn test_project_only_merges_with_defaults() {
        let base = Config::default();
        let override_config = Config {
            main_branch: Some("develop".to_string()),
            ..Default::default()
        };

        let result = base.merge(override_config);

        assert_eq!(result.main_branch, Some("develop".to_string()));
        assert_eq!(result.workspace_dir, "../{repo}__workspaces"); // Should still have default
    }

    // Test 4: Both - Project overrides global overrides defaults
    #[test]
    fn test_project_overrides_global() {
        let base = Config::default();

        // First merge global
        let global_config = Config {
            workspace_dir: "../global".to_string(),
            ..Default::default()
        };
        let result = base.merge(global_config);
        assert_eq!(result.workspace_dir, "../global");

        // Then merge project (should override)
        let project_config = Config {
            workspace_dir: "../project".to_string(),
            ..Default::default()
        };
        let final_result = result.merge(project_config);

        assert_eq!(final_result.workspace_dir, "../project");
    }

    // Test 11: Partial config - Unspecified values use defaults
    #[test]
    fn test_partial_config_uses_defaults() {
        let base = Config::default();
        let partial = Config {
            workspace_dir: "../custom".to_string(),
            ..Default::default()
        };
        // All other fields remain default

        let result = base.merge(partial);

        assert_eq!(result.workspace_dir, "../custom");
        assert_eq!(result.default_template, "standard"); // Still default
        assert!(result.watch.enabled); // Still default
    }

    // Test 12: Deep merge - hooks.post_create in global + project → project replaces
    #[test]
    fn test_deep_merge_replaces_not_appends() {
        let mut base = Config::default();
        base.hooks.post_create = vec!["a".to_string(), "b".to_string()];

        let mut override_config = Config::default();
        override_config.hooks.post_create = vec!["c".to_string()];

        let result = base.merge(override_config);

        assert_eq!(result.hooks.post_create, vec!["c".to_string()]);
        assert_ne!(
            result.hooks.post_create,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn test_default_config_values() {
        let config = Config::default();
        assert_eq!(config.workspace_dir, "../{repo}__workspaces");
        assert_eq!(config.main_branch, None);
        assert_eq!(config.default_template, "standard");
        assert_eq!(config.state_db, ".jjz/state.db");
        assert!(config.watch.enabled);
        assert_eq!(config.watch.debounce_ms, 100);
        assert_eq!(config.dashboard.refresh_ms, 1000);
        assert_eq!(config.zellij.session_prefix, "jjz");
    }
}
