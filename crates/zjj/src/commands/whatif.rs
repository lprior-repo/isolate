//! `WhatIf` command - Preview what a command would do
//!
//! Provides detailed preview of command effects without execution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zjj_core::{OutputFormat, SchemaEnvelope};

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
        println!("What if: {} {}", result.command, result.args.join(" "));
        println!();

        if !result.prerequisites.is_empty() {
            println!("Prerequisites:");
            result.prerequisites.iter().for_each(|prereq| {
                let status = match prereq.status {
                    PrerequisiteStatus::Met => "✓",
                    PrerequisiteStatus::NotMet => "✗",
                    PrerequisiteStatus::Unknown => "?",
                };
                println!("  {status} {}: {}", prereq.check, prereq.description);
            });
            println!();
        }

        println!("Execution plan:");
        result.steps.iter().for_each(|step| {
            println!("  {}. {}", step.order, step.description);
            println!("     Action: {}", step.action);
            if step.can_fail {
                if let Some(on_fail) = &step.on_failure {
                    println!("     On failure: {on_fail}");
                }
            }
        });
        println!();

        if !result.creates.is_empty() {
            println!("Would create:");
            result.creates.iter().for_each(|c| {
                println!(
                    "  + [{}] {}: {}",
                    c.resource_type, c.resource, c.description
                );
            });
            println!();
        }

        if !result.modifies.is_empty() {
            println!("Would modify:");
            result.modifies.iter().for_each(|m| {
                println!(
                    "  ~ [{}] {}: {}",
                    m.resource_type, m.resource, m.description
                );
            });
            println!();
        }

        if !result.deletes.is_empty() {
            println!("Would delete:");
            result.deletes.iter().for_each(|d| {
                println!(
                    "  - [{}] {}: {}",
                    d.resource_type, d.resource, d.description
                );
            });
            println!();
        }

        if !result.side_effects.is_empty() {
            println!("Side effects:");
            result.side_effects.iter().for_each(|effect| {
                println!("  • {effect}");
            });
            println!();
        }

        if !result.warnings.is_empty() {
            println!("Warnings:");
            result.warnings.iter().for_each(|warning| {
                println!("  ⚠ {warning}");
            });
            println!();
        }

        if result.reversible {
            println!("Reversible: yes");
            if let Some(undo) = &result.undo_command {
                println!("Undo command: {undo}");
            }
        } else {
            println!("Reversible: no");
        }
    }

    Ok(())
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

#[allow(clippy::unnecessary_wraps)]
fn preview_add(args: &[String]) -> Result<WhatIfResult> {
    let name = args.first().map(String::as_str).unwrap_or("<name>");

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
    let name = args.first().map(String::as_str).unwrap_or("<name>");

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

#[allow(clippy::unnecessary_wraps)]
fn preview_remove(args: &[String]) -> Result<WhatIfResult> {
    let name = args.first().map(String::as_str).unwrap_or("<name>");

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

#[allow(clippy::unnecessary_wraps)]
fn preview_done(args: &[String]) -> Result<WhatIfResult> {
    let workspace = args.first().map(String::as_str).unwrap_or("<current>");

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
}
