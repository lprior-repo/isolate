//! Status data types and display implementations

use serde::Serialize;

use crate::session::Session;

/// Detailed session status information
#[derive(Debug, Clone, Serialize)]
pub struct SessionStatusInfo {
    pub name: String,
    pub status: String,
    pub workspace_path: String,
    pub branch: String,
    pub changes: FileChanges,
    pub diff_stats: DiffStats,
    pub beads: BeadStats,
    #[serde(flatten)]
    pub session: Session,
}

/// File changes in the workspace
#[derive(Debug, Clone, Default, Serialize)]
pub struct FileChanges {
    pub modified: usize,
    pub added: usize,
    pub deleted: usize,
    pub renamed: usize,
    pub unknown: usize,
}

impl FileChanges {
    pub const fn total(&self) -> usize {
        self.modified
            .saturating_add(self.added)
            .saturating_add(self.deleted)
            .saturating_add(self.renamed)
    }

    pub const fn is_clean(&self) -> bool {
        self.total() == 0
    }
}

impl std::fmt::Display for FileChanges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_clean() {
            write!(f, "clean")
        } else {
            write!(
                f,
                "M:{} A:{} D:{} R:{}",
                self.modified, self.added, self.deleted, self.renamed
            )
        }
    }
}

/// Diff statistics (insertions/deletions)
#[derive(Debug, Clone, Default, Serialize)]
pub struct DiffStats {
    pub insertions: usize,
    pub deletions: usize,
}

impl std::fmt::Display for DiffStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "+{} -{}", self.insertions, self.deletions)
    }
}

/// Beads issue statistics
#[derive(Debug, Clone, Default, Serialize)]
pub struct BeadStats {
    pub open: usize,
    pub in_progress: usize,
    pub blocked: usize,
    pub closed: usize,
}

impl std::fmt::Display for BeadStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "O:{} P:{} B:{} C:{}",
            self.open, self.in_progress, self.blocked, self.closed
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionStatus;

    #[test]
    fn test_file_changes_total() {
        let changes = FileChanges {
            modified: 2,
            added: 3,
            deleted: 1,
            renamed: 1,
            unknown: 0,
        };
        assert_eq!(changes.total(), 7);
    }

    #[test]
    fn test_file_changes_is_clean() {
        let clean = FileChanges::default();
        assert!(clean.is_clean());

        let dirty = FileChanges {
            modified: 1,
            ..Default::default()
        };
        assert!(!dirty.is_clean());
    }

    #[test]
    fn test_file_changes_display_clean() {
        let changes = FileChanges::default();
        assert_eq!(changes.to_string(), "clean");
    }

    #[test]
    fn test_file_changes_display_dirty() {
        let changes = FileChanges {
            modified: 2,
            added: 3,
            deleted: 1,
            renamed: 1,
            unknown: 0,
        };
        assert_eq!(changes.to_string(), "M:2 A:3 D:1 R:1");
    }

    #[test]
    fn test_diff_stats_display() {
        let stats = DiffStats {
            insertions: 123,
            deletions: 45,
        };
        assert_eq!(stats.to_string(), "+123 -45");
    }

    #[test]
    fn test_diff_stats_default() {
        let stats = DiffStats::default();
        assert_eq!(stats.insertions, 0);
        assert_eq!(stats.deletions, 0);
        assert_eq!(stats.to_string(), "+0 -0");
    }

    #[test]
    fn test_bead_stats_display() {
        let stats = BeadStats {
            open: 5,
            in_progress: 3,
            blocked: 2,
            closed: 10,
        };
        assert_eq!(stats.to_string(), "O:5 P:3 B:2 C:10");
    }

    #[test]
    fn test_bead_stats_default() {
        let stats = BeadStats::default();
        assert_eq!(stats.open, 0);
        assert_eq!(stats.in_progress, 0);
        assert_eq!(stats.blocked, 0);
        assert_eq!(stats.closed, 0);
    }

    #[test]
    fn test_session_status_info_serialization() -> anyhow::Result<()> {
        let session = Session {
            id: Some(1),
            name: "test-session".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "jjz:test-session".to_string(),
            branch: Some("feature".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let info = SessionStatusInfo {
            name: session.name.clone(),
            status: session.status.to_string(),
            workspace_path: session.workspace_path.clone(),
            branch: session.branch.clone().unwrap_or_else(|| "-".to_string()),
            changes: FileChanges {
                modified: 2,
                added: 1,
                deleted: 0,
                renamed: 0,
                unknown: 1,
            },
            diff_stats: DiffStats {
                insertions: 50,
                deletions: 10,
            },
            beads: BeadStats {
                open: 3,
                in_progress: 1,
                blocked: 0,
                closed: 5,
            },
            session,
        };

        let json = serde_json::to_string(&info)?;
        assert!(json.contains("\"name\":\"test-session\""));
        assert!(json.contains("\"status\":\"active\""));
        assert!(json.contains("\"modified\":2"));
        assert!(json.contains("\"insertions\":50"));
        assert!(json.contains("\"open\":3"));
        Ok(())
    }

    #[test]
    fn test_file_changes_with_unknown_files() {
        let changes = FileChanges {
            modified: 1,
            added: 0,
            deleted: 0,
            renamed: 0,
            unknown: 3,
        };
        // Unknown files don't count toward total
        assert_eq!(changes.total(), 1);
        assert!(!changes.is_clean());
    }
}
