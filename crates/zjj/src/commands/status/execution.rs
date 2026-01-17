//! Status command execution logic

use std::io::Write;

use anyhow::Result;
use tokio::time::{sleep, Duration};

use super::{formatting, gathering::gather_session_status, types::SessionStatusInfo};
use crate::commands::get_session_db;

/// Run the status command
pub async fn run(name: Option<&str>, json: bool, watch: bool) -> Result<()> {
    if watch {
        run_watch_mode(name, json).await
    } else {
        run_once(name, json).await
    }
}

/// Run status once
async fn run_once(name: Option<&str>, json: bool) -> Result<()> {
    let db = get_session_db().await?;

    let sessions = if let Some(session_name) = name {
        // Get single session
        let session = db
            .get(session_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session '{session_name}' not found"))?;
        vec![session]
    } else {
        // Get all sessions
        db.list(None).await?
    };

    if sessions.is_empty() {
        formatting::output_empty(json);
        return Ok(());
    }

    // Gather status for all sessions
    let statuses = gather_statuses(&sessions).await?;

    if json {
        formatting::output_json(&statuses)?;
    } else {
        formatting::output_table(&statuses);
    }

    Ok(())
}

/// Run status in watch mode (continuous updates)
async fn run_watch_mode(name: Option<&str>, json: bool) -> Result<()> {
    loop {
        // Clear screen (ANSI escape code)
        if !json {
            print!("\x1B[2J\x1B[1;1H");
            std::io::stdout().flush()?;
        }

        // Run status once
        if let Err(e) = run_once(name, json).await {
            if !json {
                eprintln!("Error: {e}");
            }
        }

        // Wait 1 second
        sleep(Duration::from_secs(1)).await;
    }
}

/// Gather status for all sessions
async fn gather_statuses(sessions: &[crate::session::Session]) -> Result<Vec<SessionStatusInfo>> {
    // Functional approach using manual async fold pattern
    // Sequential execution (I/O bound operations, no parallelism needed)
    let mut statuses = Vec::with_capacity(sessions.len());

    // Use iterator to make intent clear (async requires sequential await)
    for session in sessions {
        statuses.push(gather_session_status(session).await?);
    }

    Ok(statuses)
}
