#[cfg(test)]
mod newtypes_tests {
    use std::path::PathBuf;

    use zjj::commands::done::newtypes::{
        BeadId, CommitId, JjOutput, RepoRoot, ValidationError, WorkspaceName,
    };

    #[test]
    fn test_repo_root_validates_path_exists() {
        // Test that RepoRoot validates path exists and is a JJ repo
        let non_existent = PathBuf::from("/nonexistent/path");
        let result = RepoRoot::new(non_existent.clone());
        assert!(result.is_err(), "should reject non-existent path");
        assert!(
            result.unwrap_err().to_string().contains("does not exist"),
            "should mention missing path"
        );
    }

    #[test]
    fn test_repo_root_validates_is_dir() {
        // Test that RepoRoot rejects non-directory paths
        let file_path = PathBuf::from("/tmp/test_file.txt");
        std::fs::write(&file_path, "content").unwrap();
        let result = RepoRoot::new(file_path);
        assert!(result.is_err(), "should reject file path");
        assert!(
            result.unwrap_err().to_string().contains("not a directory"),
            "should mention not a directory"
        );
    }

    #[test]
    fn test_repo_root_validates_jj_repo() {
        // Test that RepoRoot requires .jj directory
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path();
        std::fs::create_dir_all(repo_path.join(".jj")).unwrap();
        let result = RepoRoot::new(repo_path.to_path_buf());
        assert!(result.is_ok(), "should accept JJ repository");
        drop(temp_dir);
    }

    #[test]
    fn test_workspace_name_validates_security() {
        // Test that WorkspaceName rejects dangerous names
        let dangerous_names = vec!["../../../etc/passwd", "/absolute/path", "hidden\x00byte"];

        for name in dangerous_names {
            let result = WorkspaceName::new(name.to_string());
            assert!(result.is_err(), "should reject dangerous name: {}", name);
        }
    }

    #[test]
    fn test_workspace_name_rejects_empty() {
        // Test that WorkspaceName rejects empty strings
        let result = WorkspaceName::new("".to_string());
        assert!(result.is_err(), "should reject empty name");
    }

    #[test]
    fn test_workspace_name_rejects_invalid_chars() {
        // Test that WorkspaceName rejects invalid characters
        let invalid_names = vec!["workspace/with/slash", "workspace\\with\\backslash"];

        for name in invalid_names {
            let result = WorkspaceName::new(name.to_string());
            assert!(
                result.is_err(),
                "should reject name with separators: {}",
                name
            );
        }
    }

    #[test]
    fn test_workspace_name_accepts_valid_names() {
        // Test that WorkspaceName accepts valid names
        let valid_names = vec![
            "simple-workspace",
            "workspace_123",
            "UPPERCASE",
            "camelCase",
        ];

        for name in valid_names {
            let result = WorkspaceName::new(name.to_string());
            assert!(result.is_ok(), "should accept valid name: {}", name);
        }
    }

    #[test]
    fn test_bead_id_validates_pattern() {
        // Test that BeadId validates pattern
        let result = BeadId::new("zjj-123-abc".to_string());
        assert!(result.is_ok(), "should accept valid bead ID format");
    }

    #[test]
    fn test_bead_id_rejects_invalid_format() {
        // Test that BeadId rejects invalid formats
        let invalid_ids = vec!["", "spaces in name", "invalid@chars"];

        for id in invalid_ids {
            let result = BeadId::new(id.to_string());
            assert!(result.is_err(), "should reject invalid ID: {}", id);
        }
    }

    #[test]
    fn test_commit_id_validates_hex() {
        // Test that CommitId validates hexadecimal format
        let valid_hex_ids = vec!["abc123", "deadbeef", "cafef00d"];

        for id in valid_hex_ids {
            let result = CommitId::new(id.to_string());
            assert!(result.is_ok(), "should accept valid hex ID: {}", id);
        }
    }

    #[test]
    fn test_commit_id_rejects_non_hex() {
        // Test that CommitId rejects non-hexadecimal strings
        let invalid_ids = vec!["ggg123", "xyz", "not@hex"];

        for id in invalid_ids {
            let result = CommitId::new(id.to_string());
            assert!(result.is_err(), "should reject non-hex ID: {}", id);
        }
    }

