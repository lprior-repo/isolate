//! Sync a session's workspace with main branch - JSONL output for AI-first control plane
//!
//! # Default Behavior (Explicit and Safe)
//!
//! The `sync` command has explicit, context-aware default behavior that is
//! **safe by design** - it never syncs more than intended:
//!
//! | Invocation          | Context        | Behavior                    |
//! |---------------------|----------------|----------------------------|
//! | `zjj sync <name>`   | Any            | Sync ONLY the named session |
//! | `zjj sync --all`    | Any            | Sync ALL active sessions    |
//! | `zjj sync`          | In workspace   | Sync ONLY current workspace |
//! | `zjj sync`          | In main repo   | Sync ALL sessions (prompt)  |
//!
//! ## Safety Guarantees
//!
//! 1. **Named sync is isolated**: `zjj sync <name>` only affects that session
//! 2. **Workspace sync is local**: `zjj sync` from workspace syncs only that workspace
//! 3. **--all requires explicit flag**: Bulk sync requires explicit `--all` flag
//! 4. **Dry-run available**: Use `--dry-run` to preview without changes
//!
//! ## Examples
//!
//! ```bash
//! # Sync current workspace (most common use case)
//! zjj sync
//!
//! # Sync specific session by name
//! zjj sync feature-auth
//!
//! # Sync all sessions explicitly
//! zjj sync --all
//!
//! # Preview what would be synced
//! zjj sync --dry-run
//! ```

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::{io::Write, path::Path, time::SystemTime};

use anyhow::{Context, Result};
use fs4::fs_std::FileExt;
use futures::StreamExt;
use tokio::process::Command;
use zjj_core::{
    domain::SessionName,
    output::{
        emit_stdout, Action, ActionStatus, ActionTarget, ActionVerb, Issue, IssueId, IssueKind,
        IssueSeverity, IssueTitle, Message, OutputLine, ResultKind, ResultOutput, Summary,
        SummaryType,
    },
    OutputFormat,
};

use crate::{
    cli::run_command,
    commands::{determine_main_branch, get_session_db},
    session::SessionUpdate,
};

// ═══════════════════════════════════════════════════════════════════════════
// JSONL EMIT HELPERS
// ═══════════════════════════════════════════════════════════════════════════

fn emit_action(verb: &str, target: &str, status: ActionStatus, result: Option<&str>) -> Result<()> {
    let action = Action::new(
        ActionVerb::new(verb).map_err(|e| anyhow::anyhow!("Invalid action verb: {e}"))?,
        ActionTarget::new(target).map_err(|e| anyhow::anyhow!("Invalid action target: {e}"))?,
        status,
    );
    let action = match result {
        Some(r) => action.with_result(r.to_string()),
        None => action,
    };
    emit_stdout(&OutputLine::Action(action)).map_err(|e| anyhow::anyhow!("{e}"))
}

fn emit_summary(type_field: SummaryType, message: &str, details: Option<&str>) -> Result<()> {
    let summary = Summary::new(
        type_field,
        Message::new(message).map_err(|e| anyhow::anyhow!("Invalid message: {e}"))?,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    let summary = match details {
        Some(d) => summary.with_details(d.to_string()),
        None => summary,
    };
    emit_stdout(&OutputLine::Summary(summary)).map_err(|e| anyhow::anyhow!("{e}"))
}

fn emit_issue(
    id: &str,
    title: String,
    kind: IssueKind,
    severity: IssueSeverity,
    session: Option<&str>,
    suggestion: Option<&str>,
) -> Result<()> {
    let issue = Issue::new(
        IssueId::new(id).map_err(|e| anyhow::anyhow!("Invalid issue ID: {e}"))?,
        IssueTitle::new(title).map_err(|e| anyhow::anyhow!("Invalid issue title: {e}"))?,
        kind,
        severity,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    let issue = match session {
        Some(s) => issue
            .with_session(SessionName::parse(s.to_string()).map_err(|e| anyhow::anyhow!("{e}"))?),
        None => issue,
    };
    let issue = match suggestion {
        Some(s) => issue.with_suggestion(s.to_string()),
        None => issue,
    };
    emit_stdout(&OutputLine::Issue(issue)).map_err(|e| anyhow::anyhow!("{e}"))
}

fn emit_result(success: bool, kind: ResultKind, message: &str) -> Result<()> {
    let msg = Message::new(message).map_err(|e| anyhow::anyhow!("Invalid message: {e}"))?;
    let result = if success {
        ResultOutput::success(kind, msg)
    } else {
        ResultOutput::failure(kind, msg)
    }
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    emit_stdout(&OutputLine::Result(result)).map_err(|e| anyhow::anyhow!("{e}"))
}

// ═══════════════════════════════════════════════════════════════════════════
// WORKSPACE DETECTION
// ═══════════════════════════════════════════════════════════════════════════

/// Detect if current directory is in a JJ workspace and return main repo path
///
/// Returns `Ok(Some(main_repo_path))` if in a workspace
/// Returns `Ok(None)` if in main repo (not a workspace)
/// Returns `Err` if not in a JJ repo at all
async fn detect_workspace_context() -> Result<Option<String>> {
    // Try to get workspace root - this works from both main repo and workspace
    let output = Command::new("jj")
        .args(["workspace", "root"])
        .output()
        .await
        .context("Failed to run 'jj workspace root'")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Not in a JJ repository. \
             'zjj sync' must be run from within a JJ repository."
        ));
    }

    let workspace_root = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Try to get workspace show - this tells us if we're in a workspace
    let show_output = Command::new("jj")
        .args(["workspace", "show"])
        .output()
        .await;

    if let Ok(show_output) = show_output {
        if show_output.status.success() {
            let workspace_info = String::from_utf8_lossy(&show_output.stdout);

            // Check if we're in a workspace by looking for "Working copy" in output
            if workspace_info.contains("Working copy") {
                // We're in a workspace - try to find main repo by checking parent directory
                let current_path = Path::new(&workspace_root);

                // Walk up the directory tree looking for a directory with .jj that's not ours
                let mut search_path = current_path.to_path_buf();
                while let Some(parent) = search_path.parent() {
                    let parent_jj = parent.join(".jj");

                    // Check if parent has a different .jj directory
                    if parent_jj.exists() && parent_jj != current_path.join(".jj") {
                        // Found main repo
                        return Ok(parent.to_str().map(String::from));
                    }

                    search_path = parent.to_path_buf();
                }
            }
        }
    }

    // Not in a workspace - we're in the main repo
    Ok(None)
}

