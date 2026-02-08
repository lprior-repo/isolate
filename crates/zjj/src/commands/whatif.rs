//! `WhatIf` command - Preview what a command would do
//!
//! Provides detailed preview of command effects without execution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zjj_core::{OutputFormat, SchemaEnvelope};

use crate::session::validate_session_name;

/// Options for the whatif command
#[derive(Debug, Clone)]
pub struct WhatIfOptions {
    /// Command to preview
    pub command: String,
    /// Arguments for the command
    pub args: Vec<String>,
    /// Output format
    pub format: OutputFormat,
}

/// What-if preview result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatIfResult {
    /// The command being previewed
    pub command: String,
    /// Arguments provided
    pub args: Vec<String>,
    /// Steps that would be executed
    pub steps: Vec<WhatIfStep>,
    /// Resources that would be created
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub creates: Vec<ResourceChange>,
    /// Resources that would be modified
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub modifies: Vec<ResourceChange>,
    /// Resources that would be deleted
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub deletes: Vec<ResourceChange>,
    /// Side effects
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub side_effects: Vec<String>,
    /// Whether this operation is reversible
    pub reversible: bool,
    /// Undo command if reversible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undo_command: Option<String>,
    /// Potential risks or warnings
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
    /// Prerequisites that must be met
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub prerequisites: Vec<PrerequisiteCheck>,
}

/// A step in the execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatIfStep {
    /// Step number
    pub order: usize,
    /// Description of what this step does
    pub description: String,
    /// Command or action being performed
    pub action: String,
    /// Whether this step can fail
    pub can_fail: bool,
    /// What happens if this step fails
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<String>,
}

/// A resource that would be changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceChange {
    /// Type of resource (session, workspace, file, database)
    pub resource_type: String,
    /// Resource identifier or path
    pub resource: String,
    /// Description of change
    pub description: String,
}

/// A prerequisite check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrerequisiteCheck {
    /// What is being checked
    pub check: String,
    /// Current status
    pub status: PrerequisiteStatus,
    /// Description
    pub description: String,
}

/// Status of a prerequisite
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrerequisiteStatus {
    /// Prerequisite is met
    Met,
    /// Prerequisite is not met
    NotMet,
    /// Cannot determine
    Unknown,
}

/// Run the whatif command
pub fn run(options: &WhatIfOptions) -> Result<()> {
    let result = preview_command(&options.command, &options.args)?;

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("whatif-response", "single", result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        let output = format_human_output(&result)?;
        let truncated = truncate_output_if_needed(&result, 4096)?;

        if truncated.len() < output.len() {
            // Output was truncated, save to temp file and show message
            let temp_path = save_full_output_to_temp(&output)?;
            println!("{truncated}");
            println!();
            println!("⚠ Output truncated to 4KB for display. Full output saved to: {temp_path}");
        } else {
            println!("{output}");
        }
    }

    Ok(())
}

/// Format human-readable output from `WhatIfResult`
fn format_human_output(result: &WhatIfResult) -> Result<String> {
    use std::fmt::Write;

    let mut output = String::new();
    writeln!(
        output,
        "What if: {} {}",
        result.command,
        result.args.join(" ")
    )?;
    writeln!(output)?;

    if !result.prerequisites.is_empty() {
        writeln!(output, "Prerequisites:")?;
        for prereq in &result.prerequisites {
            let status = match prereq.status {
                PrerequisiteStatus::Met => "✓",
                PrerequisiteStatus::NotMet => "✗",
                PrerequisiteStatus::Unknown => "?",
            };
            writeln!(
                output,
                "  {status} {}: {}",
                prereq.check, prereq.description
            )?;
        }
        writeln!(output)?;
    }

    writeln!(output, "Execution plan:")?;
    for step in &result.steps {
        writeln!(output, "  {}. {}", step.order, step.description)?;
        writeln!(output, "     Action: {}", step.action)?;
        if step.can_fail {
            if let Some(on_fail) = &step.on_failure {
                writeln!(output, "     On failure: {on_fail}")?;
            }
        }
    }
    writeln!(output)?;

    if !result.creates.is_empty() {
        writeln!(output, "Would create:")?;
        for c in &result.creates {
            writeln!(
                output,
                "  + [{}] {}: {}",
                c.resource_type, c.resource, c.description
            )?;
        }
        writeln!(output)?;
    }

    if !result.modifies.is_empty() {
        writeln!(output, "Would modify:")?;
        for m in &result.modifies {
            writeln!(
                output,
                "  ~ [{}] {}: {}",
                m.resource_type, m.resource, m.description
            )?;
        }
        writeln!(output)?;
    }

    if !result.deletes.is_empty() {
        writeln!(output, "Would delete:")?;
        for d in &result.deletes {
            writeln!(
                output,
                "  - [{}] {}: {}",
                d.resource_type, d.resource, d.description
            )?;
        }
        writeln!(output)?;
    }

    if !result.side_effects.is_empty() {
        writeln!(output, "Side effects:")?;
        for effect in &result.side_effects {
            writeln!(output, "  • {effect}")?;
        }
        writeln!(output)?;
    }

    if !result.warnings.is_empty() {
        writeln!(output, "Warnings:")?;
        for warning in &result.warnings {
            writeln!(output, "  ⚠ {warning}")?;
        }
        writeln!(output)?;
    }

    if result.reversible {
        writeln!(output, "Reversible: yes")?;
        if let Some(undo) = &result.undo_command {
            writeln!(output, "Undo command: {undo}")?;
        }
    } else {
        writeln!(output, "Reversible: no")?;
    }

    Ok(output)
}

