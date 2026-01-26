//! Universal context command
//!
//! Provides complete environment state information for AI agents and programmatic access.

pub mod types;

use std::path::PathBuf;

use anyhow::Result;
pub use types::{
    BeadsContext, ContextOutput, HealthStatus, Location, RepositoryContext, SessionContext,
};
use zjj_core::OutputFormat;

use crate::commands::{check_in_jj_repo, get_session_db};

pub fn run(json: bool, field: Option<&str>, no_beads: bool, no_health: bool) -> Result<()> {
    let format = OutputFormat::from_json_flag(json);

    let context = gather_context(no_beads, no_health)?;

    if let Some(field_path) = field {
        extract_and_print_field(&context, field_path)?;
    } else {
        output_context(&context, format)?;
    }

    Ok(())
}

fn gather_context(no_beads: bool, no_health: bool) -> Result<ContextOutput> {
    let root = check_in_jj_repo()?;
    let location = detect_location(&root)?;

    let repository_context = get_repository_context(&root)?;

    let session_context = if matches!(location, Location::Workspace { .. }) {
        Some(get_session_info()?)
    } else {
        None
    };

    let beads_context = if no_beads { None } else { get_beads_context()? };

    let health_status = if no_health {
        HealthStatus::Good
    } else {
        check_health(&root, session_context.as_ref())
    };

    let suggestions = generate_suggestions(&location, &health_status, &repository_context);

    Ok(ContextOutput {
        location,
        session: session_context,
        repository: repository_context,
        beads: beads_context,
        health: health_status,
        suggestions,
    })
}

fn detect_location(root: &PathBuf) -> Result<Location> {
    let current_dir = std::env::current_dir()?;

    if current_dir == *root {
        return Ok(Location::Main);
    }

    let workspaces_dir = root.join(".zjj/workspaces");

    if current_dir.starts_with(&workspaces_dir) {
        let workspace_name = current_dir
            .strip_prefix(&workspaces_dir)?
            .components()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Unable to determine workspace name"))?
            .as_os_str()
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Workspace name contains invalid UTF-8"))?
            .to_string();

        return Ok(Location::Workspace {
            name: workspace_name,
            path: current_dir.to_string_lossy().to_string(),
        });
    }

    Ok(Location::Main)
}

fn get_repository_context(root: &PathBuf) -> Result<RepositoryContext> {
    let branch = get_current_branch(root)?;
    let uncommitted_files = count_uncommitted_files(root)?;
    let has_conflicts = check_conflicts(root)?;
    let commits_ahead = count_commits_ahead(root)?;

    Ok(RepositoryContext {
        root: root.to_string_lossy().to_string(),
        branch,
        uncommitted_files,
        commits_ahead,
        has_conflicts,
    })
}

