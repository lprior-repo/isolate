//! Integrity command - workspace corruption detection, validation, and repair
//!
//! This command manages workspace integrity, including:
//! - Validation: Check workspaces for corruption
//! - Repair: Auto-fix detected issues with backup protection
//! - Backup: Manage workspace backups for recovery
//! - Restore: Rollback to previous backups

use anyhow::Result;
use serde::Serialize;
use zjj_core::{
    workspace_integrity::{
        BackupManager, BackupMetadata, IntegrityValidator, RepairExecutor, RepairStrategy,
        ValidationResult,
    },
    OutputFormat, SchemaEnvelope,
};

use crate::commands::check_prerequisites;

/// Options for the integrity command
#[derive(Debug, Clone)]
pub struct IntegrityOptions {
    /// Subcommand to run
    pub subcommand: IntegritySubcommand,
    /// Output format
    pub format: OutputFormat,
}

/// Integrity subcommands
#[derive(Debug, Clone)]
pub enum IntegritySubcommand {
    /// Validate workspace integrity
    Validate {
        /// Workspace name or path
        workspace: String,
    },
    /// Repair corrupted workspace
    Repair {
        /// Workspace name or path
        workspace: String,
        /// Force repair without confirmation
        force: bool,
    },
    /// List available backups
    BackupList,
    /// Restore from backup
    BackupRestore {
        /// Backup ID to restore
        backup_id: String,
        /// Force restore without confirmation
        force: bool,
    },
}

/// Validation response
#[derive(Debug, Clone, Serialize)]
struct ValidationResponse {
    /// Workspace name
    workspace: String,
    /// Absolute path to workspace
    path: String,
    /// Whether workspace is valid
    is_valid: bool,
    /// Number of issues detected
    issue_count: usize,
    /// Detailed validation result
    validation: ValidationResult,
}

/// Repair response
#[derive(Debug, Clone, Serialize)]
struct RepairResponse {
    /// Workspace name
    workspace: String,
    /// Whether repair was successful
    success: bool,
    /// Repair summary message
    summary: String,
}

/// Backup list response
#[derive(Debug, Clone, Serialize)]
struct BackupListResponse {
    /// List of backups
    backups: Vec<BackupMetadata>,
    /// Total count
    count: usize,
}

/// Restore response
#[derive(Debug, Clone, Serialize)]
struct RestoreResponse {
    /// Workspace name
    workspace: String,
    /// Backup ID that was restored
    backup_id: String,
    /// Whether restore was successful
    success: bool,
    /// Restore summary message
    summary: String,
}

/// Run the integrity command
pub fn run(options: &IntegrityOptions) -> Result<()> {
    // Ensure we're in a JJ repository
    let jj_root = check_prerequisites()?;

    match &options.subcommand {
        IntegritySubcommand::Validate { workspace } => {
            run_validate(&jj_root, workspace, options.format)
        }
        IntegritySubcommand::Repair { workspace, force } => {
            run_repair(&jj_root, workspace, *force, options.format)
        }
        IntegritySubcommand::BackupList => run_backup_list(&jj_root, options.format),
        IntegritySubcommand::BackupRestore { backup_id, force } => {
            run_backup_restore(&jj_root, backup_id, *force, options.format)
        }
    }
}

/// Validate a workspace
fn run_validate(jj_root: &std::path::Path, workspace: &str, format: OutputFormat) -> Result<()> {
    let validator = IntegrityValidator::new(jj_root);
    let result = validator.validate(workspace)?;

    let response = ValidationResponse {
        workspace: workspace.to_string(),
        path: result.path.to_string_lossy().to_string(),
        is_valid: result.is_valid,
        issue_count: result.issues.len(),
        validation: result,
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("integrity-validate-response", "single", response);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        print_validation_result(&response);
    }

    Ok(())
}

