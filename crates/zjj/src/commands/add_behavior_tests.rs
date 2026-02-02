//! Brutal BDD edge-case tests for add command
//!
//! These tests follow Martin Fowler's BDD approach:
//! - Given-When-Then structure
//! - Test BEHAVIOR not implementation
//! - Expose edge cases that SHOULD break the code
//! - Validate outcomes, not internal state

#[cfg(test)]
mod brutal_edge_cases {
    use std::path::{Path, PathBuf};

    use anyhow::Error;
    use tempfile::TempDir;
    use zjj::commands::add::{execute_add, types::AddOptions};
    use zjj_core::OutputFormat;

    /// Test fixture providing isolated test environment
    struct TestRepo {
        _temp_dir: TempDir,
        root: PathBuf,
    }

    impl TestRepo {
        /// Create a new test repository with JJ + beads initialized
        fn new() -> Result<Self, anyhow::Error> {
            let temp_dir = TempDir::new()?;
            let root = temp_dir.path().to_path_buf();

            // Initialize JJ repo
            let _ = std::process::Command::new("jj")
                .args(["git", "init", "--colocate"])
                .current_dir(&root)
                .output()?;

            // Create .zjj directory structure
            std::fs::create_dir_all(root.join(".zjj"))?;
            std::fs::create_dir_all(root.join(".zjj/workspaces"))?;

            // Create .beads directory structure
            std::fs::create_dir_all(root.join(".beads"))?;

            // Create issues.jsonl with test bead
            let issues_path = root.join(".beads/issues.jsonl");
            std::fs::write(
                &issues_path,
                r#"{"id":"test-bead-1","title":"Test task","status":"open","type":"task","priority":2,"created_at":"2026-02-02T00:00:00Z","updated_at":"2026-02-02T00:00:00Z"}
"#,
            )?;

            Ok(Self {
                _temp_dir: temp_dir,
                root,
            })
        }

        fn path(&self) -> &Path {
            &self.root
        }
    }

    // ========================================================================
    // BRUTAL EDGE CASE 1: Invalid session names
    // ========================================================================