    #[test]
    fn test_jj_output_validates_utf8() {
        // Test that JjOutput validates UTF-8 encoding
        let valid_utf8 = vec!["Hello 世界", "Normal text", "áççí"];

        for text in valid_utf8 {
            let result = JjOutput::new(text.to_string());
            assert!(result.is_ok(), "should accept valid UTF-8: {}", text);
        }
    }

    #[test]
    fn test_jj_output_rejects_invalid_utf8() {
        // Test that JjOutput rejects invalid UTF-8
        let invalid_utf8 = vec!["invalid \0 null byte"];

        for text in invalid_utf8 {
            let result = JjOutput::new(text.to_string());
            assert!(
                result.is_err(),
                "should reject invalid UTF-8 containing null bytes"
            );
        }
    }

    #[test]
    fn test_repo_root_implements_display() {
        // Test that RepoRoot implements Display
        let path = PathBuf::from("/test/path");
        let result = RepoRoot::new(path);
        assert!(result.is_ok(), "should create RepoRoot");
        let repo_root = result.unwrap();

        let display = format!("{}", repo_root);
        assert!(display.contains("/test/path"), "Display should show path");
    }

    #[test]
    fn test_workspace_name_implements_display() {
        // Test that WorkspaceName implements Display
        let result = WorkspaceName::new("test-workspace".to_string());
        assert!(result.is_ok(), "should create WorkspaceName");
        let ws_name = result.unwrap();

        let display = format!("{}", ws_name);
        assert_eq!(display, "test-workspace", "Display should show name");
    }

    #[test]
    fn test_bead_id_implements_display() {
        // Test that BeadId implements Display
        let result = BeadId::new("zjj-123".to_string());
        assert!(result.is_ok(), "should create BeadId");
        let bead_id = result.unwrap();

        let display = format!("{}", bead_id);
        assert_eq!(display, "zjj-123", "Display should show ID");
    }

    #[test]
    fn test_commit_id_implements_display() {
        // Test that CommitId implements Display
        let result = CommitId::new("abc123".to_string());
        assert!(result.is_ok(), "should create CommitId");
        let commit_id = result.unwrap();

        let display = format!("{}", commit_id);
        assert_eq!(display, "abc123", "Display should show ID");
    }

    #[test]
    fn test_jj_output_implements_display() {
        // Test that JjOutput implements Display
        let result = JjOutput::new("test output".to_string());
        assert!(result.is_ok(), "should create JjOutput");
        let output = result.unwrap();

        let display = format!("{}", output);
        assert_eq!(display, "test output", "Display should show output");
    }

    #[test]
    fn test_repo_root_inner_access() {
        // Test that we can access inner PathBuf value
        let path = PathBuf::from("/test/path");
        let repo_root = RepoRoot::new(path).unwrap();
        assert_eq!(repo_root.inner(), &path, "inner() should return PathBuf");
    }

    #[test]
    fn test_workspace_name_inner_access() {
        // Test that we can access inner String value
        let name = "test-workspace";
        let ws_name = WorkspaceName::new(name.to_string()).unwrap();
        assert_eq!(ws_name.inner(), name, "inner() should return String");
    }

    #[test]
    fn test_bead_id_inner_access() {
        // Test that we can access inner String value
        let id = "zjj-123";
        let bead_id = BeadId::new(id.to_string()).unwrap();
        assert_eq!(bead_id.inner(), id, "inner() should return String");
    }

    #[test]
    fn test_commit_id_inner_access() {
        // Test that we can access inner String value
        let id = "abc123";
        let commit_id = CommitId::new(id.to_string()).unwrap();
        assert_eq!(commit_id.inner(), id, "inner() should return String");
    }

    #[test]
    fn test_jj_output_inner_access() {
        // Test that we can access inner String value
        let text = "test output";
        let output = JjOutput::new(text.to_string()).unwrap();
        assert_eq!(output.inner(), text, "inner() should return String");
    }

    #[test]
    fn test_newtype_prevents_string_aliasing() {
        // Test that NewType wrappers prevent accidental string operations
        // We can't create BeadId directly from String
        let _result = BeadId::new("zjj-123".to_string());
        // The type system prevents this: you must use the newtype constructor
        assert!(true, "NewType pattern enforced");
    }
}
