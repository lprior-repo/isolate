package cli

// Top-level spec wrapper required by intent
spec: {
	// Core behaviors for zjj CLI
	behaviors: {
	// Session management
	create_session: {
		description: "Create a new development session with JJ workspace and Zellij tab"
		trigger: "user invokes 'zjj add <name>'"
		command: "zjj add <name>"
		expected_result: "New workspace created, Zellij tab opened"
	}

	list_sessions: {
		description: "Display all sessions with their status"
		trigger: "user invokes 'zjj list'"
		command: "zjj list"
		expected_result: "Table of sessions shown"
	}

	remove_session: {
		description: "Remove session and cleanup workspace"
		trigger: "user invokes 'zjj remove <name>'"
		command: "zjj remove <name>"
		expected_result: "Workspace deleted, tab closed"
	}

	focus_session: {
		description: "Switch to session's Zellij tab"
		trigger: "user invokes 'zjj focus <name>'"
		command: "zjj focus <name>"
		expected_result: "Zellij switches to named tab"
	}

	sync_workspace: {
		description: "Sync session workspace with main via rebase"
		trigger: "user invokes 'zjj sync [name]'"
		command: "zjj sync"
		expected_result: "Workspace rebased onto main"
	}

	// Agent workflow
	spawn_workspace: {
		description: "Spawn isolated workspace for a bead and run agent"
		trigger: "user invokes 'zjj spawn <bead-id>'"
		command: "zjj spawn <bead-id>"
		expected_result: "New workspace created, agent started"
	}

	done_workspace: {
		description: "Complete work and merge workspace to main"
		trigger: "user invokes 'zjj done'"
		command: "zjj done"
		expected_result: "Work merged to main, workspace cleaned"
	}

	// Diagnostics
	doctor: {
		description: "Run system health checks"
		trigger: "user invokes 'zjj doctor'"
		command: "zjj doctor"
		expected_result: "Health report displayed"
	}

	dashboard: {
		description: "Launch interactive TUI dashboard"
		trigger: "user invokes 'zjj dashboard'"
		command: "zjj dashboard"
		expected_result: "Kanban TUI displayed"
	}
}

// Non-negotiable constraints
constraints: {
	error_handling: "THE SYSTEM SHALL NOT use unwrap(), expect(), or panic!"
	state_persistence: "THE SYSTEM SHALL persist state in SQLite"
	tab_naming: "Zellij tabs SHALL be named 'zjj:<session-name>'"
	sync_strategy: "Workspaces SHALL sync via rebase (jj rebase -d main)"
}

// External dependencies
dependencies: {
	jj: "Jujutsu version control (>=0.20)"
	zellij: "Terminal multiplexer (>=0.40)"
	moon: "Moon build system"
	beads: ".beads/beads.db for issue tracking"
	rust: "Rust 1.80 or later"
}

// Security considerations
security: {
	no_command_injection: "User input must be validated/escaped before shell execution"
	workspace_isolation: "Each workspace operates in isolated JJ context"
}
}  // close spec wrapper