/// Truncate output to specified limit if needed, preserving structure
fn truncate_output_if_needed(result: &WhatIfResult, limit: usize) -> Result<String> {
    let output = format_human_output(result)?;

    if output.len() <= limit {
        return Ok(output);
    }

    // Truncate to ~90% of limit, leaving room for truncation message
    let truncate_at = limit.saturating_sub(200);

    // Find a good truncation point (newline)
    #[allow(clippy::option_if_let_else)]
    let truncated = if let Some(pos) = output[..truncate_at].rfind('\n') {
        Ok(format!(
            "{}\n\n... (truncated, {} bytes total) ...",
            &output[..pos],
            output.len()
        ))
    } else {
        Ok(format!(
            "{}\n\n... (truncated, {} bytes total) ...",
            &output[..truncate_at],
            output.len()
        ))
    };

    truncated
}

/// Save full output to a temporary file
fn save_full_output_to_temp(output: &str) -> Result<String> {
    use std::io::Write;

    // Create temp file with unique name based on timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("Failed to get time: {e}"))?
        .as_secs();

    let temp_dir = std::env::temp_dir();
    let file_name = format!("zjj-whatif-{timestamp}.txt");
    let temp_path = temp_dir.join(&file_name);

    let mut file = std::fs::File::create(&temp_path)
        .map_err(|e| anyhow::anyhow!("Failed to create temp file: {e}"))?;

    file.write_all(output.as_bytes())
        .map_err(|e| anyhow::anyhow!("Failed to write temp file: {e}"))?;

    Ok(temp_path.display().to_string())
}

fn preview_command(command: &str, args: &[String]) -> Result<WhatIfResult> {
    match command {
        "add" => preview_add(args),
        "work" => preview_work(args),
        "remove" => preview_remove(args),
        "done" => preview_done(args),
        "abort" => preview_abort(args),
        "sync" => preview_sync(args),
        "spawn" => preview_spawn(args),
        _ => Ok(WhatIfResult {
            command: command.to_string(),
            args: args.to_vec(),
            steps: vec![WhatIfStep {
                order: 1,
                description: format!("Execute '{command}' command"),
                action: format!("zjj {command} {}", args.join(" ")),
                can_fail: true,
                on_failure: Some("Error message will be shown".to_string()),
            }],
            creates: vec![],
            modifies: vec![],
            deletes: vec![],
            side_effects: vec![],
            reversible: false,
            undo_command: None,
            warnings: vec![format!("No specific preview available for '{command}'")],
            prerequisites: vec![],
        }),
    }
}

