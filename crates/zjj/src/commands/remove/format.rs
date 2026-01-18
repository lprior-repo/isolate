//! Output formatting for dry-run plans

use anyhow::Result;

use crate::json_output::{RemoveDryRunOutput, RemoveDryRunPlan};

/// Output the dry-run plan in the requested format
pub fn output_plan(plan: &RemoveDryRunPlan, json: bool) -> Result<()> {
    let output = RemoveDryRunOutput {
        success: true,
        dry_run: true,
        plan,
    };

    if json {
        println!("{}", serde_json::to_string(&output)?);
    } else {
        output_human_readable(plan);
    }

    Ok(())
}

/// Output human-readable dry-run plan
fn output_human_readable(plan: &RemoveDryRunPlan) {
    println!("DRY RUN: The following operations would be performed:\n");
    println!("Session: {}", plan.session_name);
    println!("  ID: {}", plan.session_id);
    println!("  Workspace: {}", plan.workspace_path);
    println!("  Zellij Tab: {}", plan.zellij_tab);
    println!();

    println!("Planned Operations:");
    for op in &plan.planned_operations {
        let reversible_marker = if op.reversible { "" } else { " [IRREVERSIBLE]" };
        println!(
            "  {}. [{}] {}{}",
            op.order, op.action, op.description, reversible_marker
        );
    }

    if let Some(ref warns) = plan.warnings {
        println!("\nWarnings:");
        for w in warns {
            println!("  ⚠ {w}");
        }
    }

    println!("\nTo execute, run without --dry-run flag:");
    if plan.would_merge {
        println!("  zjj remove {} --merge", plan.session_name);
    } else {
        println!("  zjj remove {}", plan.session_name);
    }
}
