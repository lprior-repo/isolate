//! Dry-run planning and orchestration

use std::path::Path;

use anyhow::Result;

use super::RemoveOptions;
use crate::{cli::is_inside_zellij, json_output::RemoveDryRunPlan, session::Session};

mod format;
mod simulation;

use format::output_plan as format_output_plan;
use simulation::{add_workspace_removal, build_merge_operations, OperationBuilder};

/// Output the dry-run plan (re-exported from format module)
pub fn output(plan: &RemoveDryRunPlan, json: bool) -> Result<()> {
    format_output_plan(plan, json)
}

/// Build a dry-run plan showing what would be done
pub fn build_plan(
    name: &str,
    session: &Session,
    options: &RemoveOptions,
) -> Result<RemoveDryRunPlan> {
    let inside_zellij = is_inside_zellij();
    let workspace_exists = Path::new(&session.workspace_path).exists();
    let would_run_hooks = !options.force;

    let session_id = session
        .id
        .ok_or_else(|| anyhow::anyhow!("Session missing database ID - data corruption"))?;

    // Functional pipeline: build operations immutably
    let builder = OperationBuilder::new();

    // Step 1: Zellij tab close (if inside Zellij)
    let builder = if inside_zellij {
        builder.add_operation(
            "close_zellij_tab".to_string(),
            format!("Close Zellij tab '{}'", session.zellij_tab),
            Some(session.zellij_tab.clone()),
        )
    } else {
        builder
    };

    // Step 2: Pre-remove hooks (if not --force)
    let builder = if would_run_hooks {
        builder.add_operation(
            "run_pre_remove_hooks".to_string(),
            "Execute pre_remove hooks from configuration".to_string(),
            Some(session.workspace_path.clone()),
        )
    } else {
        builder
    };

    // Step 3: Merge to main (if --merge)
    let builder = if options.merge {
        builder.add_operations(build_merge_operations(&session.workspace_path))
    } else {
        builder
    };

    // Step 4: Remove workspace directory
    let builder = add_workspace_removal(builder, workspace_exists, &session.workspace_path);

    // Step 5: Forget JJ workspace
    let builder = builder.add_operation(
        "forget_jj_workspace".to_string(),
        format!("Remove JJ workspace registration for '{name}'"),
        Some(name.to_string()),
    );

    // Step 6: Delete database entry
    let builder = builder.add_operation(
        "delete_db_entry".to_string(),
        format!("Remove session '{name}' from database (id: {session_id})"),
        Some(session_id.to_string()),
    );

    let (operations, warnings) = builder.build();

    Ok(RemoveDryRunPlan {
        session_name: name.to_string(),
        session_id,
        workspace_path: session.workspace_path.clone(),
        workspace_exists,
        zellij_tab: session.zellij_tab.clone(),
        inside_zellij,
        would_run_hooks,
        would_merge: options.merge,
        planned_operations: operations,
        warnings,
    })
}
