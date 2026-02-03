package zjj

import (
	"list"
	"strings"
)

#Version: "1.0"

// Input request from AI via stdin
#InputRequest: {
	cmd:  #CommandName
	rid?: string // Optional request ID

	// Command-specific args (validated per command)
	...
}

// Universal response envelope
#ResponseEnvelope: {
	"$schema":       string
	_schema_version: #Version
	success:         bool

	if success {
		// Success data flattened here
		...
		next?: [...#NextAction]
		fixes: []
	}

	if !success {
		error: #ErrorDetail
		next?: [...#NextAction]
		fixes?: [...#Fix]
	}
}

#NextAction: {
	action: string & strings.MinRunes(1)
	commands: [...string] & list.MinItems(1)
}

#Fix: {
	description: string & strings.MinRunes(1)
	commands: [...string] & list.MinItems(1)
	rationale?: string
	automatic:  bool | *false
	impact?:    "low" | "medium" | "high"
}

#ErrorDetail: {
	code:        #ErrorCode
	message:     string & strings.MinRunes(1)
	details?:    string
	suggestion?: string
}

// State types
#StateResponse: #ResponseEnvelope & {
	success: true
	state: {
		sessions: [...#DetailedSession]
		agents: [...#ActiveAgent]
		checkpoints: [...#Checkpoint]
		system: #SystemState
		repo:   #RepoState
		beads:  #BeadsState
	}
	history_summary: {
		total_actions: int
		last_action:   #HistoryEntry
		patterns:      #DetectedPatterns
	}
}

#DetailedSession: #Session & {
	locks: [...string]
	last_action:  string
	last_touched: string
	health:       "good" | "warn" | "error"
	warnings: [...string]
}

#Session: {
	name:       #SessionName
	created_at: string // ISO 8601
	updated_at: string // ISO 8601
	status:     #SessionStatus
	template?:  "minimal" | "standard" | "full"
	bead?:      string

	// Allow additional fields for DetailedSession
	...
}

#SessionStatus: "active" | "inactive" | "locked" | "error"

#ActiveAgent: {
	id:             string
	name:           string
	status:         "running" | "paused" | "stopped"
	last_heartbeat: string // ISO 8601
	session:        #SessionName
}

#Checkpoint: {
	id:           string
	name:         string
	created_at:   string // ISO 8601
	description?: string
	state:        #StateSnapshot
}

#StateSnapshot: {
	sessions: [...#DetailedSession]
	agents: [...#ActiveAgent]
	checkpoints: [...#Checkpoint]
	system:    #SystemState
	repo:      #RepoState
	beads:     #BeadsState
	timestamp: string // ISO 8601
}

#SystemState: {
	version:      string
	uptime:       int
	memory_usage: int
	cpu_usage:    float
	disk_usage:   int
}

#RepoState: {
	path:    string
	branch:  string
	commits: int
	status:  "clean" | "dirty" | "untracked"
}

#BeadsState: {
	total_beads:     int
	completed_beads: int
	pending_beads:   int
	current_bead:    string
}

#HistoryResponse: #ResponseEnvelope & {
	success: true
	history: [...#HistoryEntry]
	summary: #HistorySummary
}

#HistoryEntry: {
	timestamp:   string // ISO 8601
	action:      string
	command:     #CommandName
	session?:    #SessionName
	status:      "success" | "failure"
	duration_ms: int
	details?:    string
}

#HistorySummary: {
	total_actions: int
	actions_by_command: {[string]: int}
	recent_errors: [...#ErrorDetail]
	patterns: #DetectedPatterns
}

#DetectedPatterns: {
	frequent_actions: [...string]
	session_patterns: [...#SessionPattern]
}

#SessionPattern: {
	session_name: #SessionName
	pattern_type: "frequent" | "unusual"
	actions: [...string]
}

#CommandName:
	// State reporting
	"state" | "history" | "diff-state" | "predict-data" |
	// Session management
	"init" | "add" | "remove" | "list" | "focus" | "status" |
	"sync" | "diff" | "merge" | "abandon" | "describe" | "log" |
	"exec" | "agent" | "link" | "unlink" |
	// Checkpoints
	"checkpoint" | "restore" | "list-checkpoints" |
	// Agent coordination
	"lock" | "unlock" | "agents" | "broadcast" |
	// Atomic operations
	"batch" |
	// Queue (future)
	"queue.add" | "queue.list" | "queue.run" | "queue.daemon" |
	// Config & introspection
	"config" | "introspect" | "context" | "doctor" | "query"