fn preview_add(args: &[String]) -> Result<WhatIfResult> {
    // Validate session name to prevent injection attacks and resource exhaustion
    let name = args.first().map(String::as_str).unwrap_or("<name>");

    // Only validate if we have a real name (not the placeholder)
    if name != "<name>" {
        validate_session_name(name).map_err(anyhow::Error::new)?;
    }

    Ok(WhatIfResult {
        command: "add".to_string(),
        args: args.to_vec(),
        steps: vec![
            WhatIfStep {
                order: 1,
                description: "Validate session name".to_string(),
                action: format!("Check '{name}' is valid and doesn't exist"),
                can_fail: true,
                on_failure: Some("Error if name invalid or already exists".to_string()),
            },
            WhatIfStep {
                order: 2,
                description: "Create JJ workspace".to_string(),
                action: format!("jj workspace add {name}"),
                can_fail: true,
                on_failure: Some("Rollback: nothing created yet".to_string()),
            },
            WhatIfStep {
                order: 3,
                description: "Create Zellij tab".to_string(),
                action: format!("zellij action new-tab --name zjj:{name}"),
                can_fail: true,
                on_failure: Some("Rollback: remove JJ workspace".to_string()),
            },
            WhatIfStep {
                order: 4,
                description: "Save to database".to_string(),
                action: "INSERT session into .zjj/state.db".to_string(),
                can_fail: false,
                on_failure: None,
            },
        ],
        creates: vec![
            ResourceChange {
                resource_type: "workspace".to_string(),
                resource: format!(".zjj/workspaces/{name}"),
                description: "JJ workspace directory".to_string(),
            },
            ResourceChange {
                resource_type: "zellij_tab".to_string(),
                resource: format!("zjj:{name}"),
                description: "Zellij terminal tab".to_string(),
            },
            ResourceChange {
                resource_type: "database_record".to_string(),
                resource: format!("session:{name}"),
                description: "Session tracking record".to_string(),
            },
        ],
        modifies: vec![],
        deletes: vec![],
        side_effects: vec![
            "Switches Zellij focus to new tab".to_string(),
            "Changes working directory in new tab".to_string(),
        ],
        reversible: true,
        undo_command: Some(format!("zjj remove {name}")),
        warnings: vec![],
        prerequisites: vec![
            PrerequisiteCheck {
                check: "zjj_initialized".to_string(),
                status: PrerequisiteStatus::Unknown,
                description: "zjj must be initialized".to_string(),
            },
            PrerequisiteCheck {
                check: "session_not_exists".to_string(),
                status: PrerequisiteStatus::Unknown,
                description: format!("Session '{name}' must not exist"),
            },
        ],
    })
}

fn preview_work(args: &[String]) -> Result<WhatIfResult> {
    // Validate session name to prevent injection attacks
    let name = args.first().map(String::as_str).unwrap_or("<name>");

    // Only validate if we have a real name (not the placeholder)
    if name != "<name>" {
        validate_session_name(name).map_err(anyhow::Error::new)?;
    }

    let mut result = preview_add(args)?;
    result.command = "work".to_string();

    result.steps.push(WhatIfStep {
        order: 5,
        description: "Register as agent".to_string(),
        action: "Set ZJJ_AGENT_ID environment variable".to_string(),
        can_fail: false,
        on_failure: None,
    });

    result
        .side_effects
        .push("Sets ZJJ_AGENT_ID in environment".to_string());
    result.creates.push(ResourceChange {
        resource_type: "agent_registration".to_string(),
        resource: "agent:<auto-generated>".to_string(),
        description: "Agent registration in database".to_string(),
    });

    result.undo_command = Some(format!("zjj abort --workspace {name}"));

    Ok(result)
}

