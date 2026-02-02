//! Periodic orphan workspace cleanup
//!
//! Runs background task to detect and clean orphaned workspaces that are:
//! - Older than 2 hours (configurable threshold)
//! - Have no active bead association
//! - Have missing workspace directories
//!
//! Uses Railway-Oriented Programming for error handling and functional
//! patterns for all business logic.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use std::{path::Path, time::Duration};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::time::sleep;
use zjj_core::{OutputFormat, Session};

use crate::commands::get_session_db;

// ═══════════════════════════════════════════════════════════════════════════
// DOMAIN TYPES (Functional Core)
// ═══════════════════════════════════════════════════════════════════════════

/// Configuration for periodic cleanup task
#[derive(Debug, Clone)]
pub struct PeriodicCleanupConfig {
    /// Interval between cleanup runs (default: 1 hour)
    pub interval: Duration,
    /// Age threshold for orphaned workspaces (default: 2 hours)
    pub age_threshold: Duration,
    /// Whether to run in dry-run mode (don't actually remove)
    pub dry_run: bool,
    /// Output format for logging
    pub format: OutputFormat,
}

impl Default for PeriodicCleanupConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(3600), // 1 hour
            age_threshold: Duration::from_secs(7200), // 2 hours
            dry_run: false,
            format: OutputFormat::Human,
        }
    }
}

/// Represents a workspace candidate for cleanup
#[derive(Debug, Clone, PartialEq, Eq)]
struct OrphanCandidate {
    /// Session name
    name: String,
    /// Workspace path
    workspace_path: String,
    /// Age in seconds
    age_seconds: i64,
    /// Whether associated with active bead
    has_active_bead: bool,
    /// Whether workspace directory exists
    workspace_exists: bool,
}