    #[test]
    fn given_empty_session_name_when_add_then_clear_error() {
        // Given: An empty session name
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        let options = AddOptions {
            session_name: "".to_string(),
            format: OutputFormat::Human,
            no_open: true,
            bead_id: None,
            category: None,
            layout: None,
        };

        // When: User adds session with empty name
        let result = execute_add(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Returns clear validation error
        assert!(result.is_err(), "Should fail for empty session name");
        let err: anyhow::Error = match result {
            Err(e) => e,
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err.to_string().contains("validation")
                || err.to_string().contains("empty")
                || err.to_string().contains("required"),
            "Error should indicate validation failure for empty name: {}",
            err
        );
    }

    #[test]
    fn given_session_name_too_long_when_add_then_clear_error() {
        // Given: A session name exceeding 64 characters
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        let long_name = "x".repeat(256);

        let options = AddOptions {
            session_name: long_name,
            format: OutputFormat::Human,
            no_open: true,
            bead_id: None,
            category: None,
            layout: None,
        };

        // When: User adds session with too-long name
        let result = execute_add(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Returns clear validation error
        assert!(result.is_err(), "Should fail for too-long session name");
        let err: anyhow::Error = match result {
            Err(e) => e,
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err.to_string().contains("validation")
                || err.to_string().contains("too long")
                || err.to_string().contains("exceeds"),
            "Error should indicate name too long: {}",
            err
        );
    }

    #[test]
    fn given_session_name_with_newline_when_add_then_clear_error() {
        // Given: A session name with embedded newline
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        let name_with_newline = "test\nsession";

        let options = AddOptions {
            session_name: name_with_newline,
            format: OutputFormat::Human,
            no_open: true,
            bead_id: None,
            category: None,
            layout: None,
        };

        // When: User adds session with newline in name
        let result = execute_add(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Returns clear validation error
        assert!(result.is_err(), "Should fail for session name with newline");
        let err: anyhow::Error = match result {
            Err(e) => e,
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err.to_string().contains("validation")
                || err.to_string().contains("newline")
                || err.to_string().contains("invalid"),
            "Error should indicate invalid name: {}",
            err
        );
    }

    // ========================================================================
    // BRUTAL EDGE CASE 2: Unicode rejection
    // ========================================================================

    #[test]
    fn given_session_name_with_unicode_null_byte_when_add_then_clear_error() {
        // Given: A session name with null byte
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        let name_with_null = String::from_utf8_lossy(&[0x00, 0x41, 0x00].to_vec());

        let options = AddOptions {
            session_name: name_with_null,
            format: OutputFormat::Human,
            no_open: true,
            bead_id: None,
            category: None,
            layout: None,
        };

        // When: User adds session with null byte
        let result = execute_add(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Returns clear validation error
        assert!(
            result.is_err(),
            "Should fail for session name with null byte"
        );
        let err: anyhow::Error = match result {
            Err(e) => e,
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err.to_string().contains("validation")
                || err.to_string().contains("invalid")
                || err.to_string().contains("unicode"),
            "Error should indicate invalid unicode: {}",
            err
        );
    }

    // ========================================================================
    // BRUTAL EDGE CASE 3: Duplicate session names
    // ========================================================================

    #[test]
    fn given_adding_same_session_twice_when_second_add_then_clear_error() {
        // Given: A session that already exists
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        // Initialize zjj first
        let init_options = AddOptions {
            session_name: "test-session".to_string(),
            format: OutputFormat::Human,
            no_open: true,
            bead_id: None,
            category: None,
            layout: None,
        };
        let _ = execute_add(&init_options);

        // Try to add same session again
        let options = AddOptions {
            session_name: "test-session".to_string(),
            format: OutputFormat::Human,
            no_open: true,
            bead_id: None,
            category: None,
            layout: None,
        };

        // When: User adds session with duplicate name
        let result = execute_add(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Returns clear duplicate error
        assert!(result.is_err(), "Should fail for duplicate session name");
        let err: anyhow::Error = match result {
            Err(e) => e,
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err.to_string().contains("exists") || err.to_string().contains("duplicate"),
            "Error should indicate duplicate session: {}",
            err
        );
    }

    // ========================================================================
    // BRUTAL EDGE CASE 4: Path traversal
    // ========================================================================

    #[test]
    fn given_session_name_with_path_traversal_when_add_then_clear_error() {
        // Given: A session name with path traversal
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        let path_traversal = "../../../etc/passwd";

        let options = AddOptions {
            session_name: path_traversal.to_string(),
            format: OutputFormat::Human,
            no_open: true,
            bead_id: None,
            category: None,
            layout: None,
        };

        // When: User adds session with path traversal
        let result = execute_add(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Returns clear validation error
        assert!(result.is_err(), "Should fail for path traversal attempt");
        let err: anyhow::Error = match result {
            Err(e) => e,
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err.to_string().contains("invalid")
                || err.to_string().contains("path")
                || err.to_string().contains("traversal"),
            "Error should indicate invalid path: {}",
            err
        );
    }

    #[test]
    fn given_session_name_with_absolute_path_when_add_then_clear_error() {
        // Given: A session name with absolute path
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        let absolute_path = "/etc/passwd";

        let options = AddOptions {
            session_name: absolute_path.to_string(),
            format: OutputFormat::Human,
            no_open: true,
            bead_id: None,
            category: None,
            layout: None,
        };

        // When: User adds session with absolute path
        let result = execute_add(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Returns clear validation error
        assert!(result.is_err(), "Should fail for absolute path");
        let err: anyhow::Error = match result {
            Err(e) => e,
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err.to_string().contains("invalid")
                || err.to_string().contains("absolute")
                || err.to_string().contains("path"),
            "Error should indicate invalid path: {}",
            err
        );
    }

    // ========================================================================
    // BRUTAL EDGE CASE 5: Special characters
    // ========================================================================

    #[test]
    fn given_session_name_with_dots_when_add_then_clear_error() {
        // Given: A session name consisting only of dots
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        let options = AddOptions {
            session_name: "...".to_string(),
            format: OutputFormat::Human,
            no_open: true,
            bead_id: None,
            category: None,
            layout: None,
        };

        // When: User adds session with dots-only name
        let result = execute_add(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Returns clear validation error
        assert!(result.is_err(), "Should fail for dots-only name");
        let err: anyhow::Error = match result {
            Err(e) => e,
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err.to_string().contains("validation") || err.to_string().contains("invalid"),
            "Error should indicate invalid name: {}",
            err
        );
    }

    #[test]
    fn given_session_name_with_slash_when_add_then_clear_error() {
        // Given: A session name starting with slash
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        let options = AddOptions {
            session_name: "/invalid".to_string(),
            format: OutputFormat::Human,
            no_open: true,
            bead_id: None,
            category: None,
            layout: None,
        };

        // When: User adds session with slash prefix
        let result = execute_add(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Returns clear validation error
        assert!(result.is_err(), "Should fail for slash prefix");
        let err: anyhow::Error = match result {
            Err(e) => e,
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err.to_string().contains("invalid")
                || err.to_string().contains("slash")
                || err.to_string().contains("path"),
            "Error should indicate invalid path: {}",
            err
        );
    }

    // ========================================================================
    // BRUTAL EDGE CASE 6: Whitespace handling
    // ========================================================================

    #[test]
    fn given_session_name_with_leading_whitespace_when_add_then_clear_error() {
        // Given: A session name with leading whitespace
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        let options = AddOptions {
            session_name: "  leading-space".to_string(),
            format: OutputFormat::Human,
            no_open: true,
            bead_id: None,
            category: None,
            layout: None,
        };

        // When: User adds session with leading whitespace
        let result = execute_add(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Returns clear validation error (should trim)
        assert!(result.is_err(), "Should fail for leading whitespace");
        let err: anyhow::Error = match result {
            Err(e) => e,
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err.to_string().contains("whitespace")
                || err.to_string().contains("trim")
                || err.to_string().contains("invalid"),
            "Error should indicate whitespace issue: {}",
            err
        );
    }

    #[test]
    fn given_session_name_with_trailing_whitespace_when_add_then_clear_error() {
        // Given: A session name with trailing whitespace
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        let options = AddOptions {
            session_name: "trailing-space  ".to_string(),
            format: OutputFormat::Human,
            no_open: true,
            bead_id: None,
            category: None,
            layout: None,
        };

        // When: User adds session with trailing whitespace
        let result = execute_add(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Returns clear validation error (should trim)
        assert!(result.is_err(), "Should fail for trailing whitespace");
        let err: anyhow::Error = match result {
            Err(e) => e,
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err.to_string().contains("whitespace")
                || err.to_string().contains("trim")
                || err.to_string().contains("invalid"),
            "Error should indicate whitespace issue: {}",
            err
        );
    }

    #[test]
    fn given_session_name_only_whitespace_when_add_then_clear_error() {
        // Given: A session name that's only whitespace
        let repo = TestRepo::new().unwrap_or_else(|_| panic!("Failed to create test repo"));
        let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::env::set_current_dir(repo.path()).ok();

        let options = AddOptions {
            session_name: "   ".to_string(),
            format: OutputFormat::Human,
            no_open: true,
            bead_id: None,
            category: None,
            layout: None,
        };

        // When: User adds session with only whitespace
        let result = execute_add(&options);

        // Cleanup
        std::env::set_current_dir(original_dir).ok();

        // Then: Returns clear validation error
        assert!(result.is_err(), "Should fail for whitespace-only name");
        let err: anyhow::Error = match result {
            Err(e) => e,
            Ok(_) => panic!("Expected error but got success"),
        };
        assert!(
            err.to_string().contains("empty")
                || err.to_string().contains("whitespace")
                || err.to_string().contains("required"),
            "Error should indicate empty/whitespace issue: {}",
            err
        );
    }
}
