//! Add-batch command - create multiple sessions from stdin
//!
//! This module implements `zjj add-batch --beads-stdin` which creates
//! multiple sessions at once by reading bead IDs from stdin.
//!
//! # Workflow
//! 1. Read bead IDs from stdin (one per line)
//! 2. Validate ALL beads upfront (fail fast if any invalid)
//! 3. Create sessions sequentially (avoid workspace conflicts)
//! 4. Collect results and output (text or JSON)
//!
//! # Example
//! ```bash
//! bd ready | head -5 | zjj add-batch --beads-stdin --json
//! ```

use anyhow::{Context, Result};

use crate::{
    commands::{add, get_session_db},
    json_output::{AddBatchOutput, AddOutput, BatchItemResult},
};

/// Options for the add-batch command
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct AddBatchOptions {
    /// Read bead IDs from stdin
    pub beads_stdin: bool,
    /// Output as JSON
    pub json: bool,
    /// Don't open Zellij tabs (batch creation)
    pub no_open: bool,
    /// Don't run hooks
    pub no_hooks: bool,
    /// Template to use for all sessions
    pub template: Option<String>,
}

/// Run add-batch command
///
/// # Errors
/// Returns error if stdin reading, validation, or session creation fails
pub async fn run_with_options(options: &AddBatchOptions) -> Result<()> {
    if !options.beads_stdin {
        anyhow::bail!("--beads-stdin flag is required for add-batch command");
    }

    // Step 1: Read bead IDs from stdin
    let bead_ids = read_bead_ids_from_stdin();

    if bead_ids.is_empty() {
        output_no_beads(options.json)?;
        return Ok(());
    }

    // Step 2: Validate all beads upfront (fail fast)
    let repo_root = zjj_core::jj::check_in_jj_repo()?;
    let validated_beads = validate_all_beads(&repo_root, &bead_ids).await?;

    // Step 3: Create sessions sequentially
    let results = create_sessions_batch(&validated_beads, options).await;

    // Step 4: Output results
    output_batch_results(&results, options.json);

    // Exit with error code if any failed
    let failure_count = results.iter().filter(|r| !r.success).count();
    if failure_count > 0 {
        std::process::exit(2); // Exit code 2: System error (one or more creations failed)
    }

    Ok(())
}

/// Read bead IDs from stdin (one per line)
fn read_bead_ids_from_stdin() -> Vec<String> {
    use std::io::{self, BufRead};

    let stdin = io::stdin();
    stdin
        .lock()
        .lines()
        .map_while(Result::ok)
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

/// Validate all beads upfront (fail fast if any are invalid)
async fn validate_all_beads(
    repo_root: &std::path::Path,
    bead_ids: &[String],
) -> Result<Vec<zjj_core::beads::BeadIssue>> {
    let all_beads = zjj_core::beads::query_beads(repo_root)
        .await
        .context("Failed to query beads database")?;

    let mut validated = Vec::with_capacity(bead_ids.len());

    for bead_id in bead_ids {
        let bead = all_beads
            .iter()
            .find(|b| b.id == *bead_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Bead '{bead_id}' not found in .beads/beads.db"))?;

        validated.push(bead);
    }

    Ok(validated)
}

/// Create sessions for all validated beads (sequential to avoid workspace conflicts)
async fn create_sessions_batch(
    beads: &[zjj_core::beads::BeadIssue],
    options: &AddBatchOptions,
) -> Vec<BatchItemResult<AddOutput>> {
    let mut results = Vec::with_capacity(beads.len());

    for (index, bead) in beads.iter().enumerate() {
        // Generate session name from bead ID
        let session_name = bead.id.clone();

        // Create session using existing add command
        let add_options = add::AddOptions {
            name: session_name.clone(),
            no_hooks: options.no_hooks,
            template: options.template.clone(),
            no_open: options.no_open, // Batch creation: don't open tabs
            json: true,               // Always get structured output
            dry_run: false,
            bead: Some(bead.id.clone()),
            revision: None, // Batch creation uses default revision
        };

        match create_single_session(&add_options).await {
            Ok(output) => {
                results.push(BatchItemResult::success(session_name, index, output));
            }
            Err(e) => {
                results.push(BatchItemResult::failure(session_name, index, e.to_string()));
            }
        }
    }

    results
}

/// Create a single session and return structured output
async fn create_single_session(options: &add::AddOptions) -> Result<AddOutput> {
    // Get database connection
    let db = get_session_db().await?;

    // Load configuration
    let config = zjj_core::config::load_config()?;

    // Get repository root
    let repo_root = zjj_core::jj::check_in_jj_repo()?;

    // Build workspace path
    let workspace_path = repo_root.join(&config.workspace_dir).join(&options.name);
    let workspace_path_str = workspace_path
        .to_str()
        .context("Workspace path contains invalid UTF-8")?;

    // Run validations (use existing validation logic)
    add::validation::validate_all(&options.name, &db, &repo_root, options.no_open).await?;

    // Security validations
    add::security::validate_workspace_path(workspace_path_str, &repo_root, &config.workspace_dir)?;
    add::security::validate_no_symlinks(workspace_path_str, &repo_root)?;
    add::security::validate_workspace_dir(workspace_path_str)?;
    add::security::check_workspace_writable(workspace_path_str)?;

    // Create session (reuse existing workflow but extract result)
    // This is a simplified version - in production you'd want to extract the
    // create_session_workflow logic to avoid duplication
    add::run_with_options(options).await?;

    // Build success output
    Ok(AddOutput {
        success: true,
        session_name: options.name.clone(),
        workspace_path: workspace_path_str.to_string(),
        zellij_tab: format!("zjj:{}", options.name),
        status: "active".to_string(),
        error: None,
    })
}

/// Output message when no beads provided
fn output_no_beads(json: bool) -> Result<()> {
    if json {
        let output = AddBatchOutput::from_results(Vec::new());
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("No bead IDs provided on stdin");
    }
    Ok(())
}

/// Output batch creation results
fn output_batch_results(results: &[BatchItemResult<AddOutput>], json: bool) {
    if json {
        let output = AddBatchOutput::from_results(results.to_vec());
        if let Ok(json_str) = serde_json::to_string(&output) {
            println!("{json_str}");
        }
    } else {
        output_text_results(results);
    }
}

/// Output text results for batch creation
fn output_text_results(results: &[BatchItemResult<AddOutput>]) {
    let success_count = results.iter().filter(|r| r.success).count();
    let failure_count = results.iter().filter(|r| !r.success).count();

    println!("\nBatch session creation results:");
    println!("  Total: {}", results.len());
    println!("  Success: {success_count}");
    println!("  Failed: {failure_count}");

    if failure_count > 0 {
        println!("\nFailed sessions:");
        for result in results.iter().filter(|r| !r.success) {
            if let Some(error) = &result.error {
                println!("  - {}: {error}", result.item_id);
            }
        }
    }

    if success_count > 0 {
        println!("\nCreated sessions:");
        for result in results.iter().filter(|r| r.success) {
            println!("  ✓ {}", result.item_id);
        }
    }
}
