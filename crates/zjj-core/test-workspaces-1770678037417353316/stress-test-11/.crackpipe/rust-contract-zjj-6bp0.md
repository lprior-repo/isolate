# Rust Contract: zjj-6bp0 - Agent-to-Session Mapping Persistence

## Executive Summary

This bead addresses the **bidirectional** persistence and queryability of agent-to-session ownership. The codebase already persists `session → agent` (agents.current_session), but the inverse lookup (`agent → session`) and authoritative ownership enforcement are missing.

## Current State Analysis

### What Already Works
✅ **Agent registration stores session** (crates/zjj/src/commands/agents/mod.rs:295-307)
```sql
INSERT INTO agents (agent_id, registered_at, last_seen, current_session, ...)
VALUES (?, ?, ?, ?, ...)
ON CONFLICT(agent_id) DO UPDATE SET ... current_session = ?
```

✅ **AgentInfo includes current_session** (types.rs:102)
```rust
pub struct AgentInfo {
    pub current_session: Option<String>,
    ...
}
```

✅ **Query by agent ID works** (mod.rs:95-101)
```rust
SELECT agent_id, registered_at, last_seen, current_session, ...
FROM agents
WHERE agent_id = ?
```

### What's Missing (The Bug)
❌ **No inverse lookup**: Cannot query "which agent owns session X?"
❌ **No ownership enforcement**: Multiple agents could claim the same session
❌ **Context reads wrong source**: `extract_agent_from_metadata()` reads agent FROM session metadata, not authoritative agents table
❌ **No 1:1 invariant enforcement**: Agent could own multiple sessions, or session could have multiple owners

## Domain Model

### Core Concepts

1. **Agent**: Autonomous process working on beads (has unique ID)
2. **Session**: JJ workspace tracking a bead (has unique name)
3. **Ownership**: Bidirectional 1:1 relationship between agent and session
4. **Heartbeat**: Liveness signal (>60s = stale, loses ownership)

### Key Files
- `/home/lewis/src/zjj/crates/zjj/src/commands/agents/mod.rs` (agent registration, querying)
- `/home/lewis/src/zjj/crates/zjj/src/commands/agents/types.rs` (AgentInfo, RegisterArgs)
- `/home/lewis/src/zjj/crates/zjj/src/commands/context/mod.rs` (context query, line 254 reads agent)
- `/home/lewis/src/zjj/crates/zjj/src/commands/context/types.rs` (SessionContext with agent field)

## Contract Specification

### 1. Data Model

```rust
/// Agent ownership information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOwnership {
    pub agent_id: String,
    pub session_name: Option<String>,
    pub registered_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub actions_count: u64,
    pub is_stale: bool,
}
```

### 2. Preconditions

**For Agent Registration (`run_register`):**
- Agent ID must be non-empty (enforced by `validate_agent_id()`)
- Session name must be non-empty if provided
- Database must be initialized
- Agents table must exist

**For Context Query (`get_session_info`):**
- Must be in a workspace (not main branch)
- Session must exist in sessions table
- Database must be accessible

### 3. Postconditions

**After Successful Registration:**
- Agent record exists with `current_session` set
- Ownership is queryable by agent_id (already works)
- Ownership is queryable by session_name (MISSING - needs implementation)
- Session metadata reflects agent ownership (MIXED - reads from wrong source)

**After Context Query:**
- `SessionContext.agent` is authoritative (from agents table)
- Returns None if no active owner
- Returns error if session doesn't exist

### 4. Invariants

1. **Unidirectional Reference**: Agent → Session is stored (agents.current_session)
2. **Queryable Bidirectionally**: Can lookup agent→session AND session→agent
3. **Uniqueness**: Each session has at most one active agent (enforced by application logic, not DB)
4. **Authority**: agents table is source of truth (NOT session.metadata.agent_id)
5. **Persistence**: Survives database reconnections (SQLite durability)
6. **Staleness**: Agents >60s without heartbeat are stale but NOT auto-removed

### 5. Error Conditions

```rust
pub enum AgentOwnershipError {
    /// Agent ID is empty or whitespace-only
    InvalidAgentId,

    /// Session name provided but empty
    InvalidSessionName,

    /// Database connection/query failed
    DatabaseAccess(String),

    /// Agent not found in database
    AgentNotFound(String),

    /// Session not found in database
    SessionNotFound(String),

    /// Data invariant violated (should never happen)
    InvariantViolation(String),
}
```

### 6. API Contract

#### 6.1 Existing: `run_register` (Working)