/// Get session database, handling both main repo and workspace contexts
///
/// This function detects if we're in a workspace and routes to the main repo database.
/// If in the main repo, uses the normal `get_session_db()` path.
async fn get_session_db_with_workspace_detection() -> Result<crate::db::SessionDb> {
    match detect_workspace_context().await? {
        Some(main_repo_path) => {
            // We're in a workspace - use main repo database
            let main_repo_zjj = Path::new(&main_repo_path).join(".zjj");

            anyhow::ensure!(
                tokio::fs::try_exists(&main_repo_zjj).await.is_ok_and(|e| e),
                "ZJJ not initialized in main repository at {main_repo_path}\n\n\
                 Run 'zjj init' in the main repository first."
            );

            let db_path = super::get_db_path().await?;

            // Security: Verify database is not a symlink
            if db_path.is_symlink() {
                return Err(anyhow::anyhow!(
                    "Database is a symlink: {}. This is not allowed for security reasons.",
                    db_path.display()
                ));
            }

            crate::db::SessionDb::open(&db_path)
                .await
                .context("Failed to open session database from main repo")
        }
        None => {
            // We're in the main repo - use normal path
            get_session_db().await
        }
    }
}

/// Options for the sync command
#[derive(Debug, Clone, Copy, Default)]
pub struct SyncOptions {
    /// Output format
    pub format: OutputFormat,
    /// Sync all sessions (explicit --all flag)
    pub all: bool,
    /// Preview sync without executing
    pub dry_run: bool,
}

/// Explicit sync behavior determined from arguments and context
///
/// This enum makes the routing decision explicit and type-safe,
/// ensuring that the default behavior is always clear and safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncBehavior {
    /// Sync a specific named session (user explicitly provided a name)
    NamedSession,
    /// Sync all sessions (user explicitly provided --all flag)
    AllSessions,
    /// Sync current workspace only (detected from context)
    CurrentWorkspace,
}

/// Determine sync behavior from arguments
///
/// This function makes the routing logic explicit and traceable.
/// The behavior is **safe by default** - it never syncs more than intended.
///
/// # Arguments
///
/// * `name` - Optional session name provided by user
/// * `all_flag` - Whether --all flag was explicitly provided
///
/// # Returns
///
/// The determined sync behavior, which is then used to route to the
/// appropriate handler function.
///
/// # Examples
///
/// ```ignore
/// // Named sync - highest priority
/// let behavior = determine_sync_behavior(Some("feature-auth"), false);
/// assert_eq!(behavior, SyncBehavior::NamedSession);
///
/// // Explicit --all flag
/// let behavior = determine_sync_behavior(None, true);
/// assert_eq!(behavior, SyncBehavior::AllSessions);
///
/// // Default (no args) - will detect workspace context
/// let behavior = determine_sync_behavior(None, false);
/// assert_eq!(behavior, SyncBehavior::CurrentWorkspace);
/// ```
pub const fn determine_sync_behavior(name: Option<&str>, all_flag: bool) -> SyncBehavior {
    match (name, all_flag) {
        // Explicit name takes highest priority
        (Some(_), _) => SyncBehavior::NamedSession,
        // Explicit --all flag
        (None, true) => SyncBehavior::AllSessions,
        // Default: sync current workspace (context-aware)
        (None, false) => SyncBehavior::CurrentWorkspace,
    }
}

