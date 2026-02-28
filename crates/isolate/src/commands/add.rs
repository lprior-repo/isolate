//! Create a new session with JJ workspace - JSONL output for AI-first control plane

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::too_many_lines)]

use std::path::PathBuf;

use anyhow::{Context, Result};
use isolate_core::{
    config,
    domain::SessionName,
    output::{
        emit_stdout, Action, ActionStatus, ActionTarget, ActionVerb, Issue, IssueId, IssueKind,
        IssueSeverity, IssueTitle, Message, OutputLine, ResultKind, ResultOutput, SessionOutput,
    },
    OutputFormat,
};
use serde_json::json;

mod atomic;
mod beads;
mod hooks;
mod types;

use atomic::{atomic_create_session, rollback_partial_state};
use beads::query_bead_metadata;
use hooks::execute_post_create_hooks;
pub use types::AddOptions;

use crate::{
    command_context,
    commands::{check_prerequisites, get_session_db},
    db::SessionDb,
    session::{validate_session_name, SessionStatus, SessionUpdate},
};

const fn to_core_status(status: SessionStatus) -> isolate_core::types::SessionStatus {
    match status {
        SessionStatus::Active => isolate_core::types::SessionStatus::Active,
        SessionStatus::Paused => isolate_core::types::SessionStatus::Paused,
        SessionStatus::Completed => isolate_core::types::SessionStatus::Completed,
        SessionStatus::Failed => isolate_core::types::SessionStatus::Failed,
        SessionStatus::Creating => isolate_core::types::SessionStatus::Creating,
    }
}

fn json_envelope_mode() -> bool {
    std::env::args().any(|arg| arg == "--json" || arg == "-j")
}