// Error codes grouped by exit code (35 total)
#ErrorCode:
	// Exit code 1: Validation errors
	"SESSION_NAME_INVALID" | "VALIDATION_ERROR" |
	"INVALID_ARGUMENT" | "CONFIG_PARSE_ERROR" |
	"CHECKPOINT_ALREADY_EXISTS" | "PARSE_ERROR" | "INVALID_CONFIG" |

	// Exit code 2: Not found errors
	"SESSION_NOT_FOUND" | "CHECKPOINT_NOT_FOUND" |
	"WORKSPACE_NOT_FOUND" | "CONFIG_NOT_FOUND" |
	"CONFIG_KEY_NOT_FOUND" | "AGENT_NOT_FOUND" |
	"BEAD_NOT_FOUND" | "NOT_FOUND" |

	// Exit code 3: System/state errors
	"NOT_INITIALIZED" | "STATE_DB_CORRUPTED" |
	"STATE_DB_LOCKED" | "DATABASE_ERROR" |
	"SESSION_LOCKED" | "LOCK_EXPIRED" |
	"SESSION_ALREADY_EXISTS" | "INTERNAL_ERROR" |

	// Exit code 4: External command/operation errors
	"JJ_NOT_INSTALLED" | "ZELLIJ_NOT_RUNNING" |
	"JJ_COMMAND_FAILED" | "NOT_JJ_REPOSITORY" |
	"WORKSPACE_CREATION_FAILED" | "ZELLIJ_COMMAND_FAILED" |
	"HOOK_FAILED" | "HOOK_EXECUTION_ERROR" |
	"COMMAND_ERROR" | "BATCH_FAILED" |
	"QUEUE_EMPTY" | "UNKNOWN"

// Session name constraint (max 64 chars total: 1 + 63)
#SessionName: =~"^[a-zA-Z][a-zA-Z0-9._-]{0,63}$"

// Add command specific types
#AddRequest: #InputRequest & {
	cmd:       "add"
	name:      #SessionName
	template?: "minimal" | "standard" | "full"
	no_open?:  bool
	bead?:     string
}

#AddResponse: #ResponseEnvelope & {
	success: true
	session: #DetailedSession
}

#RemoveRequest: #InputRequest & {
	cmd:  "remove"
	name: #SessionName
}

#RemoveResponse: #ResponseEnvelope & {
	success: true
	message: string
}

#StatusRequest: #InputRequest & {
	cmd:   "status"
	name?: #SessionName
}

#StatusResponse: #ResponseEnvelope & {
	success: true
	status:  #DetailedSession
}

#ListRequest: #InputRequest & {
	cmd: "list"
}

#ListResponse: #ResponseEnvelope & {
	success: true
	sessions: [...#DetailedSession]
}

#InitRequest: #InputRequest & {
	cmd:       "init"
	name:      #SessionName
	template?: "minimal" | "standard" | "full"
}

#InitResponse: #ResponseEnvelope & {
	success: true
	session: #DetailedSession
}

#SyncRequest: #InputRequest & {
	cmd:  "sync"
	name: #SessionName
}

#SyncResponse: #ResponseEnvelope & {
	success: true
	message: string
}

#HistoryRequest: #InputRequest & {
	cmd:      "history"
	session?: #SessionName
	limit?:   int
}

#DiffStateRequest: #InputRequest & {
	cmd:     "diff-state"
	before?: string // checkpoint ID or timestamp
	after?:  string // checkpoint ID or timestamp
}

// Additional implemented commands

#FocusRequest: #InputRequest & {
	cmd:  "focus"
	name: #SessionName
}

#FocusResponse: #ResponseEnvelope & {
	success:    true
	name:       string
	zellij_tab: string
	message:    string
}

// Diff types
#FileStatus: "M" | "A" | "D" | "R" | "?"

#FileDiffStat: {
	path:       string & strings.MinRunes(1)
	insertions: int & >=0
	deletions:  int & >=0
	status:     #FileStatus
}

