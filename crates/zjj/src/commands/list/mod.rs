//! List all sessions
//!
//! This module provides functionality to list all sessions with their status,
//! branch information, change counts, and beads issue counts.

pub mod data;
pub mod formatting;
pub mod types;

// Re-export public API
use anyhow::Result;
pub use types::ListFilter;

use crate::{cli::is_tty, commands::get_session_db, session::SessionStatus};

/// Run the list command
pub async fn run(all: bool, json: bool, silent: bool, filter: ListFilter) -> Result<()> {
    let db = get_session_db().await?;

    // Filter sessions: exclude completed/failed unless --all is used
    let all_sessions: Vec<_> = db.list(None).await?;

    // Filter out completed and failed unless --all flag is set
    let status_filtered: Vec<_> = if all {
        all_sessions
    } else {
        all_sessions
            .into_iter()
            .filter(|s| s.status != SessionStatus::Completed && s.status != SessionStatus::Failed)
            .collect()
    };

    // Apply additional filters (bead_id, agent_id, with_beads, with_agents)
    let sessions = data::apply_filters(status_filtered, &filter);

    if sessions.is_empty() {
        if json {
            // Output empty response with schema metadata
            formatting::output_json(Vec::new(), &filter)?;
        } else if silent || !is_tty() {
            // Silent mode or pipe: output nothing
        } else {
            println!("No sessions found.");
            println!("Use 'zjj add <name>' to create a session.");
        }
        return Ok(());
    }

    // Get beads count once for all sessions (it's the same for all)
    let beads = data::get_beads_count().await.unwrap_or_default();

    // Format sessions for display using the output module
    let items = data::format_sessions(sessions.as_slice(), &beads);

    if json {
        formatting::output_json(items, &filter)?;
    } else if silent || !is_tty() {
        formatting::output_minimal(&items);
    } else {
        formatting::output_table(&items);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::{
        database::SessionDb,
        session::{SessionStatus, SessionUpdate},
    };

    async fn setup_test_db() -> Result<(SessionDb, TempDir)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;
        Ok((db, dir))
    }

    #[test]
    fn test_filter_completed_and_failed_sessions() -> Result<()> {
        tokio_test::block_on(async {
            let (db, _dir) = setup_test_db().await?;
            // Create sessions with different statuses
            let s1 = db.create("active-session", "/tmp/active").await?;
            db.update(
                &s1.name,
                SessionUpdate {
                    status: Some(SessionStatus::Active),
                    ..Default::default()
                },
            )
            .await?;
            let s2 = db.create("completed-session", "/tmp/completed").await?;
            db.update(
                &s2.name,
                SessionUpdate {
                    status: Some(SessionStatus::Completed),
                    ..Default::default()
                },
            )
            .await?;
            let s3 = db.create("failed-session", "/tmp/failed").await?;
            db.update(
                &s3.name,
                SessionUpdate {
                    status: Some(SessionStatus::Failed),
                    ..Default::default()
                },
            )
            .await?;
            let s4 = db.create("paused-session", "/tmp/paused").await?;
            db.update(
                &s4.name,
                SessionUpdate {
                    status: Some(SessionStatus::Paused),
                    ..Default::default()
                },
            )
            .await?;
            // Get all sessions and filter
            let mut sessions = db.list(None).await?;
            // Simulate the filtering logic from run()
            sessions.retain(|s| {
                s.status != SessionStatus::Completed && s.status != SessionStatus::Failed
            });
            // Should only have active and paused
            assert_eq!(sessions.len(), 2);
            assert!(sessions.iter().any(|s| s.name == "active-session"));
            assert!(sessions.iter().any(|s| s.name == "paused-session"));
            assert!(!sessions.iter().any(|s| s.name == "completed-session"));
            assert!(!sessions.iter().any(|s| s.name == "failed-session"));
            Ok(())
        })
    }

    #[test]
    fn test_all_flag_includes_all_sessions() -> Result<()> {
        tokio_test::block_on(async {
            let (db, _dir) = setup_test_db().await?;
            // Create sessions with different statuses
            db.create("active-session", "/tmp/active").await?;
            let s2 = db.create("completed-session", "/tmp/completed").await?;
            db.update(
                &s2.name,
                SessionUpdate {
                    status: Some(SessionStatus::Completed),
                    ..Default::default()
                },
            )
            .await?;
            // With all=true, no filtering
            let sessions = db.list(None).await?;
            assert_eq!(sessions.len(), 2);
            // With all=false, filter out completed
            let mut filtered = db.list(None).await?;
            filtered.retain(|s| {
                s.status != SessionStatus::Completed && s.status != SessionStatus::Failed
            });
            assert_eq!(filtered.len(), 1);
            Ok(())
        })
    }

    #[test]
    fn test_empty_list_handling() -> Result<()> {
        tokio_test::block_on(async {
            let (db, _dir) = setup_test_db().await?;
            let sessions = db.list(None).await?;
            assert!(sessions.is_empty());
            Ok(())
        })
    }
}