```rust
/// Register agent and associate with session
///
/// # Pre
/// - Agent ID valid (non-empty)
/// - Session name valid if provided
///
/// # Post
/// - Agent record created/updated with current_session
/// - Ownership queryable by agent_id
///
/// # Errors
/// - InvalidAgentId
/// - DatabaseAccess
pub async fn run_register(args: &RegisterArgs, format: OutputFormat) -> Result<()>;
```

**Status**: ✅ Already implemented, works correctly

#### 6.2 Existing: `run_status` (Working)

```rust
/// Get agent status by ID
///
/// # Pre
/// - ZJJ_AGENT_ID environment variable set
///
/// # Post
/// - Returns AgentStatusOutput with agent info
/// - Includes current_session field
///
/// # Errors
/// - DatabaseAccess
pub async fn run_status(format: OutputFormat) -> Result<()>;
```

**Status**: ✅ Already implemented, works correctly

#### 6.3 NEW: `get_session_owner` (Missing)

```rust
/// Query which agent owns a given session
///
/// # Pre
/// - Session exists
/// - Database accessible
///
/// # Post
/// - Returns Some(agent_id) if session has active owner
/// - Returns None if session has no owner
/// - Filters out stale agents (>60s)
///
/// # Errors
/// - SessionNotFound
/// - DatabaseAccess
pub async fn get_session_owner(session: &str) -> Result<Option<String>>;
```

**Implementation**:
```sql
SELECT agent_id
FROM agents
WHERE current_session = ?
  AND last_seen > datetime('now', '-60 seconds')
LIMIT 1
```

**Status**: ❌ Missing - needs implementation

#### 6.4 FIX: `get_session_info` (Needs Fix)

```rust
/// Get session context including authoritative owner
///
/// # Pre
/// - In workspace (not main)
/// - Session exists in sessions table
///
/// # Post
/// - SessionContext.agent is from agents table (authoritative)
/// - Returns None if no active owner
/// - Filters out stale agents
///
/// # Errors
/// - SessionNotFound
/// - DatabaseAccess
pub async fn get_session_info() -> Result<SessionContext>;
```

**Current Implementation** (context/mod.rs:254):
```rust
let agent = extract_agent_from_metadata(session.metadata.as_ref());
```
❌ Reads from session.metadata (non-authoritative)

**Fixed Implementation**:
```rust
let agent = get_session_owner(&session.name).await?;
```
✅ Reads from agents table (authoritative)

**Status**: ⚠️ Needs fix - change data source

### 7. Database Schema

**Existing Columns** (already exist, no migration needed):
```sql
CREATE TABLE agents (
    agent_id TEXT PRIMARY KEY,
    registered_at TEXT NOT NULL,
    last_seen TEXT NOT NULL,
    current_session TEXT,        -- ✅ Already exists
    current_command TEXT,
    actions_count INTEGER DEFAULT 0
);
```

**New Index** (for performance):
```sql
CREATE INDEX IF NOT EXISTS idx_agents_current_session
ON agents(current_session)
WHERE current_session IS NOT NULL;
```

**Rationale**: Enables efficient `WHERE current_session = ?` queries for inverse lookup.

### 8. Testing Requirements

#### Unit Tests

```rust
#[cfg(test)]
mod ownership_tests {
    use super::*;

    #[test]
    fn test_register_stores_session() {
        // Register agent with session
        // Query agent by ID
        // Assert current_session is set correctly
    }

    #[test]
    fn test_get_session_owner_with_active_agent() {
        // Register agent with session
        // Query session owner
        // Assert returns Some(agent_id)
    }

    #[test]
    fn test_get_session_owner_with_stale_agent() {
        // Register agent with session
        // Set last_seen to 61 seconds ago
        // Query session owner
        // Assert returns None (agent is stale)
    }

    #[test]
    fn test_get_session_owner_no_agent() {
        // Query session without owner
        // Assert returns None
    }

    #[test]
    fn test_context_uses_authoritative_source() {
        // Register agent with session
        // Set session.metadata.agent_id to "different-agent"
        // Query context
        // Assert session.agent is from agents table, not metadata
    }

    #[test]
    fn test_multiple_agents_different_sessions() {
        // Register agent1 with session1
        // Register agent2 with session2
        // Query session1 owner → agent1
        // Query session2 owner → agent2
        // Assert no cross-contamination
    }
}
```

#### Integration Tests

