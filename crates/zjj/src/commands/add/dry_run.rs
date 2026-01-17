//! Dry-run simulation for add command
//!
//! This module provides dry-run functionality (zjj-gyr) that validates
//! the add command parameters and outputs a detailed plan of operations
//! that would be performed, without actually executing any changes.
//!
//! The module is organized into:
//! - Public types and API (this file)
//! - Simulation logic (simulation.rs) - operation building and planning

mod simulation;

use std::path::Path;

use anyhow::Result;
use serde::Serialize;
use zjj_core::config::Config;

use self::simulation::OperationBuilder;

/// Dry-run output for add command (zjj-gyr)
#[derive(Debug, Serialize)]
pub struct AddDryRunOutput {
    pub success: bool,
    pub dry_run: bool,
    pub plan: AddDryRunPlan,
}

/// Plan of what would happen during add
#[derive(Debug, Serialize)]
pub struct AddDryRunPlan {
    pub session_name: String,
    pub workspace_path: String,
    pub branch: String,
    pub layout_template: String,
    pub zellij_tab_name: String,
    pub will_open_zellij: bool,
    pub will_run_hooks: bool,
    pub operations: im::Vector<PlannedOperation>,
}

/// A single planned operation
#[derive(Debug, Clone, Serialize)]
pub struct PlannedOperation {
    pub action: String,
    pub target: String,
    pub details: Option<String>,
}

/// Parameters for generating a dry-run plan
#[derive(Debug)]
pub struct DryRunParams<'a> {
    pub session_name: &'a str,
    pub workspace_path: &'a str,
    pub root: &'a Path,
    pub config: &'a Config,
    pub template: Option<&'a str>,
    pub no_open: bool,
    pub no_hooks: bool,
    pub bead: Option<&'a str>,
}

/// Generate a dry-run plan showing what operations would be performed
///
/// This function creates a detailed plan of all operations that would occur
/// during session creation, without actually executing any of them.
pub fn generate_plan(params: &DryRunParams<'_>) -> AddDryRunPlan {
    let template_name = params
        .template
        .map(ToString::to_string)
        .unwrap_or_else(|| params.config.default_template.clone());

    let tab_name = format!("jjz:{}", params.session_name);

    let builder = OperationBuilder::new(
        params.session_name,
        params.workspace_path,
        &template_name,
        &tab_name,
        params.root,
        params.config,
        params.no_open,
        params.no_hooks,
        params.bead,
    );

    let operations = builder.build();

    AddDryRunPlan {
        session_name: params.session_name.to_string(),
        workspace_path: params.workspace_path.to_string(),
        branch: params.session_name.to_string(),
        layout_template: template_name,
        zellij_tab_name: tab_name,
        will_open_zellij: !params.no_open,
        will_run_hooks: !params.no_hooks,
        operations,
    }
}

/// Print dry-run plan in human-readable format
pub fn print_plan(plan: &AddDryRunPlan) {
    println!("DRY RUN - No changes will be made\n");
    println!("Session: {}", plan.session_name);
    println!("Workspace: {}", plan.workspace_path);
    println!("Branch: {}", plan.branch);
    println!("Template: {}", plan.layout_template);
    println!("Zellij Tab: {}", plan.zellij_tab_name);
    println!();

    println!("Planned Operations:");
    plan.operations.iter().enumerate().for_each(|(i, op)| {
        println!("  {}. {} → {}", i.saturating_add(1), op.action, op.target);
        if let Some(ref details) = op.details {
            println!("     {details}");
        }
    });
    println!();

    println!("Flags:");
    println!(
        "  Will open Zellij tab: {}",
        if plan.will_open_zellij { "yes" } else { "no" }
    );
    println!(
        "  Will run hooks: {}",
        if plan.will_run_hooks { "yes" } else { "no" }
    );
}