#DiffSummary: {
	insertions:    int & >=0
	deletions:     int & >=0
	files_changed: int & >=0
	files: [...#FileDiffStat]

	// Cross-field constraint: files_changed must equal len(files)
	files_changed: len(files)
}

#DiffRequest: #InputRequest & {
	cmd:   "diff"
	name:  #SessionName
	stat?: bool
}

#DiffResponse: #ResponseEnvelope & {
	success:       true
	name:          string
	base:          string
	head:          string
	diff_stat?:    #DiffSummary
	diff_content?: string
}

#ConfigRequest: #InputRequest & {
	cmd:     "config"
	key?:    string
	value?:  string
	global?: bool
}

#ConfigResponse: #ResponseEnvelope & {
	success: true
	config?: {...} // Full config when no key specified
	key?:          string
	value?:        _ // Any type
}

#IntrospectRequest: #InputRequest & {
	cmd:      "introspect"
	command?: #CommandName
}

#IntrospectResponse: #ResponseEnvelope & {
	success:          true
	zjj_version:      string
	command_details?: #CommandIntrospection
}

#CommandIntrospection: {
	command:     string
	description: string
	aliases: [...string]
	arguments: [...string]
	flags: [...string]
	examples: [...string]
}

#DoctorRequest: #InputRequest & {
	cmd:  "doctor"
	fix?: bool
}

#DoctorResponse: #ResponseEnvelope & {
	success: true
	healthy: bool
	checks: [...#DoctorCheck]
	warnings: int & >=0
	errors:   int & >=0
}

#DoctorCheck: {
	name:        string
	status:      "pass" | "warn" | "fail"
	message:     string
	suggestion?: string
}

#QueryRequest: #InputRequest & {
	cmd:        "query"
	query_type: "session-exists" | "session-count" | "can-run" | "suggest-name"
	args: {...}
}

#QueryResponse: #ResponseEnvelope & {
	success: true
	result:  _ // Type varies by query_type
}

// Output types for structured response data

/// AddOutput represents the output data for add command
/// Wraps session creation result with metadata
#AddOutput: {
	name:           string & strings.MinRunes(1)
	workspace_path: string & strings.MinRunes(1)
	zellij_tab:     string & strings.MinRunes(1)
	status:         #SessionStatus
}

/// ListOutput represents the output data for list command
/// Contains array of sessions with summary information
#ListOutput: {
	sessions: [...#DetailedSession]
	count: int & >=0
	filter?: {
		bead?:  string
		agent?: string
	}
}

/// ErrorDetail represents detailed error information
/// Completes the partial definition at top of file
#ErrorDetail: {
	code:        #ErrorCode
	message:     string & strings.MinRunes(1)
	exit_code:   int & >=1 & <=4
	details?:    string
	suggestion?: string
}

// Beads integration types

/// IssueStatus represents the current state of a Beads issue
#IssueStatus: "open" | "in_progress" | "blocked" | "closed"

/// BeadsIssue represents a single issue in the Beads system
/// ID follows pattern: prefix-hash (e.g., zjj-fl0d)
#BeadsIssue: {
	id:          string & =~"^[a-z]+-[a-z0-9]{4,6}$" // Pattern: zjj-fl0d
	title:       string & strings.MinRunes(1)
	status:      #IssueStatus
	priority?:   int & >=0 & <=4
	issue_type?: "task" | "bug" | "feature" | "epic"
}

/// BeadsSummary represents aggregated counts of Beads issues
#BeadsSummary: {
	open:        int & >=0
	in_progress: int & >=0
	blocked:     int & >=0
	closed:      int & >=0
}

/// BeadStats represents statistics about Beads issues
/// Alias for BeadsSummary to maintain compatibility
#BeadStats: {
	open:        int & >=0
	in_progress: int & >=0
	blocked:     int & >=0
	closed:      int & >=0
}

/// SessionBeadsContext represents Beads issues relevant to a session
/// Used when creating or viewing session-specific issue context
#SessionBeadsContext: {
	available: [...#BeadsIssue]
	recommended: [...#BeadsIssue]
	counts: #BeadsSummary
}

// ============================================================================
// REMAINING COMMAND PAIRS (19 commands)
// ============================================================================

// --- State Commands ---

#StateRequest: #InputRequest & {
	cmd:      "state"
	session?: #SessionName
	include?: [...string] // Optional: filter which state sections to include
}