```rust
#[tokio::test]
async fn test_full_ownership_lifecycle() {
    // 1. Register agent with session
    // 2. Verify ownership via get_session_owner
    // 3. Send heartbeat
    // 4. Verify ownership still active
    // 5. Unregister agent
    // 6. Verify ownership cleared (returns None)
}

#[tokio::test]
async fn test_context_command_integration() {
    // 1. Register agent with session in workspace
    // 2. Run zjj context --json
    // 3. Parse JSON output
    // 4. Assert session.agent matches registered agent_id
    // 5. Assert agent is from agents table (verify DB directly)
}
```

### 9. Implementation Phases

#### Phase 1: Add Inverse Lookup Function
**File**: `crates/zjj/src/commands/agents/mod.rs`

```rust
/// Get the agent ID that owns a given session
///
/// Returns None if session has no active owner (no agents or all stale)
pub async fn get_session_owner(
    pool: &SqlitePool,
    session: &str,
) -> Result<Option<String>> {
    let cutoff = Utc::now() - chrono::Duration::seconds(60);

    let agent_id: Option<String> = sqlx::query_scalar(
        "SELECT agent_id
         FROM agents
         WHERE current_session = ?
           AND last_seen > ?"
    )
    .bind(session)
    .bind(cutoff.to_rfc3339())
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("Failed to query session owner: {e}"))?;

    Ok(agent_id)
}
```

#### Phase 2: Fix Context Command
**File**: `crates/zjj/src/commands/context/mod.rs`

**Change line 254 from:**
```rust
let agent = extract_agent_from_metadata(session.metadata.as_ref());
```

**To:**
```rust
let agent = get_session_owner(&pool, &session.name).await
    .map_err(|e| anyhow::anyhow!("Failed to query session owner: {e}"))?;
```

**Note**: Must pass `pool` to `get_session_info()`

#### Phase 3: Add Database Index
**File**: `crates/zjj-core/src/database.rs` (or wherever migrations run)

```sql
CREATE INDEX IF NOT EXISTS idx_agents_current_session
ON agents(current_session)
WHERE current_session IS NOT NULL;
```

#### Phase 4: Add Tests
**File**: `crates/zjj/src/commands/agents/tests.rs`

Implement all unit tests from Section 8.

### 10. Verification Checklist

- [ ] `get_session_owner()` function implemented and returns `Result<Option<String>>`
- [ ] Function filters out stale agents (>60s)
- [ ] Context command uses `get_session_owner()` instead of metadata
- [ ] Index `idx_agents_current_session` created
- [ ] All unit tests pass (tests.rs)
- [ ] Integration test passes (full lifecycle)
- [ ] No unwrap/expect/panic (all errors via `Result`)
- [ ] Documentation updated with new function
- [ ] Performance: query <10ms with index

### 11. Open Questions

1. **Should ownership be exclusive?** (Can multiple agents own one session?)
   - **Decision**: No, enforce 1:1 in application logic (add validation in `run_register`)

2. **What happens when agent is stale?** (Ownership revoked or just marked stale?)
   - **Decision**: Marked stale but NOT revoked (manual cleanup only)

3. **Should we track ownership history?** (When did agent switch sessions?)
   - **Decision**: No, only current session (out of scope for this bead)

### 12. Migration Notes

**No schema migration required** - columns already exist.

**Database migration** (for performance index):
```bash
# Manual SQL execution
sqlite3 .zjj/state.db \
  "CREATE INDEX IF NOT EXISTS idx_agents_current_session
   ON agents(current_session)
   WHERE current_session IS NOT NULL;"
```

**Or via zjj** (if migration system exists):
```bash
zjj doctor --fix-indexes
```

---

## References

**Code Files**:
- `/home/lewis/src/zjj/crates/zjj/src/commands/agents/mod.rs` (registration, query logic)
- `/home/lewis/src/zjj/crates/zjj/src/commands/agents/types.rs` (type definitions)
- `/home/lewis/src/zjj/crates/zjj/src/commands/context/mod.rs` (context query, needs fix)
- `/home/lewis/src/zjj/crates/zjj/src/commands/context/types.rs` (SessionContext)

**Key Functions**:
- `run_register()` (agents/mod.rs:275) - stores session in agents table ✅
- `extract_agent_from_metadata()` (context/mod.rs:224) - reads from wrong source ❌
- `get_session_info()` (context/mod.rs:232) - needs to call get_session_owner ⚠️

**Bead**: zjj-6bp0
**Priority**: P3
**Effort**: 2hr
**Architect**: architect-1
**Generated**: 2026-02-08