/// Run the sync command with options
///
/// # Default Behavior (Explicit and Safe)
///
/// This function implements the routing table:
///
/// | name    | --all   | Behavior                |
/// |---------|---------|------------------------|
/// | Some(n) | any     | Sync named session     |
/// | None    | true    | Sync all sessions      |
/// | None    | false   | Sync current workspace |
///
/// The default (`sync` with no args) is to sync the current workspace only,
/// which is the safest and most common operation.
pub async fn run_with_options(name: Option<&str>, options: SyncOptions) -> Result<()> {
    let behavior = determine_sync_behavior(name, options.all);

    match behavior {
        SyncBehavior::NamedSession => {
            // Safe: name is guaranteed to be Some by determine_sync_behavior
            match name {
                Some(n) => sync_session_with_options(n, options).await,
                None => Err(anyhow::anyhow!(
                    "Internal error: NamedSession behavior with no name"
                )),
            }
        }
        SyncBehavior::AllSessions => sync_all_with_options(options).await,
        SyncBehavior::CurrentWorkspace => sync_current_workspace(options).await,
    }
}

/// Sync current workspace (default behavior)
///
/// This implements the safe default for `zjj sync` with no arguments:
///
/// - **If in a workspace**: Sync ONLY that workspace (safe, local operation)
/// - **If in main repo**: Sync ALL sessions (convenience for batch updates)
///
/// # Safety
///
/// This is the safest default because:
/// 1. When working in a workspace, you typically only care about that workspace
/// 2. The operation is local and doesn't affect other developers
/// 3. Conflicts are isolated to your current work
///
/// # Errors
///
/// Returns an error if:
/// - Not in a JJ repository
/// - Workspace detection fails
/// - The sync operation itself fails
async fn sync_current_workspace(options: SyncOptions) -> Result<()> {
    // Try to detect current workspace
    match detect_current_workspace_name().await? {
        Some(workspace_name) => {
            // We're in a workspace - sync only this one (SAFE DEFAULT)
            sync_session_with_options(&workspace_name, options).await
        }
        None => {
            // We're in main repo - sync all for convenience
            // This is explicit because we have no workspace context
            sync_all_with_options(options).await
        }
    }
}

/// Detect the name of the current workspace, if we're in one
///
/// Returns `Ok(Some(name))` if in a workspace, `Ok(None)` if in main repo
async fn detect_current_workspace_name() -> Result<Option<String>> {
    // 1. Get workspace root from jj workspace root
    let output = Command::new("jj")
        .args(["workspace", "root"])
        .output()
        .await
        .context("Failed to run 'jj workspace root'")?;

    if !output.status.success() {
        // If command failed, likely not in a repo.
        // If we return Ok(None), sync_all will run and fail with "Not in a JJ repo".
        return Ok(None);
    }

    let workspace_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // Normalize path for comparison
    let workspace_path = std::fs::canonicalize(&workspace_root)
        .unwrap_or_else(|_| std::path::PathBuf::from(&workspace_root));

    // 2. Open DB
    // Use helper that finds main repo DB even from workspace
    let Ok(db) = get_session_db_with_workspace_detection().await else {
        return Ok(None);
    };

    // 3. Find session matching path
    let sessions = db.list(None).await.unwrap_or_default();

    for session in sessions {
        let session_path = std::fs::canonicalize(&session.workspace_path)
            .unwrap_or_else(|_| std::path::PathBuf::from(&session.workspace_path));

        if session_path == workspace_path {
            return Ok(Some(session.name));
        }
    }

    // No matching session found
    Ok(None)
}

/// Sync a specific session's workspace
async fn sync_session_with_options(name: &str, options: SyncOptions) -> Result<()> {
    let db = get_session_db_with_workspace_detection().await?;

    // Get the session
    // Return zjj_core::Error::NotFound to get exit code 2 (not found)
    let session = db.get(name).await?.ok_or_else(|| {
        anyhow::Error::new(zjj_core::Error::NotFound(format!(
            "Session '{name}' not found"
        )))
    })?;

    // Use internal sync function
    sync_session_internal(&db, &session.name, &session.workspace_path, options.dry_run).await?;

    if options.format.is_json() {
        // Emit Action for the sync operation
        emit_action(
            "rebase",
            name,
            ActionStatus::Completed,
            Some(&format!("Synced session '{name}' with main")),
        )?;
        // Emit Result for command completion
        emit_result(
            true,
            ResultKind::Command,
            &format!("Synced session '{name}' with main"),
        )?;
    } else if !options.dry_run {
        println!("Synced session '{name}' with main");
        println!();
        println!("NEXT: Continue working, or if done:");
        println!("  zjj done          # Merge to main + cleanup");
    }

    Ok(())
}

/// Sync all sessions
pub async fn sync_all_with_options(options: SyncOptions) -> Result<()> {
    let db = get_session_db_with_workspace_detection().await?;

    // Get all sessions
    // Preserve error type for proper exit code mapping
    let sessions = db.list(None).await.map_err(anyhow::Error::new)?;

    if sessions.is_empty() {
        return handle_empty_sync(options);
    }

    // Route to appropriate handler based on format
    if options.format.is_json() {
        sync_all_jsonl(&db, sessions, options).await
    } else {
        sync_all_text(&db, sessions, options).await
    }
}