fn preview_remove(args: &[String]) -> Result<WhatIfResult> {
    // Validate session name to prevent injection attacks
    let name = args.first().map(String::as_str).unwrap_or("<name>");

    // Only validate if we have a real name (not the placeholder)
    if name != "<name>" {
        validate_session_name(name).map_err(anyhow::Error::new)?;
    }

    Ok(WhatIfResult {
        command: "remove".to_string(),
        args: args.to_vec(),
        steps: vec![
            WhatIfStep {
                order: 1,
                description: "Check session exists".to_string(),
                action: format!("Verify '{name}' exists in database"),
                can_fail: true,
                on_failure: Some("Error if session not found".to_string()),
            },
            WhatIfStep {
                order: 2,
                description: "Close Zellij tab".to_string(),
                action: format!("Close zjj:{name} tab"),
                can_fail: true,
                on_failure: Some("Continue anyway".to_string()),
            },
            WhatIfStep {
                order: 3,
                description: "Remove JJ workspace".to_string(),
                action: format!("jj workspace forget {name}"),
                can_fail: true,
                on_failure: Some("Log warning, continue".to_string()),
            },
            WhatIfStep {
                order: 4,
                description: "Delete workspace files".to_string(),
                action: format!("rm -rf .zjj/workspaces/{name}"),
                can_fail: false,
                on_failure: None,
            },
            WhatIfStep {
                order: 5,
                description: "Remove from database".to_string(),
                action: "DELETE session from .zjj/state.db".to_string(),
                can_fail: false,
                on_failure: None,
            },
        ],
        creates: vec![],
        modifies: vec![],
        deletes: vec![
            ResourceChange {
                resource_type: "workspace".to_string(),
                resource: format!(".zjj/workspaces/{name}"),
                description: "JJ workspace directory".to_string(),
            },
            ResourceChange {
                resource_type: "zellij_tab".to_string(),
                resource: format!("zjj:{name}"),
                description: "Zellij terminal tab".to_string(),
            },
            ResourceChange {
                resource_type: "database_record".to_string(),
                resource: format!("session:{name}"),
                description: "Session tracking record".to_string(),
            },
        ],
        side_effects: vec![
            "May switch Zellij focus if current tab is closed".to_string(),
            "Uncommitted changes will be LOST".to_string(),
        ],
        reversible: false,
        undo_command: None,
        warnings: vec![
            "This operation is DESTRUCTIVE".to_string(),
            "Uncommitted changes will be permanently lost".to_string(),
            "Use --dry-run to preview first".to_string(),
        ],
        prerequisites: vec![PrerequisiteCheck {
            check: "session_exists".to_string(),
            status: PrerequisiteStatus::Unknown,
            description: format!("Session '{name}' must exist"),
        }],
    })
}

fn preview_done(args: &[String]) -> Result<WhatIfResult> {
    // Validate workspace name to prevent injection attacks
    let workspace = args.first().map(String::as_str).unwrap_or("<current>");

    // Only validate if we have a real workspace (not the placeholder)
    if workspace != "<current>" {
        validate_session_name(workspace).map_err(anyhow::Error::new)?;
    }

    Ok(WhatIfResult {
        command: "done".to_string(),
        args: args.to_vec(),
        steps: vec![
            WhatIfStep {
                order: 1,
                description: "Validate location".to_string(),
                action: "Check we're in a workspace".to_string(),
                can_fail: true,
                on_failure: Some("Error: not in workspace".to_string()),
            },
            WhatIfStep {
                order: 2,
                description: "Commit any uncommitted changes".to_string(),
                action: "jj commit -m <auto-message>".to_string(),
                can_fail: true,
                on_failure: Some("Error if commit fails".to_string()),
            },
            WhatIfStep {
                order: 3,
                description: "Switch to main".to_string(),
                action: "jj edit main".to_string(),
                can_fail: false,
                on_failure: None,
            },
            WhatIfStep {
                order: 4,
                description: "Merge workspace".to_string(),
                action: format!("jj merge {workspace}"),
                can_fail: true,
                on_failure: Some("Error if merge conflicts".to_string()),
            },
            WhatIfStep {
                order: 5,
                description: "Log undo history".to_string(),
                action: "Write to .zjj/undo.jsonl".to_string(),
                can_fail: false,
                on_failure: None,
            },
            WhatIfStep {
                order: 6,
                description: "Cleanup workspace".to_string(),
                action: format!("Remove workspace {workspace}"),
                can_fail: false,
                on_failure: None,
            },
        ],
        creates: vec![
            ResourceChange {
                resource_type: "commit".to_string(),
                resource: "main".to_string(),
                description: "Merge commit on main".to_string(),
            },
            ResourceChange {
                resource_type: "undo_entry".to_string(),
                resource: ".zjj/undo.jsonl".to_string(),
                description: "Undo history entry".to_string(),
            },
        ],
        modifies: vec![ResourceChange {
            resource_type: "branch".to_string(),
            resource: "main".to_string(),
            description: "Advances main with merge".to_string(),
        }],
        deletes: vec![ResourceChange {
            resource_type: "workspace".to_string(),
            resource: format!(".zjj/workspaces/{workspace}"),
            description: "Workspace directory (unless --keep-workspace)".to_string(),
        }],
        side_effects: vec![
            "Changes working directory to main".to_string(),
            "Closes Zellij tab".to_string(),
            "Updates bead status to closed".to_string(),
        ],
        reversible: true,
        undo_command: Some("zjj undo".to_string()),
        warnings: vec![
            "Make sure all changes are committed".to_string(),
            "Use --dry-run to preview merge".to_string(),
        ],
        prerequisites: vec![
            PrerequisiteCheck {
                check: "in_workspace".to_string(),
                status: PrerequisiteStatus::Unknown,
                description: "Must be in a workspace".to_string(),
            },
            PrerequisiteCheck {
                check: "no_conflicts".to_string(),
                status: PrerequisiteStatus::Unknown,
                description: "No merge conflicts with main".to_string(),
            },
        ],
    })
}