/// Repair a workspace
fn run_repair(
    jj_root: &std::path::Path,
    workspace: &str,
    force: bool,
    format: OutputFormat,
) -> Result<()> {
    let validator = IntegrityValidator::new(jj_root);
    let validation = validator.validate(workspace)?;

    if validation.is_valid {
        let response = RepairResponse {
            workspace: workspace.to_string(),
            success: true,
            summary: "Workspace is already valid, no repair needed".to_string(),
        };

        if format.is_json() {
            let envelope = SchemaEnvelope::new("integrity-repair-response", "single", response);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else {
            println!("Workspace '{workspace}' is valid - no repair needed");
        }

        return Ok(());
    }

    // Check if issues can be auto-repaired
    if !validation.has_auto_repairable_issues() {
        let response = RepairResponse {
            workspace: workspace.to_string(),
            success: false,
            summary:
                "Workspace has issues that cannot be auto-repaired. Manual intervention required."
                    .to_string(),
        };

        if format.is_json() {
            let envelope = SchemaEnvelope::new("integrity-repair-response", "single", response);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else {
            println!("Workspace has non-repairable issues:");
            for issue in &validation.issues {
                println!("  - {}: {}", issue.corruption_type, issue.description);
            }
            println!("\nManual intervention required.");
        }

        return Ok(());
    }

    // Ask for confirmation unless force is set
    if !force && !confirm_repair(&validation) {
        if format.is_json() {
            let response = RepairResponse {
                workspace: workspace.to_string(),
                success: false,
                summary: "Repair cancelled by user".to_string(),
            };
            let envelope = SchemaEnvelope::new("integrity-repair-response", "single", response);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else {
            println!("Repair cancelled");
        }
        return Ok(());
    }

    // Perform repair
    let backup_manager = BackupManager::new(jj_root);
    let executor = RepairExecutor::new(backup_manager);

    // Get the workspace path
    let workspace_path = jj_root.join(workspace);

    // Get the most severe issue to determine the repair strategy
    let strategy = validation
        .most_severe_issue()
        .map(|i| i.recommended_strategy)
        .unwrap_or(RepairStrategy::NoRepairPossible);

    match executor.execute(workspace, &workspace_path, &validation, strategy) {
        Ok(repair_result) => {
            let response = RepairResponse {
                workspace: workspace.to_string(),
                success: repair_result.success,
                summary: repair_result.summary.clone(),
            };

            if format.is_json() {
                let envelope = SchemaEnvelope::new("integrity-repair-response", "single", response);
                println!("{}", serde_json::to_string_pretty(&envelope)?);
            } else if repair_result.success {
                println!("Successfully repaired workspace '{workspace}'");
                println!("Summary: {}", repair_result.summary);
            } else {
                println!("Repair failed: {}", repair_result.summary);
            }

            Ok(())
        }
        Err(e) => {
            let response = RepairResponse {
                workspace: workspace.to_string(),
                success: false,
                summary: format!("Repair failed: {e}"),
            };

            if format.is_json() {
                let envelope = SchemaEnvelope::new("integrity-repair-response", "single", response);
                println!("{}", serde_json::to_string_pretty(&envelope)?);
            } else {
                eprintln!("Repair failed: {e}");
            }

            Ok(())
        }
    }
}

/// List available backups
fn run_backup_list(jj_root: &std::path::Path, format: OutputFormat) -> Result<()> {
    let manager = BackupManager::new(jj_root);

    // Collect all backups from all workspaces
    let mut all_backups = Vec::new();

    // Get all workspace directories
    if let Ok(entries) = std::fs::read_dir(jj_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(workspace_name) = path.file_name() {
                    if let Some(name_str) = workspace_name.to_str() {
                        // Skip special directories
                        if !name_str.starts_with('.') && !name_str.starts_with('_') {
                            if let Ok(backups) = manager.list_backups(name_str) {
                                all_backups.extend(backups);
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort by creation time (newest first)
    all_backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let response = BackupListResponse {
        count: all_backups.len(),
        backups: all_backups,
    };

    if format.is_json() {
        let envelope = SchemaEnvelope::new("integrity-backup-list-response", "single", response);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        print_backup_list(&response);
    }

    Ok(())
}

/// Restore from a backup
fn run_backup_restore(
    jj_root: &std::path::Path,
    backup_id: &str,
    force: bool,
    format: OutputFormat,
) -> Result<()> {
    let manager = BackupManager::new(jj_root);

    // Find the backup across all workspaces
    let mut backup_found = None;

    if let Ok(entries) = std::fs::read_dir(jj_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(workspace_name) = path.file_name() {
                    if let Some(name_str) = workspace_name.to_str() {
                        // Skip special directories
                        if !name_str.starts_with('.') && !name_str.starts_with('_') {
                            if let Ok(backups) = manager.list_backups(name_str) {
                                if let Some(backup) = backups.iter().find(|b| b.id == backup_id) {
                                    backup_found = Some((name_str.to_string(), backup.clone()));
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let (workspace_name, backup) =
        backup_found.ok_or_else(|| anyhow::anyhow!("Backup '{backup_id}' not found"))?;

    // Ask for confirmation unless force is set
    if !force && !confirm_restore(&backup) {
        if format.is_json() {
            let response = RestoreResponse {
                workspace: workspace_name,
                backup_id: backup_id.to_string(),
                success: false,
                summary: "Restore cancelled by user".to_string(),
            };
            let envelope = SchemaEnvelope::new("integrity-restore-response", "single", response);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else {
            println!("Restore cancelled");
        }
        return Ok(());
    }

    // Get the workspace path
    let workspace_path = jj_root.join(&workspace_name);

    // Perform restore
    match manager.restore_backup(backup_id, &workspace_name, &workspace_path) {
        Ok(rollback_result) => {
            let summary = rollback_result.summary.clone();
            let response = RestoreResponse {
                workspace: workspace_name.clone(),
                backup_id: backup_id.to_string(),
                success: rollback_result.success,
                summary: rollback_result.summary,
            };

            if format.is_json() {
                let envelope =
                    SchemaEnvelope::new("integrity-restore-response", "single", response);
                println!("{}", serde_json::to_string_pretty(&envelope)?);
            } else if rollback_result.success {
                println!("Successfully restored from backup '{backup_id}'");
                println!("Summary: {summary}");
            } else {
                println!("Restore failed: {summary}");
            }

            Ok(())
        }
        Err(e) => {
            let response = RestoreResponse {
                workspace: workspace_name.clone(),
                backup_id: backup_id.to_string(),
                success: false,
                summary: format!("Restore failed: {e}"),
            };

            if format.is_json() {
                let envelope =
                    SchemaEnvelope::new("integrity-restore-response", "single", response);
                println!("{}", serde_json::to_string_pretty(&envelope)?);
            } else {
                eprintln!("Restore failed: {e}");
            }

            Ok(())
        }
    }
}

/// Print human-readable validation result
fn print_validation_result(response: &ValidationResponse) {
    println!("Workspace Validation: {}", response.workspace);
    println!("Path: {}", response.path);
    println!();

    if response.is_valid {
        println!("Status: VALID");
    } else {
        println!("Status: INVALID");
        println!("Issues: {}", response.issue_count);
        println!();

        for issue in &response.validation.issues {
            println!("  Issue: {}", issue.corruption_type);
            println!("    Description: {}", issue.description);
            if let Some(ctx) = &issue.context {
                println!("    Context: {ctx}");
            }
            println!(
                "    Recommended Action: {}",
                issue.recommended_strategy.description()
            );
            println!();
        }
    }

    println!(
        "Validated at: {}",
        response.validation.validated_at.to_rfc3339()
    );
    println!("Duration: {}ms", response.validation.duration_ms);
}

/// Print human-readable backup list
fn print_backup_list(response: &BackupListResponse) {
    println!("Available Backups: {}", response.count);
    println!();

    if response.backups.is_empty() {
        println!("No backups found.");
        return;
    }

    for backup in &response.backups {
        println!("ID: {}", backup.id);
        println!("  Workspace: {}", backup.workspace);
        println!("  Created: {}", backup.created_at.to_rfc3339());
        println!("  Size: {} bytes", backup.size_bytes);
        println!("  Reason: {:?}", backup.reason);
        if let Some(checksum) = &backup.checksum {
            println!("  Checksum: {checksum}");
        }
        println!();
    }
}

/// Ask user to confirm repair
fn confirm_repair(validation: &ValidationResult) -> bool {
    use std::io::{self, Write};

    println!(
        "Workspace has {} integrity issue(s):",
        validation.issues.len()
    );
    for issue in &validation.issues {
        println!("  - {}: {}", issue.corruption_type, issue.description);
    }
    println!();
    println!(
        "Recommended action: {}",
        validation
            .most_severe_issue()
            .map(|i| i.recommended_strategy.description())
            .unwrap_or("No issues")
    );
    println!();

    print!("Continue with repair? [y/N] ");
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .ok()
        .map(|_| input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes"))
        .unwrap_or(false)
}

/// Ask user to confirm restore
fn confirm_restore(backup: &BackupMetadata) -> bool {
    use std::io::{self, Write};

    println!("About to restore from backup:");
    println!("  ID: {}", backup.id);
    println!("  Workspace: {}", backup.workspace);
    println!("  Created: {}", backup.created_at.to_rfc3339());
    println!("  Size: {} bytes", backup.size_bytes);
    println!();
    println!("This will restore the workspace to the state at the time of backup.");
    println!();

    print!("Continue with restore? [y/N] ");
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .ok()
        .map(|_| input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes"))
        .unwrap_or(false)
}