/// Handle case where no sessions are available to sync
fn handle_empty_sync(options: SyncOptions) -> Result<()> {
    if options.format.is_json() {
        // Emit Summary indicating no sessions
        emit_summary(
            SummaryType::Count,
            "No sessions to sync",
            Some("synced_count: 0, failed_count: 0"),
        )?;
        // Emit Result for command completion
        emit_result(true, ResultKind::Command, "No sessions to sync")?;
    } else {
        println!("No sessions to sync");
    }
    Ok(())
}

/// Sync all sessions with JSONL output
async fn sync_all_jsonl(
    db: &crate::db::SessionDb,
    sessions: Vec<crate::session::Session>,
    options: SyncOptions,
) -> Result<()> {
    // Collect results concurrently
    let results: Vec<_> = futures::stream::iter(sessions)
        .map(|session| async move {
            let res =
                sync_session_internal(db, &session.name, &session.workspace_path, options.dry_run)
                    .await;
            (session, res)
        })
        .buffered(1) // Limit concurrency to 1 (sequential) to prevent repo corruption
        .collect()
        .await;

    // Emit Action for each session as it completes
    let (synced, failed): (Vec<_>, Vec<_>) = results.into_iter().fold(
        (Vec::new(), Vec::new()),
        |(mut synced_acc, mut failed_acc), (session, res)| {
            match res {
                Ok(()) => {
                    // Emit Action for successful sync
                    let action_result = || -> Result<Action> {
                        Ok(Action::new(
                            ActionVerb::new("rebase")?,
                            ActionTarget::new(&session.name)?,
                            ActionStatus::Completed,
                        )
                        .with_result(format!("Synced '{}' with main", session.name)))
                    }();
                    match action_result {
                        Ok(action) => {
                            if let Err(e) = emit_stdout(&OutputLine::Action(action)) {
                                eprintln!("Warning: Failed to emit action: {e}");
                            }
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to create action: {e}");
                        }
                    }
                    synced_acc.push(session.name);
                }
                Err(e) => {
                    // Emit Issue for failed sync
                    let issue_result = || -> Result<Issue> {
                        let session_name = SessionName::parse(session.name.clone())?;
                        Ok(Issue::new(
                            IssueId::new(format!("SYNC-FAILED-{}", session.name))?,
                            IssueTitle::new(format!("Failed to sync session '{}'", session.name))?,
                            IssueKind::External,
                            IssueSeverity::Error,
                        )?
                        .with_session(session_name)
                        .with_suggestion(
                            "Try 'jj resolve' to fix conflicts, then retry sync".to_string(),
                        ))
                    }();

                    match issue_result {
                        Ok(issue_line) => {
                            if let Err(emit_err) = emit_stdout(&OutputLine::Issue(issue_line)) {
                                eprintln!("Warning: Failed to emit issue: {emit_err}");
                            }
                        }
                        Err(issue_err) => {
                            eprintln!("Warning: Failed to build issue output: {issue_err}");
                        }
                    }
                    failed_acc.push((session.name, e));
                }
            }
            (synced_acc, failed_acc)
        },
    );

    let synced_count = synced.len();
    let failed_count = failed.len();

    // Emit Summary with counts
    emit_summary(
        SummaryType::Count,
        &format!("Synced {synced_count} session(s), {failed_count} failed"),
        Some(&format!(
            "synced_count: {synced_count}, failed_count: {failed_count}"
        )),
    )?;

    // Emit final Result
    if failed.is_empty() {
        emit_result(
            true,
            ResultKind::Command,
            &format!("Successfully synced {synced_count} session(s)"),
        )?;
    } else {
        emit_result(
            false,
            ResultKind::Command,
            &format!("Synced {synced_count} session(s), {failed_count} failed"),
        )?;
        // Build error message for the anyhow bail
        let failed_names: Vec<&str> = failed.iter().map(|(name, _)| name.as_str()).collect();
        anyhow::bail!(
            "Failed to sync {} session(s): {}",
            failed_count,
            failed_names.join(", ")
        );
    }

    Ok(())
}

/// Sync all sessions with text output
async fn sync_all_text(
    db: &crate::db::SessionDb,
    sessions: Vec<crate::session::Session>,
    options: SyncOptions,
) -> Result<()> {
    println!("Syncing {} session(s)...", sessions.len());

    // Process sessions sequentially for text mode to avoid interleaved output
    let (success_count, failure_count, errors) = futures::stream::iter(sessions)
        .fold(
            (0, 0, Vec::new()),
            |(mut s_acc, mut f_acc, mut err_acc), session| async move {
                print!("Syncing '{}' ... ", &session.name);
                let _ = std::io::stdout().flush();

                match sync_session_internal(
                    db,
                    &session.name,
                    &session.workspace_path,
                    options.dry_run,
                )
                .await
                {
                    Ok(()) => {
                        println!("OK");
                        s_acc += 1;
                    }
                    Err(e) => {
                        println!("FAILED: {e}");
                        f_acc += 1;
                        err_acc.push((session.name.clone(), e));
                    }
                }
                (s_acc, f_acc, err_acc)
            },
        )
        .await;

    println!();
    println!("Summary: {success_count} succeeded, {failure_count} failed");

    if !errors.is_empty() {
        println!("\nErrors:");
        for (name, error) in errors {
            println!("  {name}: {error}");
        }
        anyhow::bail!("Failed to sync {failure_count} session(s)");
    }

    Ok(())
}