// StateResponse already defined above as #StateResponse

#DiffStateResponse: #ResponseEnvelope & {
	success: true
	before:  #StateSnapshot
	after:   #StateSnapshot
	changes: #StateDiff
}

#StateDiff: {
	sessions_added: [...#SessionName]
	sessions_removed: [...#SessionName]
	sessions_changed: [...#SessionChangeDiff]
	agents_changed:      bool
	checkpoints_changed: bool
}

#SessionChangeDiff: {
	name: #SessionName
	changes: [...string] // List of changed fields
}

#PredictDataRequest: #InputRequest & {
	cmd:      "predict-data"
	session?: #SessionName
	horizon?: int // How far ahead to predict
}

#PredictDataResponse: #ResponseEnvelope & {
	success: true
	predictions: [...#Prediction]
}

#Prediction: {
	type:       "session_activity" | "conflict" | "resource_usage"
	confidence: float & >=0 & <=1
	message:    string
	timestamp?: string // ISO 8601
}

// --- Session Management Commands ---

#MergeRequest: #InputRequest & {
	cmd:     "merge"
	name:    #SessionName
	target?: string // Branch to merge into, defaults to main
}

#MergeResponse: #ResponseEnvelope & {
	success:   true
	name:      string
	merged_to: string
	commits:   int & >=0
	message:   string
}

#AbandonRequest: #InputRequest & {
	cmd:    "abandon"
	name:   #SessionName
	force?: bool
}

#AbandonResponse: #ResponseEnvelope & {
	success: true
	name:    string
	message: string
}

#DescribeRequest: #InputRequest & {
	cmd:          "describe"
	name:         #SessionName
	description?: string
}

#DescribeResponse: #ResponseEnvelope & {
	success:     true
	name:        string
	description: string
}

#LogRequest: #InputRequest & {
	cmd:     "log"
	name:    #SessionName
	limit?:  int & >=1
	format?: "oneline" | "full" | "json"
}

#LogResponse: #ResponseEnvelope & {
	success: true
	name:    string
	commits: [...#CommitInfo]
}

#CommitInfo: {
	id:          string
	description: string
	author:      string
	timestamp:   string // ISO 8601
	change_id?:  string // JJ change ID
}

#ExecRequest: #InputRequest & {
	cmd:     "exec"
	name:    #SessionName
	command: string & strings.MinRunes(1)
	args?: [...string]
}

#ExecResponse: #ResponseEnvelope & {
	success:   true
	name:      string
	exit_code: int
	stdout:    string
	stderr:    string
}

#AgentRequest: #InputRequest & {
	cmd:      "agent"
	name:     #SessionName
	agent_id: string
	action:   "start" | "stop" | "status"
}

#AgentResponse: #ResponseEnvelope & {
	success:  true
	name:     string
	agent_id: string
	status:   "running" | "stopped" | "unknown"
}

#LinkRequest: #InputRequest & {
	cmd:    "link"
	name:   #SessionName
	target: string // What to link to (bead, agent, etc.)
}

#LinkResponse: #ResponseEnvelope & {
	success: true
	name:    string
	linked:  string
	message: string
}

#UnlinkRequest: #InputRequest & {
	cmd:    "unlink"
	name:   #SessionName
	target: string
}

#UnlinkResponse: #ResponseEnvelope & {
	success:  true
	name:     string
	unlinked: string
	message:  string
}

// --- Checkpoint Commands ---

#CheckpointRequest: #InputRequest & {
	cmd:          "checkpoint"
	name:         #SessionName
	checkpoint?:  string // Checkpoint name, auto-generated if omitted
	description?: string
}

#CheckpointResponse: #ResponseEnvelope & {
	success:       true
	session:       string
	checkpoint_id: string
	message:       string
}

#RestoreRequest: #InputRequest & {
	cmd:        "restore"
	name:       #SessionName
	checkpoint: string // Checkpoint ID or name
}

#RestoreResponse: #ResponseEnvelope & {
	success:       true
	session:       string
	checkpoint_id: string
	message:       string
}