/// Result of a periodic cleanup run
#[derive(Debug, Clone, Serialize)]
pub struct PeriodicCleanupOutput {
    /// Timestamp of cleanup run
    pub timestamp: String,
    /// Number of orphans detected
    pub orphans_detected: usize,
    /// Number of orphans cleaned
    pub orphans_cleaned: usize,
    /// List of cleaned session names
    pub cleaned_sessions: Vec<String>,
    /// List of skipped sessions (with reasons)
    pub skipped_sessions: Vec<SkippedSession>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkippedSession {
    pub name: String,
    pub reason: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// FUNCTIONAL CORE (Pure Logic)
// ═══════════════════════════════════════════════════════════════════════════

/// Determine if a session is an orphan candidate
///
/// A session is an orphan if:
/// 1. Workspace directory doesn't exist, OR
/// 2. Session is older than threshold AND has no active bead
fn is_orphan_candidate(
    session: &Session,
    age_threshold: &Duration,
    now: &DateTime<Utc>,
) -> Option<OrphanCandidate> {
    let age = calculate_age(session, now)?;
    let workspace_exists = Path::new(&session.workspace_path).exists();
    let has_active_bead = check_active_bead(session);

    // Orphan if workspace missing
    if !workspace_exists {
        return Some(OrphanCandidate {
            name: session.name.clone(),
            workspace_path: session.workspace_path.to_string_lossy().to_string(),
            age_seconds: age.num_seconds(),
            has_active_bead,
            workspace_exists: false,
        });
    }

    // Orphan if old enough AND no active bead
    let is_old_enough = age > chrono::Duration::from_std(*age_threshold).ok()?;
    if is_old_enough && !has_active_bead {
        Some(OrphanCandidate {
            name: session.name.clone(),
            workspace_path: session.workspace_path.to_string_lossy().to_string(),
            age_seconds: age.num_seconds(),
            has_active_bead: false,
            workspace_exists: true,
        })
    } else {
        None
    }
}

/// Calculate session age
fn calculate_age(
    session: &Session,
    now: &DateTime<Utc>,
) -> Option<chrono::Duration> {
    now.signed_duration_since(session.updated_at).into()
}

/// Check if session has an active bead
///
/// Looks in metadata for bead_id and checks if bead status is active
fn check_active_bead(session: &Session) -> bool {
    session
        .metadata
        .get("bead_id")
        .and_then(|v| v.as_str())
        .map_or(false, |bead_id| is_bead_active(bead_id))
}

/// Check if a bead is in active status
///
/// Uses Railway pattern - if any step fails, returns false
fn is_bead_active(bead_id: &str) -> bool {
    check_bead_status(bead_id)
        .map_or(false, |status| matches!(status.as_str(), "in_progress" | "open"))
}

/// Query bead status from beads system
///
/// Returns None if bead doesn't exist or can't be queried
fn check_bead_status(bead_id: &str) -> Option<String> {
    // Functional pipeline: try to execute br command and parse output
    std::process::Command::new("br")
        .args(["show", bead_id, "--json"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|json_str| serde_json::from_str::<serde_json::Value>(&json_str).ok())
        .and_then(|json| {
            json.as_array()
                .and_then(|arr| arr.first())
                .and_then(|obj| obj.get("status"))
                .and_then(|s| s.as_str())
                .map(ToString::to_string)
        })
}

/// Filter sessions to orphan candidates using functional pipeline
fn find_orphan_candidates(
    sessions: &[Session],
    age_threshold: &Duration,
    now: &DateTime<Utc>,
) -> Vec<OrphanCandidate> {
    sessions
        .iter()
        .filter_map(|session| is_orphan_candidate(session, age_threshold, now))
        .collect()
}

/// Categorize orphans into cleanable and skippable
fn categorize_orphans(
    orphans: Vec<OrphanCandidate>,
) -> (Vec<OrphanCandidate>, Vec<SkippedSession>) {
    orphans.into_iter().fold(
        (Vec::new(), Vec::new()),
        |(mut cleanable, mut skipped), orphan| {
            if should_clean(&orphan) {
                cleanable.push(orphan);
            } else {
                skipped.push(SkippedSession {
                    name: orphan.name.clone(),
                    reason: skip_reason(&orphan),
                });
            }
            (cleanable, skipped)
        },
    )
}

/// Determine if orphan should be cleaned
fn should_clean(orphan: &OrphanCandidate) -> bool {
    // Clean if workspace missing OR (old AND no active bead)
    !orphan.workspace_exists || (!orphan.has_active_bead && orphan.age_seconds >= 7200)
}

/// Generate skip reason for orphan
fn skip_reason(orphan: &OrphanCandidate) -> String {
    if orphan.has_active_bead {
        format!("Has active bead (age: {}h)", orphan.age_seconds / 3600)
    } else {
        format!("Not old enough (age: {}h < 2h)", orphan.age_seconds / 3600)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// IMPERATIVE SHELL (Side Effects)
// ═══════════════════════════════════════════════════════════════════════════

/// Run a single cleanup iteration
///
/// Railway-Oriented workflow:
/// 1. Fetch all sessions from database
/// 2. Filter to orphan candidates
/// 3. Categorize into cleanable/skippable
/// 4. Clean eligible orphans
/// 5. Return results
async fn run_cleanup_iteration(config: &PeriodicCleanupConfig) -> Result<PeriodicCleanupOutput> {
    let now = Utc::now();

    // 1. Fetch sessions (imperative I/O)
    let db = get_session_db()?;
    let sessions = db.list_blocking(None).map_err(anyhow::Error::new)?;

    // 2. Find orphan candidates (pure functional)
    let orphan_candidates = find_orphan_candidates(&sessions, &config.age_threshold, &now);

    // 3. Categorize (pure functional)
    let (cleanable, skipped) = categorize_orphans(orphan_candidates);

    // 4. Clean orphans (imperative I/O)
    let cleaned_sessions = if config.dry_run {
        Vec::new()
    } else {
        clean_orphans(&db, &cleanable).await?
    };

    // 5. Build result
    Ok(PeriodicCleanupOutput {
        timestamp: now.to_rfc3339(),
        orphans_detected: cleanable.len() + skipped.len(),
        orphans_cleaned: cleaned_sessions.len(),
        cleaned_sessions,
        skipped_sessions: skipped,
    })
}

/// Clean orphaned sessions from database
///
/// Uses functional fold to accumulate successful removals
async fn clean_orphans(
    db: &zjj_core::session_state::SessionDatabase,
    orphans: &[OrphanCandidate],
) -> Result<Vec<String>> {
    // Functional fold over orphans, collecting successful removals
    orphans.iter().try_fold(Vec::new(), |mut cleaned, orphan| {
        db.delete_blocking(&orphan.name)
            .map_err(anyhow::Error::new)
            .map(|()| {
                cleaned.push(orphan.name.clone());
                cleaned
            })
    })
}

/// Log cleanup results
fn log_cleanup_results(output: &PeriodicCleanupOutput, format: OutputFormat) {
    if format.is_json() {
        if let Ok(json_str) = serde_json::to_string_pretty(output) {
            println!("{json_str}");
        }
    } else {
        println!(
            "[{}] Periodic cleanup: detected={}, cleaned={}, skipped={}",
            output.timestamp, output.orphans_detected, output.orphans_cleaned, output.skipped_sessions.len()
        );

        if !output.cleaned_sessions.is_empty() {
            println!("  Cleaned sessions:");
            for session in &output.cleaned_sessions {
                println!("    - {session}");
            }
        }

        if !output.skipped_sessions.is_empty() {
            println!("  Skipped sessions:");
            for skipped in &output.skipped_sessions {
                println!("    - {}: {}", skipped.name, skipped.reason);
            }
        }
    }
}

/// Run periodic cleanup task indefinitely
///
/// This is the main entry point for the background cleanup daemon
pub async fn run_periodic_cleanup(config: PeriodicCleanupConfig) -> Result<()> {
    loop {
        // Run cleanup iteration
        match run_cleanup_iteration(&config).await {
            Ok(output) => log_cleanup_results(&output, config.format),
            Err(e) => {
                if config.format.is_json() {
                    eprintln!(r#"{{"error": "Cleanup failed: {}"}}"#, e);
                } else {
                    eprintln!("Periodic cleanup error: {e}");
                }
            }
        }

        // Sleep until next iteration
        sleep(config.interval).await;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn mock_session(name: &str, age_hours: i64, workspace_exists: bool) -> Session {
        let now = Utc::now();
        let created_at = now - chrono::Duration::hours(age_hours);

        Session {
            id: name.to_string(),
            name: name.to_string(),
            status: zjj_core::types::SessionStatus::Active,
            workspace_path: if workspace_exists {
                PathBuf::from("/tmp")
            } else {
                PathBuf::from("/nonexistent/path")
            },
            branch: None,
            created_at,
            updated_at: created_at,
            last_synced: None,
            metadata: serde_json::Value::Null,
        }
    }

    #[test]
    fn test_orphan_detection_missing_workspace() {
        let session = mock_session("test", 1, false);
        let now = Utc::now();
        let threshold = Duration::from_secs(7200);

        let result = is_orphan_candidate(&session, &threshold, &now);

        assert!(result.is_some());
        let orphan = result.unwrap_or_else(|| panic!("Expected orphan"));
        assert!(!orphan.workspace_exists);
    }

    #[test]
    fn test_orphan_detection_old_no_bead() {
        let session = mock_session("test", 3, true);
        let now = Utc::now();
        let threshold = Duration::from_secs(7200);

        let result = is_orphan_candidate(&session, &threshold, &now);

        assert!(result.is_some());
        let orphan = result.unwrap_or_else(|| panic!("Expected orphan"));
        assert!(orphan.workspace_exists);
        assert!(!orphan.has_active_bead);
    }

    #[test]
    fn test_not_orphan_recent() {
        let session = mock_session("test", 1, true);
        let now = Utc::now();
        let threshold = Duration::from_secs(7200);

        let result = is_orphan_candidate(&session, &threshold, &now);

        assert!(result.is_none());
    }

    #[test]
    fn test_should_clean_missing_workspace() {
        let orphan = OrphanCandidate {
            name: "test".to_string(),
            workspace_path: "/nonexistent".to_string(),
            age_seconds: 3600,
            has_active_bead: false,
            workspace_exists: false,
        };

        assert!(should_clean(&orphan));
    }

    #[test]
    fn test_should_not_clean_active_bead() {
        let orphan = OrphanCandidate {
            name: "test".to_string(),
            workspace_path: "/tmp".to_string(),
            age_seconds: 10800,
            has_active_bead: true,
            workspace_exists: true,
        };

        assert!(!should_clean(&orphan));
    }

    #[test]
    fn test_config_defaults() {
        let config = PeriodicCleanupConfig::default();

        assert_eq!(config.interval, Duration::from_secs(3600));
        assert_eq!(config.age_threshold, Duration::from_secs(7200));
        assert!(!config.dry_run);
    }

    #[test]
    fn test_categorize_orphans() {
        let orphans = vec![
            OrphanCandidate {
                name: "cleanable".to_string(),
                workspace_path: "/tmp".to_string(),
                age_seconds: 10800,
                has_active_bead: false,
                workspace_exists: true,
            },
            OrphanCandidate {
                name: "has_bead".to_string(),
                workspace_path: "/tmp".to_string(),
                age_seconds: 10800,
                has_active_bead: true,
                workspace_exists: true,
            },
        ];

        let (cleanable, skipped) = categorize_orphans(orphans);

        assert_eq!(cleanable.len(), 1);
        assert_eq!(skipped.len(), 1);
        assert_eq!(cleanable[0].name, "cleanable");
        assert_eq!(skipped[0].name, "has_bead");
    }
}
