//! Output formatting functions for list command

use anyhow::Result;

use super::types::SessionListItem;

/// Format runtime duration as human-readable string
///
/// Returns format like "2h 15m" for hours and minutes, or "45m" for minutes only
#[must_use]
pub fn format_runtime(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;

    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

/// Output sessions as formatted table
pub fn output_table(items: &[SessionListItem]) {
    // Detect if any sessions have beads or agents
    let has_beads = items.iter().any(|item| item.bead.is_some());
    let has_agents = items.iter().any(|item| item.agent.is_some());

    // Build header based on available data
    let mut header = format!(
        "{:<20} {:<12} {:<15} {:<10} {:<12}",
        "NAME", "STATUS", "BRANCH", "CHANGES", "BEADS"
    );

    if has_beads {
        header.push_str(&format!(" {:<12} {:<10}", "BEAD", "PRIORITY"));
    }

    if has_agents {
        header.push_str(&format!(" {:<20} {:<10}", "AGENT", "RUNTIME"));
    }

    println!("{header}");

    // Calculate separator width dynamically
    let separator_width = 70 + if has_beads { 24 } else { 0 } + if has_agents { 32 } else { 0 };
    println!("{}", "-".repeat(separator_width));

    for item in items {
        let mut row = format!(
            "{:<20} {:<12} {:<15} {:<10} {:<12}",
            item.name, item.status, item.branch, item.changes, item.beads
        );

        if has_beads {
            let bead_id = item.bead.as_ref().map_or("-", |b| b.id.as_str());
            let priority = item
                .bead
                .as_ref()
                .and_then(|b| b.priority.as_deref())
                .unwrap_or("-");
            row.push_str(&format!(" {:<12} {:<10}", bead_id, priority));
        }

        if has_agents {
            let agent_id = item.agent.as_ref().map_or("-", |a| a.agent_id.as_str());
            let runtime = item
                .agent
                .as_ref()
                .and_then(|a| a.runtime_seconds)
                .map_or_else(|| "-".to_string(), format_runtime);
            row.push_str(&format!(" {:<20} {:<10}", agent_id, runtime));
        }

        println!("{row}");
    }
}

/// Output sessions as minimal tab-separated format (pipe-friendly)
/// Format: name\tstatus\tbranch\tchanges\tbeads
pub fn output_minimal(items: &[SessionListItem]) {
    for item in items {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            item.name, item.status, item.branch, item.changes, item.beads
        );
    }
}

/// Output sessions as JSON
pub fn output_json(items: &[SessionListItem]) -> Result<()> {
    let json = serde_json::to_string_pretty(items)?;
    println!("{json}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::list::types::{SessionAgentInfo, SessionBeadInfo};

    #[test]
    fn test_format_runtime_hours_minutes() {
        assert_eq!(format_runtime(7865), "2h 11m");
        assert_eq!(format_runtime(3600), "1h 0m");
        assert_eq!(format_runtime(7200), "2h 0m");
    }

    #[test]
    fn test_format_runtime_minutes_only() {
        assert_eq!(format_runtime(300), "5m");
        assert_eq!(format_runtime(1800), "30m");
        assert_eq!(format_runtime(60), "1m");
    }

    #[test]
    fn test_format_runtime_zero() {
        assert_eq!(format_runtime(0), "0m");
    }

    #[test]
    fn test_output_table_format() {
        let items = vec![SessionListItem {
            name: "test-session".to_string(),
            status: "active".to_string(),
            branch: "main".to_string(),
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "jjz:test-session".to_string(),
            changes: "5".to_string(),
            beads: "3/2/1".to_string(),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            bead: None,
            agent: None,
        }];

        // This test just verifies the function doesn't panic
        output_table(&items);
    }

    #[test]
    fn test_output_table_with_bead_and_agent() {
        let items = vec![SessionListItem {
            name: "test-session".to_string(),
            status: "active".to_string(),
            branch: "main".to_string(),
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "jjz:test-session".to_string(),
            changes: "5".to_string(),
            beads: "3/2/1".to_string(),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            bead: Some(SessionBeadInfo {
                id: "zjj-1234".to_string(),
                title: Some("Fix bug".to_string()),
                status: Some("open".to_string()),
                priority: Some("high".to_string()),
                bead_type: Some("bug".to_string()),
            }),
            agent: Some(SessionAgentInfo {
                agent_id: "claude-code-5678".to_string(),
                task_id: Some("zjj-1234".to_string()),
                spawned_at: Some(1_000_000_000),
                runtime_seconds: Some(3665),
            }),
        }];

        // This test just verifies the function doesn't panic with enhanced columns
        output_table(&items);
    }

    #[test]
    fn test_output_json_format() {
        let items = vec![SessionListItem {
            name: "test-session".to_string(),
            status: "active".to_string(),
            branch: "main".to_string(),
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "jjz:test-session".to_string(),
            changes: "5".to_string(),
            beads: "3/2/1".to_string(),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            bead: None,
            agent: None,
        }];

        let result = output_json(&items);
        assert!(result.is_ok());
    }
}
