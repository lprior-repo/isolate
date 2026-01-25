package zjj

#Version: "1.0"

// Input request from AI via stdin
#InputRequest: {
    cmd: #CommandName
    rid?: string  // Optional request ID
    
    // Command-specific args (validated per command)
    ...
}

// Universal response envelope
#ResponseEnvelope: {
    "$schema": string
    _schema_version: #Version
    success: bool
    
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
    automatic: bool | *false
    impact?: "low" | "medium" | "high"
}

#ErrorDetail: {
    code: #ErrorCode
    message: string & strings.MinRunes(1)
    details?: string
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
        repo: #RepoState
        beads: #BeadsState
    }
    history_summary: {
        total_actions: int
        last_action: #HistoryEntry
        patterns: #DetectedPatterns
    }
}

#DetailedSession: #Session & {
    locks: [...string]
    last_action: string
    last_touched: string
    health: "good" | "warn" | "error"
    warnings: [...string]
}

#Session: {
    name: #SessionName
    created_at: string  // ISO 8601
    updated_at: string  // ISO 8601
    status: #SessionStatus
    template?: "minimal" | "standard" | "full"
    bead?: string
}

#SessionStatus: "active" | "inactive" | "locked" | "error"

#ActiveAgent: {
    id: string
    name: string
    status: "running" | "paused" | "stopped"
    last_heartbeat: string  // ISO 8601
    session: #SessionName
}

#Checkpoint: {
    id: string
    name: string
    created_at: string  // ISO 8601
    description?: string
    state: #StateSnapshot
}

#StateSnapshot: {
    sessions: [...#DetailedSession]
    agents: [...#ActiveAgent]
    checkpoints: [...#Checkpoint]
    system: #SystemState
    repo: #RepoState
    beads: #BeadsState
    timestamp: string  // ISO 8601
}

#SystemState: {
    version: string
    uptime: int
    memory_usage: int
    cpu_usage: float
    disk_usage: int
}

#RepoState: {
    path: string
    branch: string
    commits: int
    status: "clean" | "dirty" | "untracked"
}

#BeadsState: {
    total_beads: int
    completed_beads: int
    pending_beads: int
    current_bead: string
}

#HistoryResponse: #ResponseEnvelope & {
    success: true
    history: [...#HistoryEntry]
    summary: #HistorySummary
}

#HistoryEntry: {
    timestamp: string  // ISO 8601
    action: string
    command: #CommandName
    session?: #SessionName
    status: "success" | "failure"
    duration_ms: int
    details?: string
}

#HistorySummary: {
    total_actions: int
    actions_by_command: map[string]int
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

#ErrorCode:
    "SESSION_NOT_FOUND" | "SESSION_ALREADY_EXISTS" | 
    "SESSION_NAME_INVALID" | "NOT_INITIALIZED" |
    "JJ_NOT_INSTALLED" | "ZELLIJ_NOT_RUNNING" |
    "STATE_DB_CORRUPTED" | "CHECKPOINT_NOT_FOUND" |
    "SESSION_LOCKED" | "LOCK_EXPIRED" | "BATCH_FAILED" |
    "VALIDATION_ERROR" | "INTERNAL_ERROR"

// Session name constraint
#SessionName: =~"^[a-zA-Z][a-zA-Z0-9._-]{0,254}$"

// Add command specific types
#AddRequest: #InputRequest & {
    cmd: "add"
    name: #SessionName
    template?: "minimal" | "standard" | "full"
    no_open?: bool
    bead?: string
}

#AddResponse: #ResponseEnvelope & {
    success: true
    session: #DetailedSession
}

#RemoveRequest: #InputRequest & {
    cmd: "remove"
    name: #SessionName
}

#RemoveResponse: #ResponseEnvelope & {
    success: true
    message: string
}

#StatusRequest: #InputRequest & {
    cmd: "status"
    name?: #SessionName
}

#StatusResponse: #ResponseEnvelope & {
    success: true
    status: #DetailedSession
}

#ListRequest: #InputRequest & {
    cmd: "list"
}

#ListResponse: #ResponseEnvelope & {
    success: true
    sessions: [...#DetailedSession]
}

#InitRequest: #InputRequest & {
    cmd: "init"
    name: #SessionName
    template?: "minimal" | "standard" | "full"
}

#InitResponse: #ResponseEnvelope & {
    success: true
    session: #DetailedSession
}

#SyncRequest: #InputRequest & {
    cmd: "sync"
    name: #SessionName
}

#SyncResponse: #ResponseEnvelope & {
    success: true
    message: string
}

#HistoryRequest: #InputRequest & {
    cmd: "history"
    session?: #SessionName
    limit?: int
}

#DiffStateRequest: #InputRequest & {
    cmd: "diff-state"
    before?: string  // checkpoint ID or timestamp
    after?: string   // checkpoint ID or timestamp
}