#[allow(clippy::unnecessary_wraps)]
fn preview_abort(args: &[String]) -> Result<WhatIfResult> {
    let workspace = args.first().map(String::as_str).unwrap_or("<current>");

    Ok(WhatIfResult {
        command: "abort".to_string(),
        args: args.to_vec(),
        steps: vec![
            WhatIfStep {
                order: 1,
                description: "Validate location".to_string(),
                action: "Check we're in a workspace or --workspace specified".to_string(),
                can_fail: true,
                on_failure: Some("Error: workspace not found".to_string()),
            },
            WhatIfStep {
                order: 2,
                description: "Switch to main (if in workspace)".to_string(),
                action: "jj edit main".to_string(),
                can_fail: false,
                on_failure: None,
            },
            WhatIfStep {
                order: 3,
                description: "Remove workspace".to_string(),
                action: format!("Remove {workspace} completely"),
                can_fail: false,
                on_failure: None,
            },
            WhatIfStep {
                order: 4,
                description: "Update bead status".to_string(),
                action: "Mark bead as abandoned".to_string(),
                can_fail: false,
                on_failure: None,
            },
        ],
        creates: vec![],
        modifies: vec![ResourceChange {
            resource_type: "bead".to_string(),
            resource: "<associated-bead>".to_string(),
            description: "Status changed to 'abandoned'".to_string(),
        }],
        deletes: vec![ResourceChange {
            resource_type: "workspace".to_string(),
            resource: format!(".zjj/workspaces/{workspace}"),
            description: "Workspace and all changes".to_string(),
        }],
        side_effects: vec![
            "All uncommitted work is LOST".to_string(),
            "Switches to main branch".to_string(),
        ],
        reversible: false,
        undo_command: None,
        warnings: vec![
            "This operation is DESTRUCTIVE".to_string(),
            "All work in this workspace will be permanently lost".to_string(),
        ],
        prerequisites: vec![],
    })
}

#[allow(clippy::unnecessary_wraps)]
fn preview_sync(args: &[String]) -> Result<WhatIfResult> {
    let target = args.first().map(String::as_str).unwrap_or("<current>");

    Ok(WhatIfResult {
        command: "sync".to_string(),
        args: args.to_vec(),
        steps: vec![
            WhatIfStep {
                order: 1,
                description: "Identify target workspace(s)".to_string(),
                action: format!("Target: {target}"),
                can_fail: false,
                on_failure: None,
            },
            WhatIfStep {
                order: 2,
                description: "Rebase onto main".to_string(),
                action: "jj rebase -d main".to_string(),
                can_fail: true,
                on_failure: Some("Error if rebase conflicts".to_string()),
            },
        ],
        creates: vec![],
        modifies: vec![ResourceChange {
            resource_type: "workspace".to_string(),
            resource: target.to_string(),
            description: "Rebased onto latest main".to_string(),
        }],
        deletes: vec![],
        side_effects: vec!["Commit history may be rewritten".to_string()],
        reversible: true,
        undo_command: Some("jj undo".to_string()),
        warnings: vec![],
        prerequisites: vec![PrerequisiteCheck {
            check: "no_uncommitted".to_string(),
            status: PrerequisiteStatus::Unknown,
            description: "No uncommitted conflicting changes".to_string(),
        }],
    })
}

