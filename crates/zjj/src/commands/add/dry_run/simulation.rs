//! Dry-run simulation logic for operation planning
//!
//! This module contains the core simulation logic for generating
//! planned operations without executing them. It separates the
//! simulation concerns from output formatting.

use std::path::Path;

use crate::commands::add::dry_run::PlannedOperation;
use zjj_core::config::Config;

/// Builder for constructing the list of planned operations
///
/// This struct encapsulates the context needed for building operations,
/// reducing parameter passing and improving code clarity.
pub struct OperationBuilder<'a> {
    session_name: &'a str,
    workspace_path: &'a str,
    template_name: &'a str,
    tab_name: &'a str,
    root: &'a Path,
    config: &'a Config,
    no_open: bool,
    no_hooks: bool,
    bead: Option<&'a str>,
}

impl<'a> OperationBuilder<'a> {
    /// Create a new operation builder with the given parameters
    pub fn new(
        session_name: &'a str,
        workspace_path: &'a str,
        template_name: &'a str,
        tab_name: &'a str,
        root: &'a Path,
        config: &'a Config,
        no_open: bool,
        no_hooks: bool,
        bead: Option<&'a str>,
    ) -> Self {
        Self {
            session_name,
            workspace_path,
            template_name,
            tab_name,
            root,
            config,
            no_open,
            no_hooks,
            bead,
        }
    }

    /// Build the complete sequence of planned operations
    pub fn build(self) -> im::Vector<PlannedOperation> {
        // Functional approach: chain base operations with conditional operations
        let base_operations = self.build_base_operations();

        let open_tab_operation = self.build_open_tab_operation();
        let hook_operation = self.build_hook_operation();
        let final_operation = self.build_final_operation();
        let bead_operations = self.build_bead_operations();

        base_operations
            .into_iter()
            .chain(open_tab_operation)
            .chain(hook_operation)
            .chain(std::iter::once(final_operation))
            .chain(bead_operations.into_iter().flatten())
            .collect()
    }

    /// Build base operations that always occur
    fn build_base_operations(&self) -> [PlannedOperation; 3] {
        [
            PlannedOperation {
                action: "create_database_entry".to_string(),
                target: "sessions table".to_string(),
                details: Some(format!("name={}, status=creating", self.session_name)),
            },
            PlannedOperation {
                action: "create_jj_workspace".to_string(),
                target: self.workspace_path.to_string(),
                details: Some(format!("jj workspace add --name {}", self.session_name)),
            },
            PlannedOperation {
                action: "generate_layout".to_string(),
                target: format!(
                    "{}/{}/layouts/{}.kdl",
                    self.root.display(),
                    self.config.workspace_dir,
                    self.session_name
                ),
                details: Some(format!("template={}", self.template_name)),
            },
        ]
    }

    /// Build optional open_zellij_tab operation
    fn build_open_tab_operation(&self) -> Option<PlannedOperation> {
        (!self.no_open).then_some(PlannedOperation {
            action: "open_zellij_tab".to_string(),
            target: self.tab_name.to_string(),
            details: Some("zellij action new-tab --layout <layout_file>".to_string()),
        })
    }

    /// Build optional run_hook operation
    fn build_hook_operation(&self) -> Option<PlannedOperation> {
        (!self.no_hooks && !self.config.hooks.post_create.is_empty()).then_some(PlannedOperation {
            action: "run_hook".to_string(),
            target: "post_create".to_string(),
            details: Some(self.config.hooks.post_create.join("; ")),
        })
    }

    /// Build final database update operation
    fn build_final_operation(&self) -> PlannedOperation {
        PlannedOperation {
            action: "update_database_entry".to_string(),
            target: "sessions table".to_string(),
            details: Some(format!("name={}, status=active", self.session_name)),
        }
    }

    /// Build optional bead-related operations
    fn build_bead_operations(&self) -> Option<[PlannedOperation; 4]> {
        self.bead.map(|bead_id| {
            [
                PlannedOperation {
                    action: "validate_bead".to_string(),
                    target: bead_id.to_string(),
                    details: Some("Check bead exists in .beads/beads.db".to_string()),
                },
                PlannedOperation {
                    action: "store_bead_metadata".to_string(),
                    target: "session metadata".to_string(),
                    details: Some(format!("bead_id={bead_id}")),
                },
                PlannedOperation {
                    action: "write_bead_spec".to_string(),
                    target: format!("{}/BEAD_SPEC.md", self.workspace_path),
                    details: Some("Generate BEAD_SPEC.md from bead details".to_string()),
                },
                PlannedOperation {
                    action: "update_bead_status".to_string(),
                    target: bead_id.to_string(),
                    details: Some("bd update --status in_progress".to_string()),
                },
            ]
        })
    }
}
