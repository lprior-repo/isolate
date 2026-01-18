// CUE schemas for ZJJ JSON output validation
// Use: cue eval -c schemas/output.cue data.json

package zjj

import (
	"strings"
	"list"
)

// Error detail structure - STANDARD across all commands
ErrorDetail :: {
	code: "VALIDATION_ERROR" | "NOT_FOUND" | "SYSTEM_ERROR" |
	      "INVALID_STATE" | "PERMISSION_ERROR" | "DATABASE_ERROR" |
	      "COMMAND_ERROR" | "HOOK_FAILED" | "DEPENDENCY_ERROR"
	message: string & strings.MinRunes(1)
	field?: string
	details?: {...}
}

// Session name validation - STANDARD across all commands
SessionName :: string & =~"^[a-zA-Z][a-zA-Z0-9_-]*$" & strings.MaxRunes(64)

// Base command output structure
CommandOutput :: {
	success: bool
	command?: string
	dry_run?: bool | *false
	timestamp?: string
	operation_id?: string
}

// ═══════════════════════════════════════════════════════════
// ADD COMMAND OUTPUT
// ═══════════════════════════════════════════════════════════

AddOutput :: CommandOutput & {
	success: bool
	session_name: SessionName
	workspace_path: string & strings.MinRunes(1)
	zellij_tab: string & strings.MinRunes(1)
	status: "creating" | "active" | "completed" | "failed"
	plan?: AddDryRunPlan  // Only if dry_run: true
	error?: ErrorDetail
}

AddDryRunPlan :: {
	session_name: SessionName
	workspace_path: string
	would_create_workspace: bool
	would_generate_layout: bool
	would_open_tab: bool
	would_run_hooks: bool
	template: string
	layout_content?: string
	estimated_time_seconds?: number
}

// ═══════════════════════════════════════════════════════════
// REMOVE COMMAND OUTPUT
// ═══════════════════════════════════════════════════════════

RemoveOutput :: CommandOutput & {
	success: bool
	session_name: SessionName  // NOT "session"
	operations?: [RemoveOperation, ...RemoveOperation] | []
	closed_bead?: string
	message?: string
	plan?: RemoveDryRunPlan  // Only if dry_run: true
	error?: ErrorDetail
}

RemoveOperation :: {
	action: string & ("close_tab" | "remove_workspace" | "delete_database" | "run_hook" | "merge_to_main")
	path?: string
	id?: number
	tab?: string
	success?: bool
}

RemoveDryRunPlan :: {
	session_name: SessionName
	session_id: number
	workspace_path: string
	workspace_exists: bool
	zellij_tab: string
	inside_zellij: bool
	would_run_hooks: bool
	would_merge: bool
	planned_operations: [PlannedRemoveOperation, ...PlannedRemoveOperation]
	warnings?: [string, ...string]
}

PlannedRemoveOperation :: {
	order: number & >=0
	action: string
	description: string
	target?: string
	reversible: bool
}

// ═══════════════════════════════════════════════════════════
// FOCUS COMMAND OUTPUT
// ═══════════════════════════════════════════════════════════

FocusOutput :: CommandOutput & {
	success: bool
	session_name: SessionName  // NOT "session"
	tab: string & strings.MinRunes(1)
	switched: bool
	error?: ErrorDetail
}

// ═══════════════════════════════════════════════════════════
// SYNC COMMAND OUTPUT
// ═══════════════════════════════════════════════════════════

SyncOutput :: CommandOutput & {
	success: bool
	session_name?: SessionName
	synced_count: number & >=0
	failed_count: number & >=0
	errors: [SyncError, ...SyncError] | []
	rebased_commits?: number & >=0
	conflicts?: number & >=0
	plan?: SyncDryRunPlan  // Only if dry_run: true
	error?: ErrorDetail
}

SyncError :: {
	session_name: SessionName
	error: string & strings.MinRunes(1)
}

SyncDryRunPlan :: {
	session_name?: SessionName
	sessions_to_sync: [SyncSessionPlan, ...SyncSessionPlan]
	target_branch: string
	target_branch_source: string
	total_count: number & >=0
	operations_per_session: [string, ...string]
}

SyncSessionPlan :: {
	name: SessionName
	workspace_path: string
	workspace_exists: bool
	status: string
	can_sync: bool
	skip_reason?: string
}

// ═══════════════════════════════════════════════════════════
// LIST COMMAND OUTPUT
// ═══════════════════════════════════════════════════════════

ListOutput :: CommandOutput & {
	success: bool
	sessions: [SessionItem, ...SessionItem] | []
	total_count: number & >=0
	active_count: number & >=0
}

SessionItem :: {
	name: SessionName
	status: "creating" | "active" | "completed" | "failed"
	workspace_path: string
	zellij_tab: string
	created_at: string
	updated_at: string
	bead_id?: string
	agent_id?: string
	uncommitted_changes?: number & >=0
	sync_status?: string
}

// ═══════════════════════════════════════════════════════════
// BATCH OPERATION OUTPUT
// ═══════════════════════════════════════════════════════════

BatchOperationOutput :: {
	success: bool
	operation: string
	total_count: number & >=0
	success_count: number & >=0
	failure_count: number & >=0
	partial_success: bool
	results: [BatchItemResult, ...BatchItemResult]
}

BatchItemResult :: {
	success: bool
	index: number & >=0
	item_id: string & strings.MinRunes(1)
	data?: {...}
	error?: ErrorDetail
}

// ═══════════════════════════════════════════════════════════
// DIFF COMMAND OUTPUT
// ═══════════════════════════════════════════════════════════

DiffOutput :: CommandOutput & {
	success: bool
	session_name: SessionName
	base: string
	head: string
	diff_stat?: DiffStat
	diff_content?: string
	error?: ErrorDetail
}

DiffStat :: {
	files_changed: number & >=0
	insertions: number & >=0
	deletions: number & >=0
	files: [FileDiffStat, ...FileDiffStat] | []
}

FileDiffStat :: {
	path: string & strings.MinRunes(1)
	insertions: number & >=0
	deletions: number & >=0
	status: "added" | "deleted" | "modified" | "renamed"
}

// ═══════════════════════════════════════════════════════════
// VALIDATION CONSTRAINTS
// ═══════════════════════════════════════════════════════════

#NonEmptyString :: string & strings.MinRunes(1)

#PathString :: string & strings.MinRunes(1) & =~"^[/~]|^[a-zA-Z]:" // Unix or Windows path

#SessionStatus :: "creating" | "active" | "completed" | "failed"

#ExitCodeMapping :: {
	0: "success"
	1: "user_error"
	2: "system_error"
	3: "not_found"
	4: "invalid_state"
}

// ═══════════════════════════════════════════════════════════
// INVARIANT CHECKS
// ═══════════════════════════════════════════════════════════

// These are logical invariants that should always hold:
// 1. If success: true, then error must be null/absent
// 2. If success: false, then error must be present with valid code
// 3. session_name (not session) used everywhere
// 4. All error codes must be from ErrorDetail.code enum
// 5. Batch operations must have partial_success = (success_count > 0 && failure_count > 0)
