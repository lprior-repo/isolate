//! Bookmark/branch management commands
//!
//! This module provides a wrapper around JJ's bookmark commands to maintain
//! zjj's single-interface principle - AI agents use 'zjj bookmark' not 'jj bookmark'.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]

use anyhow::{Context, Result};
use serde::Serialize;
use thiserror::Error;
use tokio::process::Command;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::commands::get_session_db;

/// Bookmark-specific errors
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BookmarkError {
    #[error("bookmark '{0}' already exists (use move instead)")]
    AlreadyExists(String),

    #[error("bookmark '{0}' not found")]
    NotFound(String),

    #[error("invalid bookmark name '{0}': {1}")]
    InvalidName(String, String),

    #[error("session '{0}' not found")]
    SessionNotFound(String),

    #[error("workspace path does not exist: {0}")]
    WorkspaceNotFound(String),

    #[error("jj command failed: {0}")]
    JjCommandFailed(String),
}

fn validate_bookmark_name(name: &str) -> Result<()> {
    let is_valid = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if is_valid {
        Ok(())
    } else {
        Err(BookmarkError::InvalidName(
            name.to_string(),
            "must be alphanumeric, underscore, or hyphen".to_string(),
        )
        .into())
    }
}

/// Bookmark information
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BookmarkInfo {
    pub name: String,
    pub revision: String,
    pub remote: bool,
}

/// Options for bookmark list operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListOptions {
    pub session: Option<String>,
    pub show_all: bool,
    pub format: OutputFormat,
}

/// Options for bookmark create operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateOptions {
    pub name: String,
    pub session: Option<String>,
    pub push: bool,
    pub format: OutputFormat,
}

/// Options for bookmark delete operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteOptions {
    pub name: String,
    pub session: Option<String>,
    pub format: OutputFormat,
}

/// Options for bookmark move operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveOptions {
    pub name: String,
    pub to_revision: String,
    pub session: Option<String>,
    pub format: OutputFormat,
}

/// List bookmarks in a session workspace
///
/// # Errors
///
/// Returns an error if:
/// - Session not found
/// - Workspace path doesn't exist
/// - JJ command fails
pub async fn list(options: &ListOptions) -> Result<Vec<BookmarkInfo>> {
    let workspace_path = resolve_workspace_path(options.session.as_deref()).await?;

    // Build JJ command
    let mut cmd = Command::new("jj");
    cmd.args(["bookmark", "list"]).current_dir(&workspace_path);

    if options.show_all {
        cmd.arg("--all");
    }

    // Execute command
    let output = cmd
        .output()
        .await
        .context("failed to execute jj bookmark list")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BookmarkError::JjCommandFailed(stderr.to_string()).into());
    }

    // Parse output
    parse_bookmark_list(&output.stdout)
}

/// Create a new bookmark at current revision
///
/// # Errors
///
/// Returns an error if:
/// - Bookmark already exists
/// - Session not found
/// - Workspace path doesn't exist
/// - JJ command fails
pub async fn create(options: &CreateOptions) -> Result<BookmarkInfo> {
    validate_bookmark_name(&options.name)?;
    let workspace_path = resolve_workspace_path(options.session.as_deref()).await?;

    // Check if bookmark already exists
    let existing = list(&ListOptions {
        session: options.session.clone(),
        show_all: false,
        format: options.format,
    })
    .await?;

    if existing.iter().any(|b| b.name == options.name) {
        return Err(BookmarkError::AlreadyExists(options.name.clone()).into());
    }

    // Build JJ command
    let mut cmd = Command::new("jj");
    cmd.args(["bookmark", "create", &options.name])
        .current_dir(&workspace_path);

    // Execute command
    let output = cmd
        .output()
        .await
        .context("failed to execute jj bookmark create")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BookmarkError::JjCommandFailed(stderr.to_string()).into());
    }

    // Get current revision
    let revision = get_current_revision(&workspace_path).await?;

    // Push to remote if requested
    if options.push {
        push_bookmark(&options.name, &workspace_path).await?;
    }

    Ok(BookmarkInfo {
        name: options.name.clone(),
        revision,
        remote: options.push,
    })
}

/// Delete a bookmark
///
/// # Errors
///
/// Returns an error if:
/// - Bookmark doesn't exist (exit code 3)
/// - Session not found
/// - Workspace path doesn't exist
/// - JJ command fails
pub async fn delete(options: &DeleteOptions) -> Result<()> {
    validate_bookmark_name(&options.name)?;
    let workspace_path = resolve_workspace_path(options.session.as_deref()).await?;

    // Check if bookmark exists
    let existing = list(&ListOptions {
        session: options.session.clone(),
        show_all: false,
        format: options.format,
    })
    .await?;

    if !existing.iter().any(|b| b.name == options.name) {
        return Err(BookmarkError::NotFound(options.name.clone()).into());
    }

    // Build JJ command
    let mut cmd = Command::new("jj");
    cmd.args(["bookmark", "delete", &options.name])
        .current_dir(&workspace_path);

    // Execute command
    let output = cmd
        .output()
        .await
        .context("failed to execute jj bookmark delete")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BookmarkError::JjCommandFailed(stderr.to_string()).into());
    }

    Ok(())
}

