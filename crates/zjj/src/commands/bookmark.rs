//! Bookmark/branch management commands
//!
//! This module provides a wrapper around JJ's bookmark commands to maintain
//! zjj's single-interface principle - AI agents use 'zjj bookmark' not 'jj bookmark'.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

use std::{path::Path, process::Command};

use anyhow::{Context, Result};
use serde::Serialize;
use thiserror::Error;
use zjj_core::{json::SchemaEnvelope, OutputFormat};

use crate::{commands::get_session_db, json};

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
pub fn list(options: &ListOptions) -> Result<Vec<BookmarkInfo>> {
    let workspace_path = resolve_workspace_path(options.session.as_deref())?;

    // Build JJ command
    let mut cmd = Command::new("jj");
    cmd.args(["bookmark", "list"]).current_dir(&workspace_path);

    if options.show_all {
        cmd.arg("--all");
    }

    // Execute command
    let output = cmd.output().context("failed to execute jj bookmark list")?;

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
pub fn create(options: &CreateOptions) -> Result<BookmarkInfo> {
    let workspace_path = resolve_workspace_path(options.session.as_deref())?;

    // Check if bookmark already exists
    let existing = list(&ListOptions {
        session: options.session.clone(),
        show_all: false,
        format: options.format,
    })?;

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
        .context("failed to execute jj bookmark create")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BookmarkError::JjCommandFailed(stderr.to_string()).into());
    }

    // Get current revision
    let revision = get_current_revision(&workspace_path)?;

    // Push to remote if requested
    if options.push {
        push_bookmark(&options.name, &workspace_path)?;
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
pub fn delete(options: &DeleteOptions) -> Result<()> {
    let workspace_path = resolve_workspace_path(options.session.as_deref())?;

    // Check if bookmark exists
    let existing = list(&ListOptions {
        session: options.session.clone(),
        show_all: false,
        format: options.format,
    })?;

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
pub fn move_bookmark(options: &MoveOptions) -> Result<BookmarkInfo> {
    let workspace_path = resolve_workspace_path(options.session.as_deref())?;

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
    let output = cmd.output().context("failed to execute jj bookmark move")?;

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
pub fn run_list(options: &ListOptions) -> Result<()> {
    let result = (|| -> Result<()> {
        let bookmarks = list(&options)?;

        if options.format.is_json() {
            let envelope = SchemaEnvelope::new("bookmark-list-response", "array", bookmarks);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else if bookmarks.is_empty() {
            println!("No bookmarks found.");
        } else {
            println!("Bookmarks:");
            for bookmark in bookmarks {
                let remote_marker = if bookmark.remote { " (remote)" } else { "" };
                println!(
                    "  {} → {}{}",
                    bookmark.name, bookmark.revision, remote_marker
                );
            }
        }

        Ok(())
    })();

    if let Err(e) = result {
        if options.format.is_json() {
            json::output_json_error_and_exit(&e);
        } else {
            return Err(e);
        }
    }

    Ok(())
}

/// Run the bookmark create command
pub fn run_create(options: &CreateOptions) -> Result<()> {
    let result = (|| -> Result<()> {
        let bookmark = create(&options)?;

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
    })();

    if let Err(e) = result {
        if options.format.is_json() {
            json::output_json_error_and_exit(&e);
        } else {
            return Err(e);
        }
    }

    Ok(())
}

/// Run the bookmark delete command
pub fn run_delete(options: &DeleteOptions) -> Result<()> {
    let result = (|| -> Result<()> {
        delete(&options)?;

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
    })();

    if let Err(e) = result {
        if options.format.is_json() {
            json::output_json_error_and_exit(&e);
        } else {
            // Exit with code 3 for "not found" errors
            if let Some(err) = e.downcast_ref::<BookmarkError>() {
                if matches!(err, BookmarkError::NotFound(_)) {
                    std::process::exit(3);
                }
            }
            return Err(e);
        }
    }

    Ok(())
}

/// Run the bookmark move command
pub fn run_move(options: &MoveOptions) -> Result<()> {
    let result = (|| -> Result<()> {
        let bookmark = move_bookmark(&options)?;

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
    })();

    if let Err(e) = result {
        if options.format.is_json() {
            json::output_json_error_and_exit(&e);
        } else {
            return Err(e);
        }
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS (Pure, Private)
// ═══════════════════════════════════════════════════════════════════════════

/// Resolve workspace path from session name or current directory
fn resolve_workspace_path(session: Option<&str>) -> Result<String> {
    let path = if let Some(name) = session {
        let db = get_session_db()?;
        let sessions = db.list_blocking(None)?;

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

    if Path::new(&path).exists() {
        Ok(path)
    } else {
        Err(BookmarkError::WorkspaceNotFound(path).into())
    }
}

/// Parse JJ bookmark list output
fn parse_bookmark_list(output: &[u8]) -> Result<Vec<BookmarkInfo>> {
    let stdout = String::from_utf8_lossy(output);

    stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            // Parse format: "bookmark_name: revision_hash"
            let parts: Vec<&str> = line.splitn(2, ':').collect();

            if parts.len() == 2 {
                let name = parts[0].trim().to_string();
                let revision = parts[1]
                    .split_whitespace()
                    .next()
                    .unwrap_or("unknown")
                    .to_string();
                let remote = line.contains("@origin") || line.contains("(remote)");

                Ok(BookmarkInfo {
                    name,
                    revision,
                    remote,
                })
            } else {
                Err(anyhow::anyhow!("invalid bookmark list output: {line}"))
            }
        })
        .collect()
}

/// Get current revision hash
fn get_current_revision(workspace_path: &str) -> Result<String> {
    let output = Command::new("jj")
        .args(["log", "-r", "@", "--no-graph", "-T", "commit_id"])
        .current_dir(workspace_path)
        .output()
        .context("failed to get current revision")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BookmarkError::JjCommandFailed(stderr.to_string()).into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Push bookmark to remote
fn push_bookmark(bookmark_name: &str, workspace_path: &str) -> Result<()> {
    let output = Command::new("jj")
        .args(["git", "push", "--bookmark", bookmark_name])
        .current_dir(workspace_path)
        .output()
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

        let bookmarks = bookmarks;
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