#ListCheckpointsRequest: #InputRequest & {
	cmd:   "list-checkpoints"
	name?: #SessionName // If omitted, list all checkpoints
}

#ListCheckpointsResponse: #ResponseEnvelope & {
	success: true
	checkpoints: [...#Checkpoint]
}

// --- Agent Coordination Commands ---

#LockRequest: #InputRequest & {
	cmd:      "lock"
	name:     #SessionName
	holder:   string    // Who is holding the lock
	timeout?: int & >=0 // Lock timeout in seconds
	reason?:  string
}

#LockResponse: #ResponseEnvelope & {
	success:  true
	name:     string
	lock_id:  string
	holder:   string
	expires?: string // ISO 8601
}

#UnlockRequest: #InputRequest & {
	cmd:      "unlock"
	name:     #SessionName
	lock_id?: string // If omitted, unlock current lock
	force?:   bool
}

#UnlockResponse: #ResponseEnvelope & {
	success: true
	name:    string
	message: string
}

#AgentsRequest: #InputRequest & {
	cmd:      "agents"
	session?: #SessionName // If omitted, list all agents
}

#AgentsResponse: #ResponseEnvelope & {
	success: true
	agents: [...#ActiveAgent]
}

#BroadcastRequest: #InputRequest & {
	cmd:     "broadcast"
	message: string & strings.MinRunes(1)
	target?: [...#SessionName] // If omitted, broadcast to all
}

#BroadcastResponse: #ResponseEnvelope & {
	success:    true
	recipients: int & >=0
	message:    string
}

// --- Batch Command ---

#BatchRequest: #InputRequest & {
	cmd: "batch"
	operations: [...#BatchOperation] & list.MinItems(1)
	stop_on_error?: bool | *true
}

#BatchOperation: {
	command: #CommandName
	args: {...}
}

#BatchResponse: #ResponseEnvelope & {
	success: true
	results: [...#BatchResult]
	summary: {
		total:     int & >=0
		succeeded: int & >=0
		failed:    int & >=0
	}
}

#BatchResult: {
	index:   int & >=0
	command: #CommandName
	success: bool
	result?: _            // Command-specific result if success
	error?:  #ErrorDetail // Error if failure
}

// --- Context Command ---

#ContextRequest: #InputRequest & {
	cmd:      "context"
	session?: #SessionName
	include?: [...string] // Which context sections to include
}

#ContextResponse: #ResponseEnvelope & {
	success: true
	context: #UniversalContext
}

#UniversalContext: {
	session?: #DetailedSession
	repo:     #RepoState
	beads:    #SessionBeadsContext
	recent_files: [...string]
	suggestions: [...string]
}

// --- Queue Commands (Future) ---

#QueueAddRequest: #InputRequest & {
	cmd:     "queue.add"
	command: #CommandName
	args: {...}
	priority?: int & >=0 & <=4
	delay?:    int & >=0 // Delay in seconds
}

#QueueAddResponse: #ResponseEnvelope & {
	success:  true
	queue_id: string
	position: int & >=0
	message:  string
}

#QueueListRequest: #InputRequest & {
	cmd:     "queue.list"
	status?: "pending" | "running" | "completed" | "failed"
}

#QueueListResponse: #ResponseEnvelope & {
	success: true
	items: [...#QueueItem]
}

#QueueItem: {
	id:      string
	command: #CommandName
	args: {...}
	status:        "pending" | "running" | "completed" | "failed"
	priority:      int & >=0 & <=4
	created_at:    string // ISO 8601
	started_at?:   string
	completed_at?: string
	error?:        #ErrorDetail
}

#QueueRunRequest: #InputRequest & {
	cmd:       "queue.run"
	queue_id?: string // Run specific item, or next if omitted
}

#QueueRunResponse: #ResponseEnvelope & {
	success:  true
	queue_id: string
	result:   _ // Command result
}

#QueueDaemonRequest: #InputRequest & {
	cmd:    "queue.daemon"
	action: "start" | "stop" | "status"
}

#QueueDaemonResponse: #ResponseEnvelope & {
	success:    true
	status:     "running" | "stopped"
	processed?: int & >=0
	pending?:   int & >=0
}
