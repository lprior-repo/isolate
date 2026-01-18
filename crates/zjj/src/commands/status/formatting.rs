//! Status output formatting functions

use anyhow::Result;

use super::types::SessionStatusInfo;

/// Output sessions as formatted table
pub fn output_table(items: &[SessionStatusInfo]) {
    println!(
        "{:<20} {:<12} {:<15} {:<20} {:<15} {:<20}",
        "NAME", "STATUS", "BRANCH", "CHANGES", "DIFF", "BEADS"
    );
    println!("{}", "-".repeat(105));

    for item in items {
        println!(
            "{:<20} {:<12} {:<15} {:<20} {:<15} {:<20}",
            item.name, item.status, item.branch, item.changes, item.diff_stats, item.beads
        );
    }
}

/// Output sessions as JSON
pub fn output_json(items: &[SessionStatusInfo]) -> Result<()> {
    let json = serde_json::to_string_pretty(items)?;
    println!("{json}");
    Ok(())
}

/// Output empty state message
pub fn output_empty(json: bool) {
    if json {
        println!("[]");
    } else {
        println!("No sessions found.");
        println!("Use 'zjj add <name>' to create a session.");
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::types::{BeadStats, DiffStats, FileChanges},
        *,
    };
    use crate::session::{Session, SessionStatus};

    #[test]
    fn test_output_table_format() {
        let session = Session {
            id: Some(1),
            name: "test".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "zjj:test".to_string(),
            branch: Some("main".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let items = vec![SessionStatusInfo {
            name: session.name.clone(),
            status: "active".to_string(),
            workspace_path: session.workspace_path.clone(),
            branch: "main".to_string(),
            changes: FileChanges {
                modified: 2,
                added: 1,
                deleted: 0,
                renamed: 0,
                unknown: 0,
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
        }];

        // This test just verifies the function doesn't panic
        output_table(&items);
    }

    #[test]
    fn test_output_json_format() {
        let session = Session {
            id: Some(1),
            name: "test".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "zjj:test".to_string(),
            branch: Some("main".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let items = vec![SessionStatusInfo {
            name: session.name.clone(),
            status: "active".to_string(),
            workspace_path: session.workspace_path.clone(),
            branch: "main".to_string(),
            changes: FileChanges::default(),
            diff_stats: DiffStats::default(),
            beads: BeadStats::default(),
            session,
        }];

        let result = output_json(&items);
        assert!(result.is_ok());
    }
}