fn get_current_branch(root: &PathBuf) -> Result<String> {
    let output = std::process::Command::new("jj")
        .current_dir(root)
        .args(["log", "-r", "@", "--no-graph", "-T", "change_id"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to get current branch: {e}"))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to get current branch: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let change_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(change_id)
}

fn count_uncommitted_files(root: &PathBuf) -> Result<usize> {
    let output = std::process::Command::new("jj")
        .current_dir(root)
        .args(["status", "--no-pager"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to get uncommitted files: {e}"))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to get uncommitted files: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Count lines that start with file change indicators (A, M, D, R, C)
    let count = stdout
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("A ")
                || trimmed.starts_with("M ")
                || trimmed.starts_with("D ")
                || trimmed.starts_with("R ")
                || trimmed.starts_with("C ")
        })
        .count();
    Ok(count)
}

fn check_conflicts(root: &PathBuf) -> Result<bool> {
    let output = std::process::Command::new("jj")
        .current_dir(root)
        .args(["status", "--no-pager"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to check for conflicts: {e}"))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to check for conflicts: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.contains("Conflicting"))
}

fn count_commits_ahead(root: &PathBuf) -> Result<usize> {
    let output = std::process::Command::new("jj")
        .current_dir(root)
        .args(["log", "-r", "@..@", "--no-graph", "-T", "commit_id"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to count commits ahead: {e}"))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to count commits ahead: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout.lines().filter(|line| !line.is_empty()).count();
    Ok(count)
}

fn get_session_info() -> Result<SessionContext> {
    let session_db = get_session_db()?;
    let sessions = session_db.list(None)?;

    let current_workspace = std::env::current_dir()?;
    let workspace_name = current_workspace
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Unable to determine workspace name"))?;

    let session = sessions
        .iter()
        .find(|s| s.name == workspace_name)
        .ok_or_else(|| anyhow::anyhow!("Session not found for workspace: {workspace_name}"))?;

    let bead_id = session
        .metadata
        .as_ref()
        .and_then(|m| m.get("bead_id"))
        .and_then(|v| v.as_str())
        .map(String::from);

    #[allow(clippy::cast_possible_wrap)]
    let created_at = chrono::DateTime::from_timestamp(session.created_at as i64, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid created_at timestamp"))?;

    #[allow(clippy::cast_possible_wrap)]
    let last_synced = session
        .last_synced
        .map(|ts| {
            chrono::DateTime::from_timestamp(ts as i64, 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid last_synced timestamp"))
        })
        .transpose()?;

    Ok(SessionContext {
        name: session.name.clone(),
        status: session.status.to_string(),
        bead_id,
        created_at,
        last_synced,
    })
}

fn get_beads_context() -> Result<Option<BeadsContext>> {
    let beads_dir = std::path::Path::new(".beads");
    if !beads_dir.exists() {
        return Ok(None);
    }

    let beads_db = beads_dir.join("issues.jsonl");
    if !beads_db.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&beads_db)?;
    let mut active: Option<String> = None;
    let mut blocked_by: Vec<String> = Vec::new();
    let mut ready_count = 0usize;
    let mut in_progress_count = 0usize;

    for line in content.lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(status) = json.get("status").and_then(|s| s.as_str()) {
                match status {
                    "in_progress" => {
                        in_progress_count += 1;
                        active = active
                            .or_else(|| json.get("id").and_then(|i| i.as_str()).map(String::from));
                    }
                    "ready" | "●" => ready_count += 1,
                    "blocked" => {
                        if let Some(id) = json.get("id").and_then(|i| i.as_str()) {
                            blocked_by.push(id.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(Some(BeadsContext {
        active,
        blocked_by,
        ready_count,
        in_progress_count,
    }))
}

fn check_health(root: &std::path::Path, session_context: Option<&SessionContext>) -> HealthStatus {
    let mut warnings: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    let db_path = root.join(".zjj/sessions.db");
    if !db_path.exists() {
        errors.push("Session database not found".to_string());
    }

    if let Some(session) = session_context {
        let workspace_path = root.join(".zjj/workspaces").join(&session.name);
        if !workspace_path.exists() {
            warnings.push(format!(
                "Workspace path missing for session: {}",
                session.name
            ));
        }
    }

    if errors.is_empty() && warnings.is_empty() {
        HealthStatus::Good
    } else if !errors.is_empty() {
        HealthStatus::Error { critical: errors }
    } else {
        HealthStatus::Warn { issues: warnings }
    }
}

fn generate_suggestions(
    location: &Location,
    health: &HealthStatus,
    repo: &RepositoryContext,
) -> Vec<String> {
    let mut suggestions = Vec::new();

    match location {
        Location::Main => {
            suggestions.push("Use 'zjj add <name>' to create a workspace".to_string());
        }
        Location::Workspace { name, .. } => {
            suggestions.push(format!("Working in workspace: {name}"));
            if repo.uncommitted_files > 0 {
                suggestions.push(format!(
                    "You have {} uncommitted files. Use 'jj status' to review.",
                    repo.uncommitted_files
                ));
            }
        }
    }

    match health {
        HealthStatus::Warn { issues } => {
            for issue in issues {
                suggestions.push(format!("Warning: {issue}"));
            }
        }
        HealthStatus::Error { critical } => {
            for error in critical {
                suggestions.push(format!("Error: {error}"));
            }
        }
        HealthStatus::Good => {}
    }

    suggestions
}

fn output_context(context: &ContextOutput, format: OutputFormat) -> Result<()> {
    if format.is_json() {
        println!("{}", serde_json::to_string_pretty(context)?);
    } else {
        print_human_readable(context);
    }
    Ok(())
}

fn print_human_readable(context: &ContextOutput) {
    match &context.location {
        Location::Main => {
            println!("📍 Location: Main branch");
        }
        Location::Workspace { name, .. } => {
            println!("📍 Location: Workspace '{name}'");
        }
    }

    if let Some(ref session) = context.session {
        println!("🎯 Session: {} ({})", session.name, session.status);
    }

    println!("🌿 Branch: {}", context.repository.branch);
    println!(
        "📊 Uncommitted: {} files",
        context.repository.uncommitted_files
    );
    println!("⬆️  Ahead: {} commits", context.repository.commits_ahead);

    if context.repository.has_conflicts {
        println!("⚠️  Conflicts detected!");
    }

    if let Some(ref beads) = context.beads {
        if let Some(ref active) = beads.active {
            println!("🔴 Active task: {active}");
        }
        println!("📋 Ready tasks: {}", beads.ready_count);
        if !beads.blocked_by.is_empty() {
            println!("🚫 Blocked by: {}", beads.blocked_by.join(", "));
        }
    }

    match &context.health {
        HealthStatus::Good => println!("✅ Health: Good"),
        HealthStatus::Warn { issues } => {
            println!("⚠️  Health: Warning");
            for issue in issues {
                println!("  - {issue}");
            }
        }
        HealthStatus::Error { critical } => {
            println!("❌ Health: Error");
            for error in critical {
                println!("  - {error}");
            }
        }
    }

    if !context.suggestions.is_empty() {
        println!("\n💡 Suggestions:");
        for suggestion in &context.suggestions {
            println!("  • {suggestion}");
        }
    }
}

fn extract_and_print_field(context: &ContextOutput, field_path: &str) -> Result<()> {
    let json_value = serde_json::to_value(context)?;
    let pointer = format!("/{}", field_path.replace('.', "/"));

    let value = json_value
        .pointer(&pointer)
        .ok_or_else(|| anyhow::anyhow!("Field not found: {field_path}"))?;

    println!("{value}");
    Ok(())
}
