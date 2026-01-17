//! State management for beads tracking
//!
//! Provides functions to query beads database status and track issue counts
//! across workspaces. State is derived from the live beads database rather
//! than stored, ensuring consistency with actual bead state.

use std::path::Path;

use crate::{Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Beads status for a workspace
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeadsStatus {
    /// No beads database found
    NoBeads,
    /// Beads database with issue counts
    Counts {
        open: u32,
        in_progress: u32,
        blocked: u32,
        closed: u32,
    },
}

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

/// Query beads status for a workspace
///
/// # Errors
///
/// Returns error if:
/// - Unable to open database
/// - Database query fails
/// - Database schema is invalid
pub async fn query_beads_status(workspace_path: &Path) -> Result<BeadsStatus> {
    use crate::beads::query_beads;

    let issues = query_beads(workspace_path)
        .await
        .map_err(|e| Error::database_error(e.to_string()))?;

    if issues.is_empty() {
        return Ok(BeadsStatus::NoBeads);
    }

    // Count by status using functional fold
    let (open, in_progress, blocked, closed) =
        issues
            .iter()
            .fold((0u32, 0u32, 0u32, 0u32), |(o, i, b, c), issue| {
                match issue.status {
                    crate::beads::IssueStatus::Open => (o.saturating_add(1), i, b, c),
                    crate::beads::IssueStatus::InProgress => (o, i.saturating_add(1), b, c),
                    crate::beads::IssueStatus::Blocked => (o, i, b.saturating_add(1), c),
                    crate::beads::IssueStatus::Closed => (o, i, b, c.saturating_add(1)),
                    crate::beads::IssueStatus::Deferred => (o, i, b, c), // Don't count deferred
                }
            });

    Ok(BeadsStatus::Counts {
        open,
        in_progress,
        blocked,
        closed,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Test 1: Query beads status - no beads
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    #[ignore = "TODO: Fix test to work with forbid(clippy::unwrap_used)"]
    fn test_query_beads_status_no_beads() {
        // This test requires async runtime and Result handling that conflicts
        // with workspace forbid lints. Need to refactor test approach.
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Test 2: Query beads status - with valid database
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    // TEMPORARILY DISABLED: Uses rusqlite which was removed during sqlx migration (zjj-5ld.1)
    // Will be re-enabled in zjj-5ld.2 with async sqlx implementation

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Test 3: BeadsStatus equality
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_beads_status_equality() {
        let status1 = BeadsStatus::Counts {
            open: 1,
            in_progress: 2,
            blocked: 0,
            closed: 3,
        };
        let status2 = BeadsStatus::Counts {
            open: 1,
            in_progress: 2,
            blocked: 0,
            closed: 3,
        };
        let status3 = BeadsStatus::NoBeads;

        assert_eq!(status1, status2);
        assert_ne!(status1, status3);
    }
}