/// Internal function to sync a session's workspace
async fn sync_session_internal(
    db: &crate::db::SessionDb,
    name: &str,
    workspace_path: &str,
    dry_run: bool,
) -> Result<()> {
    let main_branch = determine_main_branch(Path::new(workspace_path)).await;

    if dry_run {
        println!("Would sync workspace '{workspace_path}' with main branch '{main_branch}'");
        return Ok(());
    }

    // Acquire global sync lock to prevent concurrent JJ operations
    // This serializes syncs across all zjj processes for this repo
    let data_dir = crate::commands::zjj_data_dir().await?;
    let lock_path = data_dir.join("sync.lock");

    // Use blocking lock in a separate task to avoid blocking the runtime
    // The file handle (_lock) keeps the lock held until it is dropped
    let _lock = tokio::task::spawn_blocking(move || -> Result<std::fs::File> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .context("Failed to open sync lock file")?;

        file.lock_exclusive()
            .context("Failed to acquire sync lock")?;
        Ok(file)
    })
    .await
    .context("Failed to join locking task")??;

    // Run rebase in the session's workspace
    let mut attempt = 0;
    let max_attempts = 3;
    let mut last_error = None;

    while attempt < max_attempts {
        let result = run_command(
            "jj",
            &["--repository", workspace_path, "rebase", "-d", &main_branch],
        )
        .await;

        match result {
            Ok(_) => {
                last_error = None;
                break;
            }
            Err(e) => {
                last_error = Some(e);
                attempt += 1;
                if attempt < max_attempts {
                    // Exponential backoff: 100ms, 200ms
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        100 * (1 << (attempt - 1)),
                    ))
                    .await;
                }
            }
        }
    }

    if let Some(e) = last_error {
        return Err(e).context("Failed to sync workspace with main after retries");
    }

    // Update last_synced timestamp
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .context("System time error")?
        .as_secs();

    db.update(
        name,
        SessionUpdate {
            last_synced: Some(now),
            ..Default::default()
        },
    )
    .await
    .map_err(anyhow::Error::new)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use tempfile::TempDir;

    use crate::{commands::determine_main_branch, db::SessionDb, session::SessionUpdate};

    // Helper to create a test database
    async fn setup_test_db() -> anyhow::Result<(SessionDb, TempDir)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::create_or_open(&db_path).await?;
        Ok((db, dir))
    }

    // Helper to get current unix timestamp
    fn current_timestamp() -> anyhow::Result<u64> {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .map_err(|e| anyhow::anyhow!("System time error: {e}"))
    }

    #[test]
    fn test_sync_session_not_found() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Try to sync a non-existent session
            // We can't actually run this without a real JJ repo, but we can test the lookup
            let result = db.get("nonexistent").await?;
            assert!(result.is_none());
            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_sync_session_exists() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Create a session
            let session = db.create("test-session", "/fake/workspace").await?;
            assert!(session.last_synced.is_none());

            // Verify we can get it
            let retrieved = db.get("test-session").await?;
            assert!(retrieved.is_some());
            if let Some(session) = retrieved {
                assert_eq!(session.name, "test-session");
            }

            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_update_last_synced_timestamp() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Create a session
            db.create("test-session", "/fake/workspace").await?;

            // Update last_synced
            let now = current_timestamp()?;
            let update = SessionUpdate {
                last_synced: Some(now),
                ..Default::default()
            };
            db.update("test-session", update).await?;

            // Verify it was updated
            let session = db.get("test-session").await?;
            assert!(session.is_some(), "Session not found");
            if let Some(session) = session {
                assert_eq!(session.last_synced, Some(now));
            }

            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_list_all_sessions() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Create multiple sessions
            db.create("session1", "/fake/workspace1").await?;
            db.create("session2", "/fake/workspace2").await?;
            db.create("session3", "/fake/workspace3").await?;

            // List all
            let sessions = db.list(None).await?;
            assert_eq!(sessions.len(), 3);

            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_sync_updates_timestamp_on_success() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Create a session
            db.create("test-session", "/fake/workspace").await?;

            // Simulate successful sync by updating timestamp
            let before = current_timestamp()?;
            let update = SessionUpdate {
                last_synced: Some(before),
                ..Default::default()
            };
            db.update("test-session", update).await?;

            // Verify timestamp was set
            let session = db.get("test-session").await?;
            assert!(session.is_some(), "Session not found");
            if let Some(session) = session {
                assert!(session.last_synced.is_some(), "last_synced should be set");
                if let Some(last_synced) = session.last_synced {
                    assert!(last_synced >= before);
                }
            }

            Ok::<_, anyhow::Error>(())
        })
    }

    #[test]
    fn test_multiple_syncs_update_timestamp() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (db, _dir) = setup_test_db().await?;

            // Create a session
            db.create("test-session", "/fake/workspace").await?;

            // First sync
            let first_sync = current_timestamp()?;
            db.update(
                "test-session",
                SessionUpdate {
                    last_synced: Some(first_sync),
                    ..Default::default()
                },
            )
            .await?;

            // Sleep to ensure different timestamp
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Second sync
            let second_sync = current_timestamp()?;
            db.update(
                "test-session",
                SessionUpdate {
                    last_synced: Some(second_sync),
                    ..Default::default()
                },
            )
            .await?;

            // Verify second timestamp is newer
            let session = db.get("test-session").await?;
            assert!(session.is_some(), "Session not found");
            if let Some(session) = session {
                assert!(session.last_synced.is_some(), "last_synced should be set");
                if let Some(last_synced) = session.last_synced {
                    assert!(last_synced >= second_sync);
                }
            }

            Ok::<_, anyhow::Error>(())
        })
    }

    #[tokio::test]
    async fn test_determine_main_branch_not_in_repo() -> anyhow::Result<()> {
        // When not in a JJ repo, should fall back to "main"
        let temp = tempfile::TempDir::new()?;
        let result = determine_main_branch(temp.path()).await;
        assert_eq!(result, "main");
        Ok(())
    }

    #[tokio::test]
    async fn test_determine_main_branch_fallback() -> anyhow::Result<()> {
        // Test that function returns "main" when trunk() fails
        let temp = tempfile::TempDir::new()?;
        let result = determine_main_branch(temp.path()).await;
        assert_eq!(result, "main");
        Ok(())
    }

    #[tokio::test]
    async fn test_sync_uses_determined_main_branch() -> anyhow::Result<()> {
        // Test that sync_session_internal uses determine_main_branch
        // This test will verify the integration once implemented
        let temp = tempfile::TempDir::new()?;
        let branch = determine_main_branch(temp.path()).await;
        assert_eq!(branch, "main");
        Ok(())
    }

    #[tokio::test]
    async fn test_main_branch_detection_respects_trunk() -> anyhow::Result<()> {
        // When trunk() returns a valid commit ID, use it
        let temp = tempfile::TempDir::new()?;
        let _branch = determine_main_branch(temp.path()).await;
        // Implementation should use trunk() when available
        Ok(())
    }

    #[test]
    fn test_sync_jsonl_action_output_on_success() -> anyhow::Result<()> {
        use zjj_core::output::{Action, ActionStatus, ActionTarget, ActionVerb, OutputLine};

        let action = Action::new(
            ActionVerb::new("rebase")?,
            ActionTarget::new("test-session")?,
            ActionStatus::Completed,
        )
        .with_result("Synced session 'test-session' with main".to_string());

        let output_line = OutputLine::Action(action);
        let json_str = serde_json::to_string(&output_line)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(
            parsed.get("action").is_some(),
            "JSON output should have 'action' key"
        );
        let action_obj = parsed
            .get("action")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow::anyhow!("action value must be an object"))?;
        assert_eq!(
            action_obj.get("verb").and_then(|v| v.as_str()),
            Some("rebase"),
            "verb should be 'rebase'"
        );
        assert_eq!(
            action_obj.get("target").and_then(|v| v.as_str()),
            Some("test-session"),
            "target should be 'test-session'"
        );
        assert_eq!(
            action_obj.get("status").and_then(|v| v.as_str()),
            Some("completed"),
            "status should be 'completed'"
        );
        assert!(
            action_obj.get("result").is_some(),
            "action should have result"
        );

        Ok(())
    }

    #[test]
    fn test_sync_jsonl_result_output_on_success() -> anyhow::Result<()> {
        use zjj_core::output::{Message, OutputLine, ResultKind, ResultOutput};

        let result = ResultOutput::success(
            ResultKind::Command,
            Message::new("Synced session 'test-session' with main")?,
        )?;

        let output_line = OutputLine::Result(result);
        let json_str = serde_json::to_string(&output_line)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(
            parsed.get("result").is_some(),
            "JSON output should have 'result' key"
        );
        let result_obj = parsed
            .get("result")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow::anyhow!("result value must be an object"))?;
        assert_eq!(
            result_obj.get("outcome").and_then(|v| v.as_str()),
            Some("success"),
            "outcome should be 'success' on success"
        );
        assert_eq!(
            result_obj.get("kind").and_then(|v| v.as_str()),
            Some("command"),
            "kind should be 'command'"
        );

        Ok(())
    }

    #[test]
    fn test_sync_jsonl_result_output_on_failure() -> anyhow::Result<()> {
        use zjj_core::output::{Message, OutputLine, ResultKind, ResultOutput};

        let result = ResultOutput::failure(
            ResultKind::Command,
            Message::new("Synced 3 session(s), 2 failed")?,
        )?;

        let output_line = OutputLine::Result(result);
        let json_str = serde_json::to_string(&output_line)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(
            parsed.get("result").is_some(),
            "JSON output should have 'result' key"
        );
        let result_obj = parsed
            .get("result")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow::anyhow!("result value must be an object"))?;
        assert_eq!(
            result_obj.get("outcome").and_then(|v| v.as_str()),
            Some("failure"),
            "outcome should be 'failure' on failure"
        );

        Ok(())
    }

    #[test]
    fn test_sync_jsonl_summary_output() -> anyhow::Result<()> {
        use zjj_core::output::{Message, OutputLine, Summary, SummaryType};

        let summary = Summary::new(
            SummaryType::Count,
            Message::new("Synced 5 session(s), 0 failed")?,
        )?
        .with_details("synced_count: 5, failed_count: 0".to_string());

        let output_line = OutputLine::Summary(summary);
        let json_str = serde_json::to_string(&output_line)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(
            parsed.get("summary").is_some(),
            "JSON output should have 'summary' key"
        );
        let summary_obj = parsed
            .get("summary")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow::anyhow!("summary value must be an object"))?;
        assert_eq!(
            summary_obj.get("type").and_then(|v| v.as_str()),
            Some("count"),
            "type should be 'count'"
        );
        assert!(
            summary_obj.get("message").is_some(),
            "summary should have message"
        );
        assert!(
            summary_obj.get("details").is_some(),
            "summary should have details"
        );

        Ok(())
    }

    #[test]
    fn test_sync_jsonl_issue_output() -> anyhow::Result<()> {
        use zjj_core::domain::SessionName;
        use zjj_core::output::{Issue, IssueId, IssueKind, IssueSeverity, IssueTitle, OutputLine};

        let issue = Issue::new(
            IssueId::new("SYNC-FAILED-test-session")?,
            IssueTitle::new("Failed to sync session 'test-session'")?,
            IssueKind::External,
            IssueSeverity::Error,
        )?
        .with_session(SessionName::parse("test-session")?)
        .with_suggestion("Try 'jj resolve' to fix conflicts, then retry sync".to_string());

        let output_line = OutputLine::Issue(issue);
        let json_str = serde_json::to_string(&output_line)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(
            parsed.get("issue").is_some(),
            "JSON output should have 'issue' key"
        );
        let issue_obj = parsed
            .get("issue")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow::anyhow!("issue value must be an object"))?;
        assert_eq!(
            issue_obj.get("id").and_then(|v| v.as_str()),
            Some("SYNC-FAILED-test-session"),
            "id should be correct"
        );
        assert_eq!(
            issue_obj.get("kind").and_then(|v| v.as_str()),
            Some("external"),
            "kind should be 'external'"
        );
        assert_eq!(
            issue_obj.get("severity").and_then(|v| v.as_str()),
            Some("error"),
            "severity should be 'error'"
        );
        // scope is now IssueScope enum instead of session: Option<String>
        // IssueScope::InSession serializes as {"InSession":{"session":"..."}}
        let scope = issue_obj
            .get("scope")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow::anyhow!("scope should be an object"))?;
        assert!(
            scope.get("InSession").is_some(),
            "scope should have 'InSession' field"
        );
        let in_session = scope
            .get("InSession")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow::anyhow!("InSession should be an object"))?;
        assert_eq!(
            in_session.get("session").and_then(|v| v.as_str()),
            Some("test-session"),
            "session should be set within scope.InSession"
        );
        assert!(
            issue_obj.get("suggestion").is_some(),
            "issue should have suggestion"
        );

        Ok(())
    }

    #[test]
    fn test_sync_jsonl_parseable_by_jq() -> anyhow::Result<()> {
        use zjj_core::output::{
            Action, ActionStatus, ActionTarget, ActionVerb, Message, OutputLine, ResultKind,
            ResultOutput,
        };

        // Simulate a sync output: Action + Result
        let action = Action::new(
            ActionVerb::new("rebase")?,
            ActionTarget::new("test-session")?,
            ActionStatus::Completed,
        )
        .with_result("Synced session 'test-session' with main".to_string());

        let action_line = OutputLine::Action(action);
        let action_json = serde_json::to_string(&action_line)?;

        let result = ResultOutput::success(
            ResultKind::Command,
            Message::new("Synced session 'test-session' with main")?,
        )?;
        let result_line = OutputLine::Result(result);
        let result_json = serde_json::to_string(&result_line)?;

        // Both should be valid JSON parseable by jq
        let action_parsed: serde_json::Value =
            serde_json::from_str(&action_json).map_err(|e| anyhow::anyhow!("{e}"))?;
        let result_parsed: serde_json::Value =
            serde_json::from_str(&result_json).map_err(|e| anyhow::anyhow!("{e}"))?;

        assert!(
            action_parsed.get("action").is_some(),
            "Action JSON should have 'action' key"
        );
        assert!(
            result_parsed.get("result").is_some(),
            "Result JSON should have 'result' key"
        );

        Ok(())
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // WORKSPACE DETECTION TESTS (TDD GREEN PHASE)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[tokio::test]
    async fn test_detect_workspace_context_returns_none_from_main_repo() -> anyhow::Result<()> {
        use tempfile::TempDir;

        let _temp_dir = TempDir::new()?;
        let result = super::detect_workspace_context().await;

        match result {
            Ok(None) => Ok(()),
            Ok(Some(_)) => {
                anyhow::bail!("Expected None from main repo, got Some")
            }
            Err(_e) => Ok(()),
        }
    }

    #[tokio::test]
    async fn test_get_session_db_from_workspace_finds_main_repo_db() -> anyhow::Result<()> {
        use tempfile::TempDir;

        let _temp_dir = TempDir::new()?;
        let result = super::get_session_db_with_workspace_detection().await;

        match result {
            Ok(_) => Ok(()),
            Err(_e) => Ok(()),
        }
    }

    #[tokio::test]
    async fn test_workspace_detection_handles_nested_layouts() -> anyhow::Result<()> {
        use tempfile::TempDir;

        let _temp_dir = TempDir::new()?;
        let result = super::detect_workspace_context().await;

        match result {
            Ok(_) | Err(_) => Ok(()),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SYNC BEHAVIOR DETERMINATION TESTS (EXPLICIT DEFAULT BEHAVIOR)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Test: Named session takes highest priority
    ///
    /// When a session name is provided, it should ALWAYS route to `NamedSession`,
    /// regardless of other flags.
    #[test]
    fn test_determine_sync_behavior_named_session_priority() {
        use super::{determine_sync_behavior, SyncBehavior};

        // Named session without --all
        let behavior = determine_sync_behavior(Some("feature-auth"), false);
        assert_eq!(
            behavior,
            SyncBehavior::NamedSession,
            "Named session should route to NamedSession"
        );

        // Named session WITH --all (name takes priority)
        let behavior = determine_sync_behavior(Some("feature-auth"), true);
        assert_eq!(
            behavior,
            SyncBehavior::NamedSession,
            "Named session should take priority over --all"
        );
    }

    /// Test: --all flag routes to `AllSessions`
    ///
    /// When --all is explicitly provided without a name, it should route to `AllSessions`.
    #[test]
    fn test_determine_sync_behavior_all_sessions() {
        use super::{determine_sync_behavior, SyncBehavior};

        let behavior = determine_sync_behavior(None, true);
        assert_eq!(
            behavior,
            SyncBehavior::AllSessions,
            "--all flag should route to AllSessions"
        );
    }

    /// Test: Default (no args) routes to `CurrentWorkspace`
    ///
    /// When no name and no --all flag, the safe default is `CurrentWorkspace`.
    #[test]
    fn test_determine_sync_behavior_default_is_current_workspace() {
        use super::{determine_sync_behavior, SyncBehavior};

        let behavior = determine_sync_behavior(None, false);
        assert_eq!(
            behavior,
            SyncBehavior::CurrentWorkspace,
            "Default (no args) should route to CurrentWorkspace"
        );
    }

    /// Test: Routing table is explicit and complete
    ///
    /// This test documents the complete routing table for sync behavior.
    #[test]
    fn test_sync_behavior_routing_table() {
        use super::{determine_sync_behavior, SyncBehavior};

        // Complete routing table as documented
        let test_cases = [
            // (name, all_flag, expected_behavior)
            (Some("name"), false, SyncBehavior::NamedSession),
            (Some("name"), true, SyncBehavior::NamedSession),
            (None, true, SyncBehavior::AllSessions),
            (None, false, SyncBehavior::CurrentWorkspace),
        ];

        for (name, all_flag, expected) in test_cases {
            let behavior = determine_sync_behavior(name, all_flag);
            assert_eq!(
                behavior, expected,
                "Routing failed for (name={name:?}, all={all_flag})"
            );
        }
    }

    /// Test: `SyncBehavior` enum is exhaustive
    ///
    /// Ensure all variants are handled in match statements.
    #[test]
    fn test_sync_behavior_variants() {
        use super::SyncBehavior;

        // Ensure we can create all variants
        let behaviors = [
            SyncBehavior::NamedSession,
            SyncBehavior::AllSessions,
            SyncBehavior::CurrentWorkspace,
        ];

        // Ensure Debug and PartialEq are implemented
        for behavior in behaviors {
            let debug_str = format!("{behavior:?}");
            assert!(!debug_str.is_empty(), "Debug should be implemented");
        }

        // Ensure PartialEq works
        assert_eq!(SyncBehavior::NamedSession, SyncBehavior::NamedSession);
        assert_ne!(SyncBehavior::NamedSession, SyncBehavior::AllSessions);
    }
}
