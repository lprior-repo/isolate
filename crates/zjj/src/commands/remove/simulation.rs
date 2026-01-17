//! Operation simulation and planning for dry-run

use crate::json_output::PlannedRemoveOperation;

/// Intermediate state for building operations
#[derive(Debug, Clone)]
pub struct OperationBuilder {
    operations: Vec<PlannedRemoveOperation>,
    warnings: Vec<String>,
    next_order: u32,
}

impl OperationBuilder {
    pub const fn new() -> Self {
        Self {
            operations: Vec::new(),
            warnings: Vec::new(),
            next_order: 1,
        }
    }

    pub fn add_operation(
        mut self,
        action: String,
        description: String,
        target: Option<String>,
    ) -> Self {
        self.operations.push(PlannedRemoveOperation {
            order: self.next_order,
            action,
            description,
            target,
            reversible: false,
        });
        self.next_order = self.next_order.saturating_add(1);
        self
    }

    pub fn add_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }

    pub fn add_operations(self, operations: Vec<PlannedRemoveOperation>) -> Self {
        // Functional fold: accumulate operations while tracking order
        operations.into_iter().fold(self, |mut builder, op| {
            builder.operations.push(PlannedRemoveOperation {
                order: builder.next_order,
                ..op
            });
            builder.next_order = builder.next_order.saturating_add(1);
            builder
        })
    }

    pub fn build(self) -> (Vec<PlannedRemoveOperation>, Option<Vec<String>>) {
        (
            self.operations,
            if self.warnings.is_empty() {
                None
            } else {
                Some(self.warnings)
            },
        )
    }
}

/// Build merge-related operations
pub fn build_merge_operations(workspace_path: &str) -> Vec<PlannedRemoveOperation> {
    vec![
        PlannedRemoveOperation {
            order: 0, // Will be renumbered by builder
            action: "squash_commits".to_string(),
            description: "Squash all commits in workspace".to_string(),
            target: Some(workspace_path.to_string()),
            reversible: false,
        },
        PlannedRemoveOperation {
            order: 0,
            action: "rebase_onto_main".to_string(),
            description: "Rebase squashed commit onto main branch".to_string(),
            target: Some(workspace_path.to_string()),
            reversible: false,
        },
        PlannedRemoveOperation {
            order: 0,
            action: "git_push".to_string(),
            description: "Push changes to remote".to_string(),
            target: Some(workspace_path.to_string()),
            reversible: false,
        },
        PlannedRemoveOperation {
            order: 0,
            action: "run_post_merge_hooks".to_string(),
            description: "Execute post_merge hooks from configuration".to_string(),
            target: Some(workspace_path.to_string()),
            reversible: false,
        },
    ]
}

/// Add workspace removal operation conditionally
pub fn add_workspace_removal(
    builder: OperationBuilder,
    workspace_exists: bool,
    workspace_path: &str,
) -> OperationBuilder {
    if workspace_exists {
        builder.add_operation(
            "remove_workspace_directory".to_string(),
            format!("Delete workspace directory at {workspace_path}"),
            Some(workspace_path.to_string()),
        )
    } else {
        builder
            .add_warning(format!(
                "Workspace directory does not exist: {workspace_path}"
            ))
            .add_operation(
                "skip_workspace_removal".to_string(),
                "Workspace directory already gone, skipping".to_string(),
                Some(workspace_path.to_string()),
            )
    }
}