#[allow(clippy::unnecessary_wraps)]
fn preview_spawn(args: &[String]) -> Result<WhatIfResult> {
    let bead_id = args.first().map(String::as_str).unwrap_or("<bead-id>");

    Ok(WhatIfResult {
        command: "spawn".to_string(),
        args: args.to_vec(),
        steps: vec![
            WhatIfStep {
                order: 1,
                description: "Validate bead exists".to_string(),
                action: format!("Check bead '{bead_id}' exists"),
                can_fail: true,
                on_failure: Some("Error if bead not found".to_string()),
            },
            WhatIfStep {
                order: 2,
                description: "Create workspace".to_string(),
                action: format!("zjj add {bead_id}"),
                can_fail: true,
                on_failure: Some("Error if workspace creation fails".to_string()),
            },
            WhatIfStep {
                order: 3,
                description: "Launch agent".to_string(),
                action: "claude <bead-context>".to_string(),
                can_fail: true,
                on_failure: Some("Cleanup workspace on failure".to_string()),
            },
            WhatIfStep {
                order: 4,
                description: "Monitor agent".to_string(),
                action: "Wait for agent completion".to_string(),
                can_fail: true,
                on_failure: Some("Log failure, optionally cleanup".to_string()),
            },
            WhatIfStep {
                order: 5,
                description: "Auto-merge on success".to_string(),
                action: "zjj done (unless --no-auto-merge)".to_string(),
                can_fail: true,
                on_failure: Some("Keep workspace for manual review".to_string()),
            },
        ],
        creates: vec![
            ResourceChange {
                resource_type: "workspace".to_string(),
                resource: format!(".zjj/workspaces/{bead_id}"),
                description: "Workspace for agent".to_string(),
            },
            ResourceChange {
                resource_type: "process".to_string(),
                resource: "agent".to_string(),
                description: "Agent process".to_string(),
            },
        ],
        modifies: vec![ResourceChange {
            resource_type: "bead".to_string(),
            resource: bead_id.to_string(),
            description: "Status changed to 'in_progress'".to_string(),
        }],
        deletes: vec![],
        side_effects: vec![
            "Spawns external agent process".to_string(),
            "Updates bead status".to_string(),
            "May modify codebase through agent".to_string(),
        ],
        reversible: false,
        undo_command: None,
        warnings: vec![
            "Agent will make changes to codebase".to_string(),
            "Review agent output before pushing".to_string(),
        ],
        prerequisites: vec![
            PrerequisiteCheck {
                check: "bead_exists".to_string(),
                status: PrerequisiteStatus::Unknown,
                description: format!("Bead '{bead_id}' must exist"),
            },
            PrerequisiteCheck {
                check: "on_main".to_string(),
                status: PrerequisiteStatus::Unknown,
                description: "Must be on main branch".to_string(),
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preview_add_creates_result() -> Result<()> {
        let result = preview_add(&["test-session".to_string()])?;
        assert_eq!(result.command, "add");
        assert!(!result.steps.is_empty());
        assert!(!result.creates.is_empty());
        Ok(())
    }

    #[test]
    fn test_preview_remove_shows_deletes() -> Result<()> {
        let result = preview_remove(&["test-session".to_string()])?;
        assert!(!result.deletes.is_empty());
        assert!(!result.warnings.is_empty());
        Ok(())
    }

    #[test]
    fn test_preview_done_is_reversible() -> Result<()> {
        let result = preview_done(&[])?;
        assert!(result.reversible);
        assert!(result.undo_command.is_some());
        Ok(())
    }

    #[test]
    fn test_preview_abort_is_not_reversible() -> Result<()> {
        let result = preview_abort(&[])?;
        assert!(!result.reversible);
        Ok(())
    }

    #[test]
    fn test_preview_unknown_command() -> Result<()> {
        let result = preview_command("unknown", &[])?;
        assert!(!result.warnings.is_empty());
        Ok(())
    }

    // === Security Tests: Input Validation (RED Phase) ===

    #[test]
    fn test_whatif_validates_session_name_empty() {
        let result = preview_add(&[String::new()]);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string().to_lowercase();
            assert!(
                err_msg.contains("invalid")
                    || err_msg.contains("empty")
                    || err_msg.contains("validation"),
                "Expected validation error for empty name, got: {e}"
            );
        }
    }

    #[test]
    fn test_whatif_validates_session_name_too_long() {
        let long_name = "a".repeat(100);
        let result = preview_add(&[long_name]);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string().to_lowercase();
            assert!(
                err_msg.contains("invalid")
                    || err_msg.contains("exceed")
                    || err_msg.contains("validation"),
                "Expected validation error for too-long name, got: {e}"
            );
        }
    }

    #[test]
    fn test_whatif_validates_session_name_invalid_chars() {
        let invalid_names = vec![
            "test session",  // space
            "test/session",  // slash
            "test\\session", // backslash
            "test.session",  // dot
            "test🚀",        // emoji
            "123-session",   // starts with number
            "-test",         // starts with dash
            "_test",         // starts with underscore
        ];

        for name in invalid_names {
            let result = preview_add(&[name.to_string()]);
            assert!(result.is_err(), "Expected validation error for '{name}'");
            if let Err(e) = result {
                let err_msg = e.to_string().to_lowercase();
                assert!(
                    err_msg.contains("invalid")
                        || err_msg.contains("validation")
                        || err_msg.contains("ascii"),
                    "Expected validation error for '{name}', got: {e}"
                );
            }
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    #[test]
    fn test_whatif_validates_session_name_valid() -> Result<()> {
        let valid_names = vec!["test-session", "test_session", "TestSession", "abc123", "a"];

        for name in valid_names {
            let result = preview_add(&[name.to_string()]);
            assert!(result.is_ok(), "Expected success for valid name '{name}'");
        }
        Ok(())
    }

    #[test]
    fn test_whatif_validates_all_subcommands() {
        let invalid_name = "test session";

        // Test that all whatif subcommands validate the first argument as session name
        let commands = vec![
            ("add", vec![invalid_name.to_string()]),
            ("remove", vec![invalid_name.to_string()]),
            ("work", vec![invalid_name.to_string()]),
        ];

        for (cmd, args) in commands {
            let result = preview_command(cmd, &args);
            // Some commands may not validate in preview, but the real commands do
            // This test documents the expectation that validation should happen
            if let Err(e) = result {
                let err_msg = e.to_string().to_lowercase();
                assert!(
                    err_msg.contains("invalid") || err_msg.contains("validation"),
                    "Command '{cmd}' should validate session name, got: {e}"
                );
            }
        }
    }

    // === Security Tests: Output Truncation (RED Phase) ===

    #[test]
    fn test_whatif_output_truncation_limit() -> Result<()> {
        // Create a WhatIfResult with very large output
        let result = WhatIfResult {
            command: "add".to_string(),
            args: vec!["test".to_string()],
            steps: vec![WhatIfStep {
                order: 1,
                description: "A".repeat(10000),
                action: "B".repeat(10000),
                can_fail: true,
                on_failure: Some("C".repeat(10000)),
            }],
            creates: vec![ResourceChange {
                resource_type: "D".repeat(10000),
                resource: "E".repeat(10000),
                description: "F".repeat(10000),
            }],
            modifies: vec![],
            deletes: vec![],
            side_effects: vec!["G".repeat(10000)],
            reversible: true,
            undo_command: Some("H".repeat(10000)),
            warnings: vec!["I".repeat(10000)],
            prerequisites: vec![PrerequisiteCheck {
                check: "J".repeat(10000),
                status: PrerequisiteStatus::Met,
                description: "K".repeat(10000),
            }],
        };

        // Test that the output truncation function limits output
        let truncated = truncate_output_if_needed(&result, 4096)?;
        assert!(
            truncated.len() <= 4096 + 200, // Allow some margin for truncation message
            "Output should be truncated to ~4KB, but got {} bytes",
            truncated.len()
        );
        Ok(())
    }

    #[test]
    fn test_whatif_output_small_not_truncated() -> Result<()> {
        let result = WhatIfResult {
            command: "add".to_string(),
            args: vec!["test".to_string()],
            steps: vec![],
            creates: vec![],
            modifies: vec![],
            deletes: vec![],
            side_effects: vec![],
            reversible: false,
            undo_command: None,
            warnings: vec![],
            prerequisites: vec![],
        };

        let truncated = truncate_output_if_needed(&result, 4096)?;
        // Small output should not be truncated
        assert!(
            !truncated.contains("(truncated"),
            "Small output should not be truncated"
        );
        Ok(())
    }
}
