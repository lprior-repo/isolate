//! Execution error types for database and repository state issues.
//!
//! These errors represent problems with the current execution state, such as
//! invalid database conditions or missing repository state.

use std::fmt;

/// Execution errors represent problems with database or repository state.
#[derive(Debug, Clone)]
pub enum ExecutionError {
    /// Database operation failed
    DatabaseError(String),
    /// Repository has no commits yet
    NoCommitsYet { workspace_path: String },
    /// Main bookmark is missing from repository
    MainBookmarkMissing {
        workspace_path: String,
        bookmark_name: String,
        commit_count: usize,
    },
    /// Resource not found
    NotFound(String),
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DatabaseError(msg) => write!(f, "Database error: {msg}"),
            Self::NoCommitsYet { workspace_path } => {
                write!(
                    f,
                    "Cannot sync: No commits in repository yet\n\n\
                    The main bookmark/branch doesn't exist because no commits have been made.\n\n\
                    To fix this:\n\
                      1. Create an initial commit:\n\
                         jj --repository {workspace_path} commit -m \"Initial commit\"\n\
                      2. Create main bookmark:\n\
                         jj --repository {workspace_path} bookmark create main\n\
                      3. Then retry: zjj sync\n\n\
                    Current repository state:\n\
                      - JJ repo: initialized\n\
                      - Commits: 0\n\
                      - Main bookmark: missing"
                )
            }
            Self::MainBookmarkMissing {
                workspace_path,
                bookmark_name,
                commit_count,
            } => {
                write!(
                    f,
                    "Cannot sync: Main bookmark '{bookmark_name}' doesn't exist\n\n\
                    The repository has {commit_count} commit(s), but the '{bookmark_name}' bookmark is missing.\n\n\
                    To fix this:\n\
                      1. Create the '{bookmark_name}' bookmark on your desired commit:\n\
                         jj --repository {workspace_path} bookmark create {bookmark_name}\n\
                      2. Or set it to an existing revision:\n\
                         jj --repository {workspace_path} bookmark create {bookmark_name} -r <revision>\n\
                      3. Then retry: zjj sync\n\n\
                    Alternatively, configure a different main branch in .jjz/config.toml:\n\
                      main_branch = \"trunk()\"  # or another revset/bookmark\n\n\
                    Current repository state:\n\
                      - JJ repo: initialized\n\
                      - Commits: {commit_count}\n\
                      - '{bookmark_name}' bookmark: missing"
                )
            }
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
        }
    }
}

impl ExecutionError {
    /// Get exit code for execution errors.
    /// - Not found: 3
    /// - Database/state errors: 4
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::NotFound(_) => 3,
            _ => 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_error_display() {
        let err = ExecutionError::DatabaseError("connection failed".to_string());
        assert_eq!(err.to_string(), "Database error: connection failed");
    }

    #[test]
    fn test_not_found_display() {
        let err = ExecutionError::NotFound("session".to_string());
        assert_eq!(err.to_string(), "Not found: session");
    }

    #[test]
    fn test_no_commits_yet_display() {
        let err = ExecutionError::NoCommitsYet {
            workspace_path: "/tmp/repo".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("Cannot sync"));
        assert!(display.contains("No commits"));
        assert!(display.contains("/tmp/repo"));
    }

    #[test]
    fn test_main_bookmark_missing_display() {
        let err = ExecutionError::MainBookmarkMissing {
            workspace_path: "/tmp/repo".to_string(),
            bookmark_name: "main".to_string(),
            commit_count: 5,
        };
        let display = err.to_string();
        assert!(display.contains("Cannot sync"));
        assert!(display.contains("bookmark 'main' doesn't exist"));
        assert!(display.contains("5 commit"));
        assert!(display.contains("/tmp/repo"));
    }

    #[test]
    fn test_exit_code_not_found() {
        assert_eq!(
            ExecutionError::NotFound("session".to_string()).exit_code(),
            3
        );
    }

    #[test]
    fn test_exit_code_invalid_state() {
        assert_eq!(
            ExecutionError::DatabaseError("corrupt".to_string()).exit_code(),
            4
        );
        assert_eq!(
            ExecutionError::NoCommitsYet {
                workspace_path: "/tmp".to_string()
            }
            .exit_code(),
            4
        );
        assert_eq!(
            ExecutionError::MainBookmarkMissing {
                workspace_path: "/tmp".to_string(),
                bookmark_name: "main".to_string(),
                commit_count: 0
            }
            .exit_code(),
            4
        );
    }
}