/// Execute dry-run and output results
///
/// # Errors
/// Returns error if dry-run execution fails
pub fn execute(params: &DryRunParams<'_>, json_output: bool) -> Result<()> {
    let plan = generate_plan(params);

    let output = AddDryRunOutput {
        success: true,
        dry_run: true,
        plan,
    };

    if json_output {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_plan(&output.plan);
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use std::path::PathBuf;

    use zjj_core::config::Config;

    use super::*;

    fn create_test_config() -> Config {
        Config {
            workspace_dir: ".jjz".to_string(),
            default_template: "default".to_string(),
            hooks: zjj_core::config::HooksConfig {
                post_create: vec!["echo 'created'".to_string()],
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_generate_plan_basic() {
        let config = create_test_config();
        let root = PathBuf::from("/test/root");

        let params = DryRunParams {
            session_name: "test-session",
            workspace_path: "/test/root/.jjz/test-session",
            root: &root,
            config: &config,
            template: None,
            no_open: false,
            no_hooks: false,
            bead: None,
        };

        let plan = generate_plan(&params);
        assert_eq!(plan.session_name, "test-session");
        assert_eq!(plan.workspace_path, "/test/root/.jjz/test-session");
        assert_eq!(plan.branch, "test-session");
        assert_eq!(plan.layout_template, "default");
        assert_eq!(plan.zellij_tab_name, "jjz:test-session");
        assert!(plan.will_open_zellij);
        assert!(plan.will_run_hooks);
        assert_eq!(plan.operations.len(), 6); // create_db, create_workspace, generate_layout,
                                              // open_tab, run_hook, update_db
    }

    #[test]
    fn test_generate_plan_with_custom_template() {
        let config = create_test_config();
        let root = PathBuf::from("/test/root");

        let params = DryRunParams {
            session_name: "test-session",
            workspace_path: "/test/root/.jjz/test-session",
            root: &root,
            config: &config,
            template: Some("custom"),
            no_open: false,
            no_hooks: false,
            bead: None,
        };

        let plan = generate_plan(&params);
        assert_eq!(plan.layout_template, "custom");
    }

    #[test]
    fn test_generate_plan_no_open() {
        let config = create_test_config();
        let root = PathBuf::from("/test/root");

        let params = DryRunParams {
            session_name: "test-session",
            workspace_path: "/test/root/.jjz/test-session",
            root: &root,
            config: &config,
            template: None,
            no_open: true,
            no_hooks: false,
            bead: None,
        };

        let plan = generate_plan(&params);
        assert!(!plan.will_open_zellij);
        // Should have 5 operations: create_db, create_workspace, generate_layout, run_hook,
        // update_db (no open_tab)
        assert_eq!(plan.operations.len(), 5);
        assert!(plan
            .operations
            .iter()
            .all(|op| op.action != "open_zellij_tab"));
    }

    #[test]
    fn test_generate_plan_no_hooks() {
        let config = create_test_config();
        let root = PathBuf::from("/test/root");

        let params = DryRunParams {
            session_name: "test-session",
            workspace_path: "/test/root/.jjz/test-session",
            root: &root,
            config: &config,
            template: None,
            no_open: false,
            no_hooks: true,
            bead: None,
        };

        let plan = generate_plan(&params);
        assert!(!plan.will_run_hooks);
        // Should have 5 operations: create_db, create_workspace, generate_layout, open_tab,
        // update_db (no run_hook)
        assert_eq!(plan.operations.len(), 5);
        assert!(plan.operations.iter().all(|op| op.action != "run_hook"));
    }

    #[test]
    fn test_generate_plan_no_open_no_hooks() {
        let config = create_test_config();
        let root = PathBuf::from("/test/root");

        let params = DryRunParams {
            session_name: "test-session",
            workspace_path: "/test/root/.jjz/test-session",
            root: &root,
            config: &config,
            template: None,
            no_open: true,
            no_hooks: true,
            bead: None,
        };

        let plan = generate_plan(&params);
        assert!(!plan.will_open_zellij);
        assert!(!plan.will_run_hooks);
        // Should have 4 operations: create_db, create_workspace, generate_layout, update_db
        assert_eq!(plan.operations.len(), 4);
    }
}