/// Move a bookmark to a different revision
///
/// # Errors
///
/// Returns an error if:
/// - Bookmark doesn't exist
/// - Target revision doesn't exist
/// - Session not found
/// - Workspace path doesn't exist
/// - JJ command fails
pub async fn move_bookmark(options: &MoveOptions) -> Result<BookmarkInfo> {
    validate_bookmark_name(&options.name)?;
    let workspace_path = resolve_workspace_path(options.session.as_deref()).await?;

    // Build JJ command
    let mut cmd = Command::new("jj");
    cmd.args([
        "bookmark",
        "move",
        &options.name,
        "--to",
        &options.to_revision,
    ])
    .current_dir(&workspace_path);

    // Execute command
    let output = cmd
        .output()
        .await
        .context("failed to execute jj bookmark move")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BookmarkError::JjCommandFailed(stderr.to_string()).into());
    }

    Ok(BookmarkInfo {
        name: options.name.clone(),
        revision: options.to_revision.clone(),
        remote: false,
    })
}

/// Run the bookmark list command
pub async fn run_list(options: &ListOptions) -> Result<()> {
    let bookmarks = list(options).await?;

    if options.format.is_json() {
        // Use SchemaEnvelopeArray for Vec data because serde flatten cannot serialize sequences
        let envelope = zjj_core::json::SchemaEnvelopeArray::new(
            "bookmark-list-response",
            bookmarks,
        );
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else if bookmarks.is_empty() {
        println!("No bookmarks found.");
    } else {
        println!("Bookmarks:");
        for bookmark in &bookmarks {
            let remote_marker = if bookmark.remote { " (remote)" } else { "" };
            println!(
                "  {} → {}{}",
                bookmark.name, bookmark.revision, remote_marker
            );
        }
    }

    Ok(())
}

/// Run the bookmark create command
pub async fn run_create(options: &CreateOptions) -> Result<()> {
    let bookmark = create(options).await?;

    if options.format.is_json() {
        #[derive(Serialize)]
        struct CreateResponse {
            success: bool,
            bookmark: String,
            revision: String,
        }

        let response = CreateResponse {
            success: true,
            bookmark: bookmark.name,
            revision: bookmark.revision,
        };

        let envelope = SchemaEnvelope::new("bookmark-create-response", "single", response);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!(
            "Created bookmark '{}' at revision {}",
            options.name, bookmark.revision
        );
        if options.push {
            println!("Pushed to remote");
        }
    }

    Ok(())
}

/// Run the bookmark delete command
pub async fn run_delete(options: &DeleteOptions) -> Result<()> {
    delete(options).await?;

    if options.format.is_json() {
        #[derive(Serialize)]
        struct DeleteResponse {
            success: bool,
            deleted: String,
        }

        let response = DeleteResponse {
            success: true,
            deleted: options.name.clone(),
        };

        let envelope = SchemaEnvelope::new("bookmark-delete-response", "single", response);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("Deleted bookmark '{}'", options.name);
    }

    Ok(())
}

/// Run the bookmark move command
pub async fn run_move(options: &MoveOptions) -> Result<()> {
    let bookmark = move_bookmark(options).await?;

    if options.format.is_json() {
        #[derive(Serialize)]
        struct MoveResponse {
            success: bool,
            bookmark: String,
            new_revision: String,
        }

        let response = MoveResponse {
            success: true,
            bookmark: bookmark.name,
            new_revision: bookmark.revision,
        };

        let envelope = SchemaEnvelope::new("bookmark-move-response", "single", response);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!(
            "Moved bookmark '{}' to revision {}",
            options.name, bookmark.revision
        );
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS (Pure, Private)
// ═══════════════════════════════════════════════════════════════════════════

/// Resolve workspace path from session name or current directory
async fn resolve_workspace_path(session: Option<&str>) -> Result<String> {
    let path = if let Some(name) = session {
        let db = get_session_db().await?;
        let sessions = db.list(None).await?;

        sessions
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| BookmarkError::SessionNotFound(name.to_string()))?
            .workspace_path
            .clone()
    } else {
        std::env::current_dir()
            .context("failed to get current directory")?
            .to_string_lossy()
            .to_string()
    };

    if tokio::fs::try_exists(&path).await.unwrap_or(false) {
        Ok(path)
    } else {
        Err(BookmarkError::WorkspaceNotFound(path).into())
    }
}

/// Parse JJ bookmark list output
///
/// Handles multi-line format:
/// - Main bookmarks: "name: `change_id` `commit_id` description"
/// - Remote bookmarks (indented): "  @remote: `change_id` `commit_id` description"
/// - Deleted bookmarks: "name: ... (deleted)" on following line
/// - Legacy format: "name: `revision_hash`"
///
/// Only returns non-deleted local bookmarks (skips indented remote lines).
fn parse_bookmark_list(output: &[u8]) -> Result<Vec<BookmarkInfo>> {
    let stdout = String::from_utf8_lossy(output);

    // Use functional parsing with zero unwrap
    let bookmarks: std::result::Result<Vec<BookmarkInfo>, anyhow::Error> = stdout
        .lines()
        .filter(|line| {
            // Skip empty lines
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return false;
            }

            // Skip indented remote bookmark lines (start with "  @")
            if line.starts_with("  @") {
                return false;
            }

            // Filter out lines marked as deleted
            !trimmed.contains("(deleted)")
        })
        .map(|line| {
            // Parse format: "bookmark_name: change_id commit_id description"
            // or legacy: "bookmark_name: revision_hash"
            let parts: Vec<&str> = line.splitn(2, ':').collect();

            match parts.as_slice() {
                [name_part, rest] => {
                    let name = name_part.trim().to_string();

                    // Extract revision (handle both formats)
                    // New format: "change_id commit_id description" -> get commit_id
                    // Legacy format: "revision_hash" -> get first token
                    let tokens: Vec<&str> = rest.split_whitespace().collect();
                    let revision = if tokens.len() >= 2 {
                        // New format: skip change_id, get commit_id
                        tokens
                            .get(1)
                            .map_or_else(|| "unknown".to_string(), ToString::to_string)
                    } else {
                        // Legacy format: use first token
                        tokens
                            .first()
                            .map_or_else(|| "unknown".to_string(), ToString::to_string)
                    };

                    let remote = rest.contains("@origin");

                    Ok(BookmarkInfo {
                        name,
                        revision,
                        remote,
                    })
                }
                _ => Err(anyhow::anyhow!("invalid bookmark list output: {line}")),
            }
        })
        .collect();

    bookmarks
}

/// Get current revision hash
async fn get_current_revision(workspace_path: &str) -> Result<String> {
    let output = Command::new("jj")
        .args(["log", "-r", "@", "--no-graph", "-T", "commit_id"])
        .current_dir(workspace_path)
        .output()
        .await
        .context("failed to get current revision")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BookmarkError::JjCommandFailed(stderr.to_string()).into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Push bookmark to remote
async fn push_bookmark(bookmark_name: &str, workspace_path: &str) -> Result<()> {
    let output = Command::new("jj")
        .args(["git", "push", "--bookmark", bookmark_name])
        .current_dir(workspace_path)
        .output()
        .await
        .context("failed to push bookmark")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BookmarkError::JjCommandFailed(stderr.to_string()).into());
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bookmark_error_messages() {
        let err = BookmarkError::AlreadyExists("feature-v1".to_string());
        assert_eq!(
            err.to_string(),
            "bookmark 'feature-v1' already exists (use move instead)"
        );

        let err = BookmarkError::NotFound("feature-v1".to_string());
        assert_eq!(err.to_string(), "bookmark 'feature-v1' not found");
    }

    #[test]
    fn test_parse_bookmark_list_empty() {
        let output = b"";
        let result = parse_bookmark_list(output);
        assert!(result.is_ok());
        assert_eq!(result.ok(), Some(vec![]));
    }

    #[test]
    fn test_parse_bookmark_list_single() {
        let output = b"feature-v1: abc123def456\n";
        let result = parse_bookmark_list(output);
        assert!(result.is_ok());

        let bookmarks = result.ok();
        assert!(bookmarks.is_some());

        if let Some(bookmarks) = bookmarks {
            assert_eq!(bookmarks.len(), 1);
            assert_eq!(bookmarks[0].name, "feature-v1");
            assert_eq!(bookmarks[0].revision, "abc123def456");
            assert!(!bookmarks[0].remote);
        }
    }

    #[test]
    fn test_parse_bookmark_list_multiple() {
        let output = b"main: abc123\nfeature: def456\n";
        let result = parse_bookmark_list(output);
        assert!(result.is_ok());

        if let Ok(bookmarks) = result {
            assert_eq!(bookmarks.len(), 2);
            assert_eq!(bookmarks[0].name, "main");
            assert_eq!(bookmarks[1].name, "feature");
        }
    }

    #[test]
    fn test_parse_bookmark_list_multiline_with_remotes() {
        // Real JJ output format with indented remote bookmarks
        let output = b"main: ntzomurw e553bf6b feat: Implement handle_broadcast function\n
  @git: ntzomurw e553bf6b feat: Implement handle_broadcast function\n
  @origin: ntzomurw e553bf6b feat: Implement handle_broadcast function\n";

        let result = parse_bookmark_list(output);
        assert!(result.is_ok(), "Parsing should succeed: {:?}", result.err());

        if let Ok(bookmarks) = result {
            // Should only return main bookmark, skip indented remote lines
            assert_eq!(bookmarks.len(), 1);
            assert_eq!(bookmarks[0].name, "main");
            assert_eq!(bookmarks[0].revision, "e553bf6b");
            assert!(!bookmarks[0].remote);
        }
    }

    #[test]
    fn test_parse_bookmark_list_with_deleted() {
        // Bookmarks marked as (deleted) should be filtered out
        let output = b"main: abc123def456\n
old-feature: xyz789 (deleted)\n";

        let result = parse_bookmark_list(output);
        assert!(result.is_ok(), "Parsing should succeed: {:?}", result.err());

        if let Ok(bookmarks) = result {
            // Should only return non-deleted bookmarks
            assert_eq!(bookmarks.len(), 1);
            assert_eq!(bookmarks[0].name, "main");
        }
    }

    #[test]
    fn test_parse_bookmark_list_mixed_format() {
        // Complex mixed format with multiple bookmarks, remotes, and deleted
        let output = b"main: ntzomurw e553bf6b feat: Broadcast command\n
  @origin: ntzomurw e553bf6b feat: Broadcast command\n
feature: pqrlsyvw 195a784b test: Another feature\n
deprecated: vwxyz123 (deleted)\n
  @git: vwxyz123 deprecated bookmark\n
bugfix: kmnopqr6 2d4e5f6c fix: Critical bug\n";

        let result = parse_bookmark_list(output);
        assert!(result.is_ok(), "Parsing should succeed: {:?}", result.err());

        if let Ok(bookmarks) = result {
            // Should return main, feature, and bugfix (skip deleted and remotes)
            assert_eq!(bookmarks.len(), 3);
            assert_eq!(bookmarks[0].name, "main");
            assert_eq!(bookmarks[1].name, "feature");
            assert_eq!(bookmarks[2].name, "bugfix");

            // Verify revisions extracted correctly
            assert_eq!(bookmarks[0].revision, "e553bf6b");
            assert_eq!(bookmarks[1].revision, "195a784b");
            assert_eq!(bookmarks[2].revision, "2d4e5f6c");
        }
    }

    #[test]
    fn test_list_options_construction() {
        let opts = ListOptions {
            session: Some("test-session".to_string()),
            show_all: true,
            format: OutputFormat::Json,
        };

        assert_eq!(opts.session, Some("test-session".to_string()));
        assert!(opts.show_all);
        assert_eq!(opts.format, OutputFormat::Json);
    }

    #[test]
    fn test_create_options_construction() {
        let opts = CreateOptions {
            name: "feature-v1".to_string(),
            session: None,
            push: false,
            format: OutputFormat::Human,
        };

        assert_eq!(opts.name, "feature-v1");
        assert_eq!(opts.session, None);
        assert!(!opts.push);
        assert_eq!(opts.format, OutputFormat::Human);
    }

    #[test]
    fn test_delete_options_construction() {
        let opts = DeleteOptions {
            name: "old-feature".to_string(),
            session: Some("session1".to_string()),
            format: OutputFormat::Json,
        };

        assert_eq!(opts.name, "old-feature");
        assert_eq!(opts.session, Some("session1".to_string()));
        assert_eq!(opts.format, OutputFormat::Json);
    }

    #[test]
    fn test_move_options_construction() {
        let opts = MoveOptions {
            name: "feature".to_string(),
            to_revision: "abc123".to_string(),
            session: None,
            format: OutputFormat::Human,
        };

        assert_eq!(opts.name, "feature");
        assert_eq!(opts.to_revision, "abc123");
        assert_eq!(opts.session, None);
        assert_eq!(opts.format, OutputFormat::Human);
    }

    #[test]
    fn test_bookmark_info_serialization() {
        let info = BookmarkInfo {
            name: "main".to_string(),
            revision: "abc123".to_string(),
            remote: true,
        };

        let json = serde_json::to_string(&info);
        assert!(json.is_ok());

        if let Ok(json_str) = json {
            assert!(json_str.contains("\"name\":\"main\""));
            assert!(json_str.contains("\"revision\":\"abc123\""));
            assert!(json_str.contains("\"remote\":true"));
        }
    }
}