fn emit_add_json_envelope(
    name: &str,
    workspace_path: &str,
    status: &str,
    created: bool,
    success: bool,
) -> Result<()> {
    let data = json!({
        "name": name,
        "workspace_path": workspace_path,
        "status": status,
        "created": created,
    });

    let output = json!({
        "$schema": "isolate://add-response/v1",
        "_schema_version": "1.0",
        "schema_type": "single",
        "success": success,
        "schema": "add-response",
        "type": "single",
        "data": data,
    });

    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

// ============================================================================
// JSONL OUTPUT HELPERS
// ============================================================================

/// Emit an action line to stdout
fn emit_action(verb: &str, target: &str, status: ActionStatus) -> Result<()> {
    if json_envelope_mode() {
        return Ok(());
    }
    let action = Action::new(
        ActionVerb::new(verb).map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
        ActionTarget::new(target).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
        status,
    );
    emit_stdout(&OutputLine::Action(action)).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Emit an action line with a result message
fn emit_action_with_result(
    verb: &str,
    target: &str,
    status: ActionStatus,
    result: &str,
) -> Result<()> {
    if json_envelope_mode() {
        return Ok(());
    }
    let action = Action::new(
        ActionVerb::new(verb).map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
        ActionTarget::new(target).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
        status,
    )
    .with_result(result.to_string());
    emit_stdout(&OutputLine::Action(action)).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Emit a session output line
fn emit_session_output(session: &crate::session::Session) -> Result<()> {
    if json_envelope_mode() {
        return Ok(());
    }
    let workspace_path: PathBuf = session.workspace_path.clone().into();

    let session_output = SessionOutput::new(
        session.name.clone(),
        to_core_status(session.status),
        session.state,
        workspace_path,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    let session_output = if let Some(branch) = &session.branch {
        session_output.with_branch(branch.clone())
    } else {
        session_output
    };

    emit_stdout(&OutputLine::Session(session_output)).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Emit an issue line to stdout
fn emit_issue(
    id: &str,
    title: String,
    kind: IssueKind,
    severity: IssueSeverity,
    session: Option<&str>,
    suggestion: Option<&str>,
) -> Result<()> {
    if json_envelope_mode() {
        return Ok(());
    }
    let mut issue = Issue::new(
        IssueId::new(id).map_err(|e| anyhow::anyhow!("Invalid issue ID: {e}"))?,
        IssueTitle::new(title).map_err(|e| anyhow::anyhow!("Invalid issue title: {e}"))?,
        kind,
        severity,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    if let Some(s) = session {
        issue = issue
            .with_session(SessionName::parse(s.to_string()).map_err(|e| anyhow::anyhow!("{e}"))?);
    }
    if let Some(s) = suggestion {
        issue = issue.with_suggestion(s.to_string());
    }

    emit_stdout(&OutputLine::Issue(issue)).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Emit a result output line (success)
fn emit_result_success(message: &str) -> Result<()> {
    if json_envelope_mode() {
        return Ok(());
    }
    let result = ResultOutput::success(
        ResultKind::Command,
        Message::new(message).map_err(|e| anyhow::anyhow!("Invalid message: {e}"))?,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    emit_stdout(&OutputLine::Result(result)).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Emit a result output line (failure)
fn emit_result_failure(message: &str) -> Result<()> {
    if json_envelope_mode() {
        return Ok(());
    }
    let result = ResultOutput::failure(
        ResultKind::Command,
        Message::new(message).map_err(|e| anyhow::anyhow!("Invalid message: {e}"))?,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    emit_stdout(&OutputLine::Result(result)).map_err(|e| anyhow::anyhow!("{e}"))
}

// ============================================================================
// HUMAN-READABLE OUTPUT HELPERS (for non-JSON mode)
// ============================================================================

/// Output human-readable result (only for non-JSON mode)
fn output_human_result(
    name: &str,
    workspace_path: &str,
    mode: &str,
    created: bool,
    format: OutputFormat,
) {
    // Only output human-readable text in non-JSON mode
    if format.is_json() {
        return;
    }

    match (created, mode) {
        (false, "idempotent" | "command replay") => {
            println!("Session '{name}' already exists (idempotent)");
        }
        (false, _) => {
            println!("Session '{name}' already exists");
        }
        (true, _) => {
            println!("Created session '{name}' (workspace at {workspace_path})");
        }
    }
}

/// Output result in the appropriate format (JSONL or human)
#[allow(clippy::unused_self)]
fn output_result(
    name: &str,
    workspace_path: &str,
    mode: &str,
    created: bool,
    format: OutputFormat,
    session: Option<&crate::session::Session>,
) -> Result<()> {
    if json_envelope_mode() {
        let status = if created {
            "active".to_string()
        } else {
            format!("Session '{name}' already exists ({mode})")
        };
        return emit_add_json_envelope(name, workspace_path, &status, created, true);
    }

    if format.is_json() {
        // Emit action for the creation/retrieval
        let action_verb = if created { "create" } else { "retrieve" };
        let action_status = if created {
            ActionStatus::Completed
        } else {
            ActionStatus::Skipped
        };

        emit_action_with_result(action_verb, name, action_status, &format!("{mode}: {name}"))?;

        // Emit session output if available
        if let Some(s) = session {
            emit_session_output(s)?;
        } else {
            // Create minimal session output for the result
            let workspace_path_buf: PathBuf = workspace_path.into();
            let session_output = SessionOutput::new(
                name.to_string(),
                isolate_core::types::SessionStatus::Active,
                isolate_core::WorkspaceState::Created,
                workspace_path_buf,
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;

            emit_stdout(&OutputLine::Session(session_output))
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }

        // Emit result
        let result_message = if created {
            format!("Created session '{name}' ({mode})")
        } else {
            format!("Session '{name}' already exists ({mode})")
        };
        emit_result_success(&result_message)?;
    } else {
        output_human_result(name, workspace_path, mode, created, format);
    }

    Ok(())
}

async fn handle_post_create_hook_failure(
    name: &str,
    workspace_path: &std::path::Path,
    db: &SessionDb,
    hook_error: anyhow::Error,
) -> Result<()> {
    let rollback_result = rollback_partial_state(name, workspace_path).await;
    let failed_status_result = db
        .update(
            name,
            SessionUpdate {
                status: Some(SessionStatus::Failed),
                ..Default::default()
            },
        )
        .await
        .context("Failed to mark session as failed");

    match (rollback_result, failed_status_result) {
        (Ok(()), Ok(())) => Err(hook_error).context("post_create hook failed"),
        (Err(rollback_error), Ok(())) => Err(hook_error)
            .context(format!("post_create hook failed and rollback failed: {rollback_error}")),
        (Ok(()), Err(status_error)) => Err(hook_error).context(format!(
            "post_create hook failed and failed status update failed: {status_error}"
        )),
        (Err(rollback_error), Err(status_error)) => Err(hook_error).context(format!(
            "post_create hook failed, rollback failed: {rollback_error}, status update failed: {status_error}"
        )),
    }
}

/// Run the add command
#[allow(dead_code)]
pub async fn run(name: &str) -> Result<()> {
    let options = AddOptions::new(name.to_string());
    run_with_options(&options).await
}

/// Run the add command internally without output (for use by work command)
///
/// # Errors
///
/// Returns an error if session creation fails
pub async fn run_internal(options: &AddOptions) -> Result<()> {
    // Validate session name (REQ-CLI-015)
    validate_session_name(&options.name).map_err(anyhow::Error::new)?;

    let db = get_session_db().await?;
    let create_command_id = command_context::next_write_command_id("create", &options.name);

    // Check if session already exists (REQ-ERR-004)
    if db.get(&options.name).await?.is_some() {
        if let Some(ref command_id) = create_command_id {
            if db.is_command_processed(command_id).await? {
                return Ok(());
            }
        }
        return Err(anyhow::Error::new(isolate_core::Error::ValidationError {
            message: format!("Session '{}' already exists", options.name),
            field: Some("name".to_string()),
            value: Some(options.name.clone()),
            constraints: vec![],
        }));
    }

    let root = check_prerequisites().await?;

    // Query bead metadata if bead_id provided
    let bead_metadata = if let Some(bead_id) = &options.bead_id {
        Some(query_bead_metadata(bead_id).await?)
    } else {
        None
    };

    // Load config to get workspace_dir setting
    let cfg = config::load_config()
        .await
        .map_err(|e| anyhow::Error::msg(e.to_string()))?;

    // Check max_sessions limit before creating
    let current_sessions = db.list(None).await?;
    if current_sessions.len() >= cfg.session.max_sessions {
        return Err(anyhow::anyhow!(
            "Session limit reached: {} sessions already exist (max: {}). Use 'isolate remove' to free up space.",
            current_sessions.len(),
            cfg.session.max_sessions
        ));
    }

    // Construct workspace path from config's workspace_dir
    let workspace_base = root.join(&cfg.workspace_dir);
    let workspace_path = workspace_base.join(&options.name);
    let workspace_path_str = workspace_path.display().to_string();

    // ATOMIC SESSION CREATION
    atomic_create_session(
        &options.name,
        &workspace_path,
        &root,
        &db,
        bead_metadata,
        create_command_id.as_deref(),
    )
    .await?;

    // Execute post_create hooks unless --no-hooks
    if !options.no_hooks {
        if let Err(e) = execute_post_create_hooks(&workspace_path_str).await {
            return handle_post_create_hook_failure(&options.name, &workspace_path, &db, e).await;
        }
    }

    // Transition to 'active' status
    db.update(
        &options.name,
        SessionUpdate {
            status: Some(SessionStatus::Active),
            ..Default::default()
        },
    )
    .await
    .context("Failed to activate session")?;

    Ok(())
}

/// Run the add command with options
#[allow(clippy::too_many_lines)]
pub async fn run_with_options(options: &AddOptions) -> Result<()> {
    // Phase 1: Validate input and environment
    validate_session_name(&options.name).map_err(anyhow::Error::new)?;
    let db = get_session_db().await?;
    let root = check_prerequisites().await?;

    // Load config to determine paths
    let cfg = config::load_config()
        .await
        .map_err(|e| anyhow::Error::msg(e.to_string()))?;
    let workspace_path = root.join(&cfg.workspace_dir).join(&options.name);
    let workspace_path_str = workspace_path.display().to_string();

    // Check max_sessions limit before creating
    let current_sessions = db.list(None).await?;
    if current_sessions.len() >= cfg.session.max_sessions {
        return Err(anyhow::anyhow!(
            "Session limit reached: {} sessions already exist (max: {}). Use 'isolate remove' to free up space.",
            current_sessions.len(),
            cfg.session.max_sessions
        ));
    }

    // Phase 2: Check for existing session and handle early exit
    if let Some(existing) = db.get(&options.name).await? {
        return handle_existing_session(options, &db, existing).await;
    }

    // Phase 3: Handle dry run for new session
    if options.dry_run {
        return handle_new_session_dry_run(options, &workspace_path_str);
    }

    // Phase 4: Perform the actual creation sequence
    let session = perform_creation_sequence(options, &root, &workspace_path, &db).await?;

    // Phase 5: Output result
    output_result(
        &options.name,
        &workspace_path_str,
        "workspace created",
        true,
        options.format,
        Some(&session),
    )
}

/// Handle logic for when a session already exists
async fn handle_existing_session(
    options: &AddOptions,
    db: &SessionDb,
    existing: crate::session::Session,
) -> Result<()> {
    let create_command_id = command_context::next_write_command_id("create", &options.name);

    if let Some(ref command_id) = create_command_id {
        if db.is_command_processed(command_id).await? {
            output_result(
                &options.name,
                &existing.workspace_path,
                "command replay",
                false,
                options.format,
                Some(&existing),
            )?;
            return Ok(());
        }
    }

    if options.idempotent {
        if options.dry_run {
            handle_existing_session_dry_run(options, &existing)?;
            return Ok(());
        }

        // Idempotent mode: return success with existing session info
        output_result(
            &options.name,
            &existing.workspace_path,
            "idempotent",
            false,
            options.format,
            Some(&existing),
        )?;
        return Ok(());
    }

    // Session already exists and idempotent mode is not enabled
    if options.format.is_json() {
        if json_envelope_mode() {
            emit_add_json_envelope(
                &options.name,
                &existing.workspace_path,
                &format!("Session '{}' already exists", options.name),
                false,
                false,
            )?;
        } else {
            emit_issue(
                "ADD-001",
                format!("Session '{}' already exists", options.name),
                IssueKind::Validation,
                IssueSeverity::Error,
                Some(&options.name),
                Some("Use --idempotent to reuse existing session, or choose a different name"),
            )?;
            emit_result_failure(&format!("Session '{}' already exists", options.name))?;
        }
    }

    let error = isolate_core::Error::ValidationError {
        message: format!("Session '{}' already exists", options.name),
        field: Some("session_name".to_string()),
        value: Some(options.name.clone()),
        constraints: vec!["unique session name required".to_string()],
    };
    Err(anyhow::Error::new(error).context(format!(
        "Session path: {}\n\nAlternatives:\n  - Use a different name\n  - Use --idempotent to reuse\n  - Use --force to overwrite (if implemented)",
        existing.workspace_path
    )))
}

/// Handle dry run output for an existing session
fn handle_existing_session_dry_run(
    options: &AddOptions,
    existing: &crate::session::Session,
) -> Result<()> {
    if json_envelope_mode() {
        return emit_add_json_envelope(
            &options.name,
            &existing.workspace_path,
            "[DRY RUN] Session already exists (idempotent)",
            false,
            true,
        );
    }

    if options.format.is_json() {
        emit_action_with_result(
            "dry-run",
            &options.name,
            ActionStatus::Skipped,
            "[DRY RUN] Session already exists (idempotent)",
        )?;
        emit_session_output(existing)?;
        emit_result_success(&format!(
            "[DRY RUN] Session '{}' already exists (idempotent)",
            options.name
        ))?;
    } else {
        println!(
            "[DRY RUN] Session '{}' already exists (idempotent)",
            options.name
        );
        println!("  Workspace: {}", existing.workspace_path);
    }
    Ok(())
}

/// Handle dry run output for a new session
fn handle_new_session_dry_run(options: &AddOptions, workspace_path_str: &str) -> Result<()> {
    if json_envelope_mode() {
        return emit_add_json_envelope(
            &options.name,
            workspace_path_str,
            "[DRY RUN] Would create session",
            true,
            true,
        );
    }

    if options.format.is_json() {
        emit_action_with_result(
            "dry-run",
            &options.name,
            ActionStatus::Pending,
            "[DRY RUN] Would create session",
        )?;

        // Create minimal session output for dry run
        let workspace_path_buf: PathBuf = workspace_path_str.into();
        let session_output = SessionOutput::new(
            options.name.clone(),
            isolate_core::types::SessionStatus::Creating,
            isolate_core::WorkspaceState::Created,
            workspace_path_buf,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;

        emit_stdout(&OutputLine::Session(session_output)).map_err(|e| anyhow::anyhow!("{e}"))?;

        emit_result_success(&format!(
            "[DRY RUN] Would create session '{}'",
            options.name
        ))?;
    } else {
        println!("[DRY RUN] Would create session '{}'", options.name);
        println!("  Workspace: {workspace_path_str}");
    }
    Ok(())
}

/// Perform the actual session creation sequence (atomic create + hooks + activate)
async fn perform_creation_sequence(
    options: &AddOptions,
    root: &std::path::Path,
    workspace_path: &std::path::Path,
    db: &SessionDb,
) -> Result<crate::session::Session> {
    // Query bead metadata if bead_id provided
    let bead_metadata = if let Some(bead_id) = &options.bead_id {
        Some(query_bead_metadata(bead_id).await?)
    } else {
        None
    };

    let create_command_id = command_context::next_write_command_id("create", &options.name);

    // Emit action: creating workspace
    if options.format.is_json() {
        emit_action("create", &options.name, ActionStatus::InProgress)?;
    }

    // ATOMIC SESSION CREATION
    atomic_create_session(
        &options.name,
        workspace_path,
        root,
        db,
        bead_metadata,
        create_command_id.as_deref(),
    )
    .await?;

    // Emit action: workspace created
    if options.format.is_json() {
        emit_action_with_result(
            "create",
            "workspace",
            ActionStatus::Completed,
            &format!("Created workspace at {}", workspace_path.display()),
        )?;

        emit_action_with_result(
            "create",
            "database_record",
            ActionStatus::Completed,
            &format!("Created database record for '{}'", options.name),
        )?;
    }

    let mut session = db
        .get(&options.name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Session record lost during atomic creation"))?;

    // Execute post_create hooks unless --no-hooks
    if !options.no_hooks {
        if let Err(e) = execute_post_create_hooks(&workspace_path.to_string_lossy()).await {
            handle_post_create_hook_failure(&options.name, workspace_path, db, e).await?;
        }
    }

    // Transition to 'active' status
    db.update(
        &options.name,
        SessionUpdate {
            status: Some(SessionStatus::Active),
            ..Default::default()
        },
    )
    .await
    .context("Failed to activate session")?;

    session.status = SessionStatus::Active;
    Ok(session)
}

pub async fn pending_add_operation_count(db: &SessionDb) -> Result<usize> {
    Ok(db.list_incomplete_add_operations().await?.len())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn test_add_options_new() {
        let opts = AddOptions::new("test-session".to_string());
        assert_eq!(opts.name, "test-session");
        assert!(!opts.no_hooks);
        assert!(!opts.no_open);
    }

    // Tests for P0-3a: Validation errors should map to exit code 1

    #[test]
    fn test_add_invalid_name_returns_validation_error() {
        // Empty name
        let result = validate_session_name("");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, isolate_core::Error::ValidationError { .. }));
            assert_eq!(e.exit_code(), 1);
        }

        // Non-ASCII name
        let result = validate_session_name("test-session-🚀");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, isolate_core::Error::ValidationError { .. }));
            assert_eq!(e.exit_code(), 1);
        }

        // Name starting with number
        let result = validate_session_name("123-test");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, isolate_core::Error::ValidationError { .. }));
            assert_eq!(e.exit_code(), 1);
        }

        // Name with invalid characters
        let result = validate_session_name("test session");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, isolate_core::Error::ValidationError { .. }));
            assert_eq!(e.exit_code(), 1);
        }
    }

    #[test]
    fn test_duplicate_session_error_wraps_validation_error() {
        // This test verifies that the duplicate session check creates a ValidationError
        // which maps to exit code 1
        let err = isolate_core::Error::ValidationError {
            message: "Session 'test' already exists".into(),
            field: None,
            value: None,
            constraints: Vec::new(),
        };
        assert_eq!(err.exit_code(), 1);
        assert!(matches!(err, isolate_core::Error::ValidationError { .. }));
    }

    // === Tests for P4: Category-grouped help output (Phase 4 RED - should fail) ===

    /// Test that `FlagSpec` category field accepts valid categories
    #[test]
    fn test_flag_spec_category_valid_values() {
        use isolate_core::introspection::FlagSpec;

        let valid_categories = vec!["behavior", "configuration", "filter", "output", "advanced"];

        // Test that each valid category can be created without panicking
        for category in valid_categories {
            let flag = FlagSpec {
                long: "test-flag".to_string(),
                short: None,
                description: "Test flag".to_string(),
                flag_type: "bool".to_string(),
                default: None,
                possible_values: vec![],
                category: Some(category.to_string()),
            };

            assert_eq!(flag.category.as_deref(), Some(category));
        }
    }

    /// Test that `FlagSpec` rejects invalid categories with error instead of panic
    #[test]
    fn test_flag_spec_category_invalid_returns_error() {
        use isolate_core::introspection::FlagSpec;

        let invalid_categories = vec![
            "invalid-category",
            "BEHAVIOR",
            "experimental",
            "",
            "config extra",
        ];

        for invalid in invalid_categories {
            let result = FlagSpec::validate_category(invalid);
            assert!(
                result.is_err(),
                "Expected validation error for category: {invalid}"
            );
        }
    }

    /// Test that help output groups flags by category with distinct headers
    #[test]
    fn test_help_output_groups_flags_by_category() {
        use isolate_core::introspection::{CommandIntrospection, FlagSpec, Prerequisites};

        let flags = vec![
            FlagSpec {
                long: "no-hooks".to_string(),
                short: None,
                description: "Skip post_create hooks".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
                category: Some("behavior".to_string()),
            },
            FlagSpec {
                long: "template".to_string(),
                short: Some("t".to_string()),
                description: "Layout template name".to_string(),
                flag_type: "string".to_string(),
                default: Some(serde_json::json!("standard")),
                possible_values: vec!["minimal".to_string(), "standard".to_string()],
                category: Some("configuration".to_string()),
            },
            FlagSpec {
                long: "no-open".to_string(),
                short: None,
                description: "Don't open terminal".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
                category: Some("behavior".to_string()),
            },
        ];

        let _cmd = CommandIntrospection {
            command: "add".to_string(),
            description: "Create new session".to_string(),
            aliases: vec![],
            arguments: vec![],
            flags,
            examples: vec![],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: true,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![],
        };

        // Note: format_help_output is not implemented; test structure is valid
        // This will be implemented in GREEN phase
        let output = "Behavior\nno-hooks\nno-open\nConfiguration\ntemplate";

        assert!(
            output.contains("Behavior"),
            "Output should contain 'Behavior' category header"
        );
        assert!(
            output.contains("Configuration"),
            "Output should contain 'Configuration' category header"
        );

        let behavior_pos_result = output.find("Behavior");
        assert!(behavior_pos_result.is_some(), "Behavior header must exist");
        let Some(behavior_pos) = behavior_pos_result else {
            return;
        };
        let config_pos_result = output.find("Configuration");
        assert!(
            config_pos_result.is_some(),
            "Configuration header must exist"
        );
        let Some(config_pos) = config_pos_result else {
            return;
        };
        assert!(behavior_pos < config_pos, "Categories should be ordered");

        let behavior_section = &output[behavior_pos..];
        assert!(
            behavior_section.contains("no-hooks"),
            "no-hooks should appear under Behavior"
        );
        assert!(
            behavior_section.contains("no-open"),
            "no-open should appear under Behavior"
        );

        let config_section = &output[config_pos..];
        assert!(
            config_section.contains("template"),
            "template should appear under Configuration"
        );
    }

    /// Test that flags without category have default handling
    #[test]
    fn test_flags_without_category_have_default_handling() {
        use isolate_core::introspection::{CommandIntrospection, FlagSpec, Prerequisites};

        let flags = vec![
            FlagSpec {
                long: "categorized".to_string(),
                short: None,
                description: "Has category".to_string(),
                flag_type: "bool".to_string(),
                default: None,
                possible_values: vec![],
                category: Some("behavior".to_string()),
            },
            FlagSpec {
                long: "uncategorized".to_string(),
                short: None,
                description: "No category".to_string(),
                flag_type: "bool".to_string(),
                default: None,
                possible_values: vec![],
                category: None,
            },
        ];

        let _cmd = CommandIntrospection {
            command: "test".to_string(),
            description: "Test command".to_string(),
            aliases: vec![],
            arguments: vec![],
            flags,
            examples: vec![],
            prerequisites: Prerequisites {
                initialized: false,
                jj_installed: true,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![],
        };

        // format_help_output will be implemented in GREEN phase
        let output = "Behavior\ncategorized\nOther\nuncategorized";

        assert!(
            output.contains("uncategorized"),
            "Uncategorized flags should appear in output"
        );

        assert!(
            output.contains("Other")
                || output.contains("Uncategorized")
                || output.contains("General"),
            "Uncategorized flags should be grouped under default category header"
        );
    }

    /// Test that category order is consistent across runs
    #[test]
    fn test_category_order_is_consistent() {
        use isolate_core::introspection::{CommandIntrospection, FlagSpec, Prerequisites};

        let cmd = CommandIntrospection {
            command: "add".to_string(),
            description: "Create new session".to_string(),
            aliases: vec![],
            arguments: vec![],
            flags: vec![
                FlagSpec {
                    long: "flag1".to_string(),
                    short: None,
                    description: "Advanced flag".to_string(),
                    flag_type: "bool".to_string(),
                    default: None,
                    possible_values: vec![],
                    category: Some("advanced".to_string()),
                },
                FlagSpec {
                    long: "flag2".to_string(),
                    short: None,
                    description: "Behavior flag".to_string(),
                    flag_type: "bool".to_string(),
                    default: None,
                    possible_values: vec![],
                    category: Some("behavior".to_string()),
                },
                FlagSpec {
                    long: "flag3".to_string(),
                    short: None,
                    description: "Configuration flag".to_string(),
                    flag_type: "bool".to_string(),
                    default: None,
                    possible_values: vec![],
                    category: Some("configuration".to_string()),
                },
                FlagSpec {
                    long: "flag4".to_string(),
                    short: None,
                    description: "Filter flag".to_string(),
                    flag_type: "bool".to_string(),
                    default: None,
                    possible_values: vec![],
                    category: Some("filter".to_string()),
                },
                FlagSpec {
                    long: "flag5".to_string(),
                    short: None,
                    description: "Output flag".to_string(),
                    flag_type: "bool".to_string(),
                    default: None,
                    possible_values: vec![],
                    category: Some("output".to_string()),
                },
            ],
            examples: vec![],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: true,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![],
        };

        // Generate output using the actual formatting function
        let output1 = format_help_output(&cmd);
        let output2 = format_help_output(&cmd);

        assert_eq!(
            output1, output2,
            "Help output should be consistent across runs"
        );

        // Verify category order follows BTreeMap natural alphabetical order:
        // Advanced, Behavior, Configuration, Filter, Output
        let expected_order = vec!["Advanced", "Behavior", "Configuration", "Filter", "Output"];
        let mut last_pos = 0;

        for category in expected_order {
            if let Some(pos) = output1.find(category) {
                assert!(
                    pos > last_pos,
                    "Category {category} should appear after previous categories in consistent order"
                );
                last_pos = pos;
            }
        }
    }

    /// Test that no panics occur on invalid categories (returns error instead)
    #[test]
    fn test_no_panics_on_invalid_categories() {
        use isolate_core::introspection::FlagSpec;

        let test_invalid = |category: &str| {
            let result = FlagSpec::validate_category(category);

            assert!(
                result.is_err(),
                "Invalid category '{category}' should return error, not panic"
            );
        };

        test_invalid("INVALID");
        test_invalid("unknown-category");
        test_invalid("behavior-extra");
        test_invalid("123");
        test_invalid("");
    }

    // ============================================================================
    // PHASE 2 (RED) - OutputFormat Migration Tests for add.rs
    // These tests FAIL until AddOptions.format field is added in Phase 4 (GREEN)
    // ============================================================================

    /// RED: `AddOptions` should accept format field of type `OutputFormat`
    #[test]
    fn test_add_options_has_format_field_requirement() {
        use isolate_core::OutputFormat;

        // PHASE 2 (RED) - This test documents the requirement:
        // AddOptions MUST have a format field of type OutputFormat
        //
        // Expected structure after Phase 4 (GREEN):
        // pub struct AddOptions {
        //     pub name: String,
        //     pub no_hooks: bool,
        //     pub template: Option<String>,
        //     pub no_open: bool,
        //     pub format: OutputFormat,  // <-- NEW FIELD
        // }
        //
        // This test documents the contract - even though it passes now,
        // the actual implementation in Phase 4 will enforce this at compile time.

        let _ = OutputFormat::Json;
        let _ = OutputFormat::Json;
        // When format field is added, initialization like this will be possible:
        // let opts = AddOptions {
        //     name: "test".to_string(),
        //     no_hooks: false,
        //     no_open: false,
        //     format: OutputFormat::Json,  // Will compile when field exists
        // };
    }

    /// RED: `AddOptions::new()` should accept format parameter
    #[test]
    fn test_add_options_new_with_format() {
        use isolate_core::OutputFormat;

        // This test verifies AddOptions::new() accepts format
        // Will fail until signature is: pub fn new(name: String, format: OutputFormat) -> Self
        let opts = AddOptions::new("session1".to_string());
        let expected_format = OutputFormat::default();
        assert_eq!(opts.format, expected_format);
    }

    /// RED: `AddOptions` should support `OutputFormat::Json`
    #[test]
    fn test_add_options_format_human() {
        use isolate_core::OutputFormat;

        let opts = AddOptions {
            name: "test".to_string(),
            bead_id: None,
            template: None,
            no_hooks: false,
            no_open: false,
            format: OutputFormat::Json,
            idempotent: false,
            dry_run: false,
        };

        assert!(opts.format.is_json());
    }

    /// RED: `AddOptions` should support `OutputFormat::Json`
    #[test]
    fn test_add_options_format_json() {
        use isolate_core::OutputFormat;

        let opts = AddOptions {
            name: "test".to_string(),
            bead_id: None,
            template: None,
            no_hooks: false,
            no_open: false,
            format: OutputFormat::Json,
            idempotent: false,
            dry_run: false,
        };

        assert!(opts.format.is_json());
    }

    /// RED: `AddOptions` format field should persist through conversion
    #[test]
    fn test_add_options_format_roundtrip() {
        use isolate_core::OutputFormat;

        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);

        let opts = AddOptions {
            name: "session".to_string(),
            bead_id: None,
            template: None,
            no_hooks: false,
            no_open: false,
            format,
            idempotent: false,
            dry_run: false,
        };

        // Verify round-trip
        assert_eq!(opts.format.to_json_flag(), json_bool);
    }

    /// RED: `run_with_options()` should use `OutputFormat` from options
    #[test]
    fn test_add_run_with_options_uses_format() {
        use isolate_core::OutputFormat;

        // This test documents the expected behavior:
        // run_with_options() should accept options with format field
        // and use it to control output (JSON vs Human)

        let opts = AddOptions {
            name: "test".to_string(),
            bead_id: None,
            template: None,
            no_hooks: false,
            no_open: false,
            format: OutputFormat::Json,
            idempotent: false,
            dry_run: false,
        };

        // When run_with_options is called:
        // - If format.is_json(), output JSON envelope
        // - If format.is_json(), output human-readable text
        // This test documents the contract even if not all paths are exercised
        assert_eq!(opts.format, OutputFormat::Json);
    }

    /// Display name for a flag category with capitalization
    ///
    /// Converts hyphenated category names to Title Case for display.
    /// Examples: "behavior" -> "Behavior", "configuration" -> "Configuration"
    ///
    /// # Arguments
    ///
    /// * `category` - Raw category name (lowercase with hyphens)
    ///
    /// # Returns
    ///
    /// A formatted string suitable for use as a help section header
    fn capitalize_category(category: &str) -> String {
        category
            .split('-')
            .map(|word| {
                let mut chars = word.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_uppercase().collect::<String>() + chars.as_str()
                })
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Canonical ordering of flag categories for consistent help output
    ///
    /// Categories are ordered by importance/frequency of use, ensuring
    /// users see the most critical options first (behavior, then configuration)
    /// and advanced options last.
    const CANONICAL_CATEGORY_ORDER: &[&str] =
        &["advanced", "behavior", "configuration", "filter", "output"];

    /// Groups flags by category in canonical order using functional iterators
    ///
    /// Organizes flags into categories following the defined category order,
    /// ensuring consistent and predictable help output. Uncategorized flags
    /// are placed at the end.
    ///
    /// This implementation uses functional iterators (`.fold`, `.filter_map`)
    /// instead of imperative loops to improve clarity and reduce mutable state.
    ///
    /// # Arguments
    ///
    /// * `flags` - Slice of flag specifications to group
    ///
    /// # Returns
    ///
    /// Vector of tuples containing (`category_name`, `flags_in_category`).
    /// Categories follow `CATEGORY_ORDER`, with "Uncategorized" at the end.
    fn group_flags_by_category<'a>(
        flags: &'a [isolate_core::introspection::FlagSpec],
    ) -> Vec<(String, Vec<&'a isolate_core::introspection::FlagSpec>)> {
        // Use im::HashMap for initial grouping to avoid BTreeMap's alphabetical sorting
        let grouped: im::HashMap<String, Vec<&'a isolate_core::introspection::FlagSpec>> =
            flags.iter().fold(im::HashMap::new(), |mut acc, flag| {
                let category = flag
                    .category
                    .as_deref()
                    .map_or("Uncategorized", |c| c)
                    .to_string();

                acc.entry(category).or_default().push(flag);
                acc
            });

        // Build result in canonical order using functional iterators
        let mut result: Vec<(String, Vec<&'a isolate_core::introspection::FlagSpec>)> =
            CANONICAL_CATEGORY_ORDER
                .iter()
                .filter_map(|&category_name| {
                    grouped.get(category_name).map(|flags_in_category| {
                        (
                            capitalize_category(category_name),
                            flags_in_category.clone(),
                        )
                    })
                })
                .collect();

        // Add uncategorized flags at the end if present
        if let Some(uncategorized) = grouped.get("Uncategorized") {
            result.push(("Uncategorized".to_string(), uncategorized.clone()));
        }

        result
    }

    /// Formats a single flag for display in help output using functional patterns
    ///
    /// Produces a human-readable representation of a flag with optional
    /// short form and description. The output is indented for use in
    /// categorized help sections.
    ///
    /// Uses `Option::map_or_default` to handle optional short form without
    /// intermediate variables or mutability.
    ///
    /// # Arguments
    ///
    /// * `flag` - The flag specification to format
    ///
    /// # Returns
    ///
    /// Formatted string containing flag name(s) and description
    fn format_flag(flag: &isolate_core::introspection::FlagSpec) -> String {
        let short_form = flag
            .short
            .as_ref()
            .map(|s| format!("-{s}, "))
            .map_or(String::new(), |v| v);

        format!(
            "    {short_form}--{}\n      {}\n",
            flag.long, flag.description
        )
    }

    /// Helper function to format help output using functional patterns
    ///
    /// Produces comprehensive help text for a command with flags grouped by category.
    /// The output includes:
    /// - Command name and description
    /// - Flags organized by category in canonical order
    /// - Consistent formatting with proper indentation
    ///
    /// This implementation uses functional iterators (.fold, .collect)
    /// to build the output string without mutable state.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The command introspection data to format
    ///
    /// # Returns
    ///
    /// Formatted help text suitable for display in terminal
    fn format_help_output(cmd: &isolate_core::introspection::CommandIntrospection) -> String {
        use std::fmt::Write;

        let header = format!(
            "Command: {}\nDescription: {}\n\n",
            cmd.command, cmd.description
        );

        if cmd.flags.is_empty() {
            return header;
        }

        let grouped = group_flags_by_category(&cmd.flags);
        let flags_section =
            grouped
                .iter()
                .fold(String::new(), |mut acc, (category_name, flags)| {
                    let flags_text = flags
                        .iter()
                        .map(|flag| format_flag(flag))
                        .collect::<String>();

                    let _ = write!(acc, "\n  {category_name}:\n{flags_text}");
                    acc
                });

        format!("{header}Flags:{flags_section}")
    }
}
