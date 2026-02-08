//! Claim/Yield commands - Multi-agent resource locking
//!
//! Provides resource claiming and yielding for multi-agent coordination.
//! Uses file-based locking for simplicity.

use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zjj_core::{OutputFormat, SchemaEnvelope};

/// Options for the claim command
#[derive(Debug, Clone)]
pub struct ClaimOptions {
    /// Resource to claim (e.g., session:name, `<file:path>`, bead:id)
    pub resource: String,
    /// Timeout in seconds for the lock
    pub timeout: u64,
    /// Output format
    pub format: OutputFormat,
}

/// Options for the yield command
#[derive(Debug, Clone)]
pub struct YieldOptions {
    /// Resource to yield
    pub resource: String,
    /// Output format
    pub format: OutputFormat,
}

/// Result of claim operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimResult {
    /// Whether the claim succeeded
    pub claimed: bool,
    /// Resource that was claimed
    pub resource: String,
    /// Agent that now holds the lock
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder: Option<String>,
    /// When the lock expires (Unix timestamp)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    /// Previous holder if force-claimed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_holder: Option<String>,
    /// Error message if claim failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Whether this was a double claim (same agent re-claiming)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_double_claim: Option<bool>,
    /// Number of times this agent has claimed this resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_count: Option<usize>,
}

/// Result of yield operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldResult {
    /// Whether the yield succeeded
    pub yielded: bool,
    /// Resource that was yielded
    pub resource: String,
    /// Agent that yielded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Error message if yield failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Lock file content
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LockInfo {
    holder: String,
    resource: String,
    acquired_at: u64,
    expires_at: u64,
}

/// Audit entry for claim operations
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaimAuditEntry {
    agent_id: String,
    resource: String,
    timestamp: u64,
    action: ClaimAction,
    success: bool,
    expires_at: Option<u64>,
    previous_holder: Option<String>,
}

/// Types of claim actions for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ClaimAction {
    #[serde(rename = "initial_claim")]
    Initial,
    #[serde(rename = "double_claim")]
    Double,
    #[serde(rename = "expired_claim")]
    Expired,
    #[serde(rename = "failed_claim")]
    Failed,
}

/// Complete audit log for a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaimAudit {
    entries: Vec<ClaimAuditEntry>,
}

impl ClaimAudit {
    const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn add_entry(mut self, entry: ClaimAuditEntry) -> Self {
        self.entries.push(entry);
        self
    }
}

async fn get_locks_dir() -> Result<PathBuf> {
    let data_dir = super::zjj_data_dir().await?;
    let locks_dir = data_dir.join("locks");
    tokio::fs::create_dir_all(&locks_dir).await?;
    Ok(locks_dir)
}

async fn lock_file_path(resource: &str) -> Result<PathBuf> {
    let locks_dir = get_locks_dir().await?;
    // Sanitize resource name for filename
    let safe_name: String = resource
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    Ok(locks_dir.join(format!("{safe_name}.lock")))
}

async fn audit_file_path(resource: &str) -> Result<PathBuf> {
    let locks_dir = get_locks_dir().await?;
    let safe_name: String = resource
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    Ok(locks_dir.join(format!("{safe_name}.audit")))
}

fn current_timestamp() -> Result<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| anyhow::anyhow!("System time error: {e}"))
}

fn get_agent_id() -> String {
    std::env::var("ZJJ_AGENT_ID").unwrap_or_else(|_| format!("pid-{}", std::process::id()))
}

/// Read existing lock info from file
async fn read_lock(path: &std::path::Path) -> Option<LockInfo> {
    tokio::fs::read_to_string(path)
        .await
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
}

/// Read audit log from file
async fn read_audit(path: &std::path::Path) -> Result<ClaimAudit> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse audit log: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ClaimAudit::new()),
        Err(e) => Err(anyhow::anyhow!("Failed to read audit log: {e}")),
    }
}

/// Write audit entry to file
async fn write_audit_entry(path: &std::path::Path, entry: ClaimAuditEntry) -> Result<()> {
    let audit = read_audit(path).await?;
    let updated_audit = audit.add_entry(entry);
    let content = serde_json::to_string_pretty(&updated_audit)?;
    tokio::fs::write(path, content)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to write audit log: {e}"))
}

/// Count claims by a specific agent for a resource
fn count_claims_by_agent(audit: &ClaimAudit, agent_id: &str) -> usize {
    audit
        .entries
        .iter()
        .filter(|entry| entry.agent_id == agent_id && entry.success)
        .count()
}

/// Attempt to acquire lock using atomic file creation
async fn try_atomic_create_lock(path: &std::path::Path, lock_info: &LockInfo) -> Result<bool> {
    let content = serde_json::to_string_pretty(lock_info)?;

    // Try atomic create (will fail if file exists)
    match tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)  // O_CREAT | O_EXCL - atomic creation
        .open(path)
        .await
    {
        Ok(mut file) => {
            // Successfully created new file atomically
            use tokio::io::AsyncWriteExt;
            file.write_all(content.as_bytes()).await?;
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // File exists - need to check if we can take it
            Ok(false)
        }
        Err(e) => Err(anyhow::anyhow!("Failed to create lock file: {e}")),
    }
}

/// Overwrite lock file (used for extensions and takeovers)
async fn write_lock(path: &std::path::Path, lock_info: &LockInfo) -> Result<()> {
    let content = serde_json::to_string_pretty(lock_info)?;
    tokio::fs::write(path, content).await?;
    Ok(())
}

/// Parameters for creating a claim audit entry
struct AuditEntryParams<'a> {
    agent_id: &'a str,
    resource: &'a str,
    now: u64,
    action: ClaimAction,
    expires_at: Option<u64>,
    previous_holder: Option<String>,
}

/// Create audit entry for claim operation
fn create_audit_entry(params: AuditEntryParams) -> ClaimAuditEntry {
    let success = matches!(
        params.action,
        ClaimAction::Initial | ClaimAction::Double | ClaimAction::Expired
    );
    ClaimAuditEntry {
        agent_id: params.agent_id.to_string(),
        resource: params.resource.to_string(),
        timestamp: params.now,
        action: params.action,
        success,
        expires_at: params.expires_at,
        previous_holder: params.previous_holder,
    }
}

/// Create successful claim result
fn create_success_result(
    resource: &str,
    agent_id: &str,
    expires_at: u64,
    claim_count: usize,
    is_double_claim: bool,
) -> ClaimResult {
    ClaimResult {
        claimed: true,
        resource: resource.to_string(),
        holder: Some(agent_id.to_string()),
        expires_at: Some(expires_at),
        previous_holder: None,
        error: None,
        is_double_claim: Some(is_double_claim),
        claim_count: Some(claim_count),
    }
}

/// Create failed claim result with error message
fn create_error_result(
    resource: &str,
    agent_id: &str,
    error: String,
    claim_count: usize,
) -> ClaimResult {
    ClaimResult {
        claimed: false,
        resource: resource.to_string(),
        holder: Some(agent_id.to_string()),
        expires_at: None,
        previous_holder: None,
        error: Some(error),
        is_double_claim: None,
        claim_count: Some(claim_count),
    }
}

/// Handle expired lock takeover
#[allow(clippy::too_many_arguments)]
// Many parameters needed because: each represents distinct claim operation context
// (paths, lock state, resource metadata, timing, audit data). Grouping into
// struct would reduce clarity and make it harder to understand what data flows through.
async fn handle_expired_lock(
    lock_path: &std::path::Path,
    audit_path: &std::path::Path,
    new_lock: &LockInfo,
    existing: &LockInfo,
    resource: &str,
    agent_id: &str,
    now: u64,
    claim_count: usize,
) -> ClaimResult {
    let write_result = write_lock(lock_path, new_lock).await;
    let audit_entry = create_audit_entry(AuditEntryParams {
        agent_id,
        resource,
        now,
        action: ClaimAction::Expired,
        expires_at: Some(new_lock.expires_at),
        previous_holder: Some(existing.holder.clone()),
    });
    let _ = write_audit_entry(audit_path, audit_entry).await;

    match write_result {
        Ok(()) => ClaimResult {
            claimed: true,
            resource: resource.to_string(),
            holder: Some(agent_id.to_string()),
            expires_at: Some(new_lock.expires_at),
            previous_holder: Some(existing.holder.clone()),
            error: None,
            is_double_claim: Some(false),
            claim_count: Some(claim_count + 1),
        },
        Err(e) => create_error_result(
            resource,
            agent_id,
            format!("Failed to write lock: {e}"),
            claim_count,
        ),
    }
}

/// Handle double claim (agent re-claims their own lock)
#[allow(clippy::too_many_arguments)]
// Many parameters needed because: each represents distinct claim operation context
// (paths, lock state, resource metadata, timing, audit data). Grouping into
// struct would reduce clarity and make it harder to understand what data flows through.
async fn handle_double_claim(
    lock_path: &std::path::Path,
    audit_path: &std::path::Path,
    new_lock: &LockInfo,
    resource: &str,
    agent_id: &str,
    now: u64,
    claim_count: usize,
) -> ClaimResult {
    let audit_entry = create_audit_entry(AuditEntryParams {
        agent_id,
        resource,
        now,
        action: ClaimAction::Double,
        expires_at: Some(new_lock.expires_at),
        previous_holder: None,
    });
    let _ = write_audit_entry(audit_path, audit_entry).await;

    match write_lock(lock_path, new_lock).await {
        Ok(()) => ClaimResult {
            claimed: true,
            resource: resource.to_string(),
            holder: Some(agent_id.to_string()),
            expires_at: Some(new_lock.expires_at),
            previous_holder: None,
            error: None,
            is_double_claim: Some(true),
            claim_count: Some(claim_count + 1),
        },
        Err(e) => ClaimResult {
            claimed: false,
            resource: resource.to_string(),
            holder: Some(agent_id.to_string()),
            expires_at: None,
            previous_holder: None,
            error: Some(format!("Failed to extend lock: {e}")),
            is_double_claim: Some(true),
            claim_count: Some(claim_count),
        },
    }
}

/// Handle failed claim (locked by another agent)
#[allow(clippy::too_many_arguments)]
// Many parameters needed because: each represents distinct claim operation context
// (paths, lock state, resource metadata, timing, audit data). Grouping into
// struct would reduce clarity and make it harder to understand what data flows through.
async fn handle_failed_claim(
    audit_path: &std::path::Path,
    existing: &LockInfo,
    resource: &str,
    agent_id: &str,
    now: u64,
    claim_count: usize,
) -> ClaimResult {
    let audit_entry = create_audit_entry(AuditEntryParams {
        agent_id,
        resource,
        now,
        action: ClaimAction::Failed,
        expires_at: None,
        previous_holder: Some(existing.holder.clone()),
    });
    let _ = write_audit_entry(audit_path, audit_entry).await;

    ClaimResult {
        claimed: false,
        resource: resource.to_string(),
        holder: Some(existing.holder.clone()),
        expires_at: Some(existing.expires_at),
        previous_holder: None,
        error: Some(format!(
            "Resource locked by {holder}",
            holder = existing.holder
        )),
        is_double_claim: Some(false),
        claim_count: Some(claim_count),
    }
}

/// Handle corrupt/missing lock file
#[allow(clippy::too_many_arguments)]
// Many parameters needed because: each represents distinct claim operation context
// (paths, lock state, resource metadata, timing, audit data). Grouping into
// struct would reduce clarity and make it harder to understand what data flows through.
async fn handle_corrupt_lock(
    lock_path: &std::path::Path,
    audit_path: &std::path::Path,
    new_lock: &LockInfo,
    resource: &str,
    agent_id: &str,
    now: u64,
    claim_count: usize,
) -> ClaimResult {
    let write_result = write_lock(lock_path, new_lock).await;
    let audit_entry = create_audit_entry(AuditEntryParams {
        agent_id,
        resource,
        now,
        action: ClaimAction::Initial,
        expires_at: Some(new_lock.expires_at),
        previous_holder: None,
    });
    let _ = write_audit_entry(audit_path, audit_entry).await;

    match write_result {
        Ok(()) => ClaimResult {
            claimed: true,
            resource: resource.to_string(),
            holder: Some(agent_id.to_string()),
            expires_at: Some(new_lock.expires_at),
            previous_holder: None,
            error: None,
            is_double_claim: Some(false),
            claim_count: Some(claim_count + 1),
        },
        Err(e) => create_error_result(
            resource,
            agent_id,
            format!("Failed to write lock: {e}"),
            claim_count,
        ),
    }
}

/// Try to claim a resource, returning the result
#[allow(clippy::too_many_arguments)]
// Many parameters needed because: each represents distinct claim operation context
// (paths, lock state, resource metadata, timing, audit data). Grouping into
// struct would reduce clarity and make it harder to understand what data flows through.
async fn attempt_claim(
    lock_path: &std::path::Path,
    audit_path: &std::path::Path,
    resource: &str,
    agent_id: &str,
    now: u64,
    expires_at: u64,
) -> Result<ClaimResult> {
    let new_lock = LockInfo {
        holder: agent_id.to_string(),
        resource: resource.to_string(),
        acquired_at: now,
        expires_at,
    };

    // Try atomic create first (prevents TOCTOU)
    if try_atomic_create_lock(lock_path, &new_lock).await? {
        let audit_entry = create_audit_entry(AuditEntryParams {
            agent_id,
            resource,
            now,
            action: ClaimAction::Initial,
            expires_at: Some(expires_at),
            previous_holder: None,
        });
        let _ = write_audit_entry(audit_path, audit_entry).await;

        return Ok(create_success_result(
            resource, agent_id, expires_at, 1, false,
        ));
    }

    // Lock file exists - check if we can take it
    let audit = read_audit(audit_path).await?;
    let claim_count = count_claims_by_agent(&audit, agent_id);

    let result = if let Some(existing) = read_lock(lock_path).await {
        if existing.expires_at < now {
            // Lock expired - take it
            handle_expired_lock(
                lock_path,
                audit_path,
                &new_lock,
                &existing,
                resource,
                agent_id,
                now,
                claim_count,
            )
            .await
        } else if existing.holder == agent_id {
            // DOUBLE CLAIM DETECTED - we already hold it
            handle_double_claim(
                lock_path,
                audit_path,
                &new_lock,
                resource,
                agent_id,
                now,
                claim_count,
            )
            .await
        } else {
            // Someone else holds valid lock
            handle_failed_claim(audit_path, &existing, resource, agent_id, now, claim_count).await
        }
    } else {
        // Lock file unreadable/corrupt - try to take it
        handle_corrupt_lock(
            lock_path,
            audit_path,
            &new_lock,
            resource,
            agent_id,
            now,
            claim_count,
        )
        .await
    };

    Ok(result)
}

/// Run the claim command
pub async fn run_claim(options: &ClaimOptions) -> Result<()> {
    let agent_id = get_agent_id();
    let lock_path = lock_file_path(&options.resource).await?;
    let audit_path = audit_file_path(&options.resource).await?;
    let now = current_timestamp()?;
    let expires_at = now + options.timeout;

    let result = attempt_claim(
        &lock_path,
        &audit_path,
        &options.resource,
        &agent_id,
        now,
        expires_at,
    )
    .await?;

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("claim-response", "single", &result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else if result.claimed {
        if result.is_double_claim.unwrap_or(false) {
            println!("⚠ Double claim detected for resource '{}'", result.resource);
            if let Some(count) = result.claim_count {
                println!("  Total claims by you: {count}");
            }
        } else {
            println!("✓ Claimed resource '{}'", result.resource);
        }
        if let Some(h) = &result.holder {
            println!("  Lock holder: {h}");
        }
        if let Some(p) = &result.previous_holder {
            println!("  Previous holder: {p} (expired/force-claimed)");
        }
    } else {
        eprintln!("✗ Failed to claim resource '{}'", result.resource);
        if let Some(h) = &result.holder {
            eprintln!("  Currently held by: {h}");
        }
        if let Some(e) = &result.error {
            eprintln!("  Error: {e}");
        }
        anyhow::bail!("Failed to claim resource");
    }

    Ok(())
}

/// Attempt to yield a resource
async fn attempt_yield(lock_path: &std::path::Path, resource: &str, agent_id: &str) -> YieldResult {
    // No lock exists - consider it successfully yielded (idempotent)
    match tokio::fs::try_exists(lock_path).await {
        Ok(false) | Err(_) => {
            return YieldResult {
                yielded: true,
                resource: resource.to_string(),
                agent_id: Some(agent_id.to_string()),
                error: None,
            };
        }
        Ok(true) => {}
    }

    // Try to read and verify ownership
    if let Some(lock) = read_lock(lock_path).await {
        if lock.holder == agent_id {
            // We hold it - release
            tokio::fs::remove_file(lock_path)
                .await
                .map(|()| YieldResult {
                    yielded: true,
                    resource: resource.to_string(),
                    agent_id: Some(agent_id.to_string()),
                    error: None,
                })
                .unwrap_or_else(|e| YieldResult {
                    yielded: false,
                    resource: resource.to_string(),
                    agent_id: Some(agent_id.to_string()),
                    error: Some(format!("Failed to remove lock: {e}")),
                })
        } else {
            // Someone else holds it
            YieldResult {
                yielded: false,
                resource: resource.to_string(),
                agent_id: Some(agent_id.to_string()),
                error: Some(format!("Resource held by {}, not us", lock.holder)),
            }
        }
    } else {
        // Lock file corrupt - just remove it
        let _ = tokio::fs::remove_file(lock_path).await;
        YieldResult {
            yielded: true,
            resource: resource.to_string(),
            agent_id: Some(agent_id.to_string()),
            error: None,
        }
    }
}

/// Run the yield command
pub async fn run_yield(options: &YieldOptions) -> Result<()> {
    let agent_id = get_agent_id();
    let lock_path = lock_file_path(&options.resource).await?;

    let result = attempt_yield(&lock_path, &options.resource, &agent_id).await;

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("yield-response", "single", &result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else if result.yielded {
        println!("✓ Yielded resource '{}'", result.resource);
    } else {
        eprintln!("✗ Failed to yield resource '{}'", result.resource);
        if let Some(e) = &result.error {
            eprintln!("  Error: {e}");
        }
        anyhow::bail!("Failed to yield resource");
    }

    Ok(())
}

/// Query claim history for a resource
pub async fn query_claim_history(resource: &str) -> Result<Vec<ClaimAuditEntry>> {
    let audit_path = audit_file_path(resource).await?;
    let audit = read_audit(&audit_path).await?;
    Ok(audit.entries)
}

/// Show current holders of all locked resources
pub async fn show_current_holders() -> Result<Vec<LockInfo>> {
    let locks_dir = get_locks_dir().await?;
    let mut entries = tokio::fs::read_dir(&locks_dir)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read locks directory: {e}"))?;

    let mut locks = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "lock") {
            if let Some(lock_info) = read_lock(&path).await {
                // Only include non-expired locks
                let now = current_timestamp()?;
                if lock_info.expires_at > now {
                    locks.push(lock_info);
                }
            }
        }
    }

    Ok(locks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claim_result_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let result = ClaimResult {
            claimed: true,
            resource: "session:test".to_string(),
            holder: Some("agent-1".to_string()),
            expires_at: Some(1_234_567_890),
            previous_holder: None,
            error: None,
            is_double_claim: Some(false),
            claim_count: Some(1),
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"claimed\":true"));
        assert!(json.contains("\"resource\":\"session:test\""));
        Ok(())
    }

    #[test]
    fn test_yield_result_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let result = YieldResult {
            yielded: true,
            resource: "session:test".to_string(),
            agent_id: Some("agent-1".to_string()),
            error: None,
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"yielded\":true"));
        Ok(())
    }

    #[test]
    fn test_claim_result_with_error() -> Result<(), Box<dyn std::error::Error>> {
        let result = ClaimResult {
            claimed: false,
            resource: "session:test".to_string(),
            holder: Some("other-agent".to_string()),
            expires_at: None,
            previous_holder: None,
            error: Some("Resource is locked".to_string()),
            is_double_claim: Some(false),
            claim_count: Some(0),
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"claimed\":false"));
        assert!(json.contains("\"error\""));
        Ok(())
    }

    #[test]
    fn test_claim_result_double_claim() -> Result<(), Box<dyn std::error::Error>> {
        let result = ClaimResult {
            claimed: true,
            resource: "session:test".to_string(),
            holder: Some("agent-1".to_string()),
            expires_at: Some(1_234_567_890),
            previous_holder: None,
            error: None,
            is_double_claim: Some(true),
            claim_count: Some(3),
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"is_double_claim\":true"));
        assert!(json.contains("\"claim_count\":3"));
        Ok(())
    }

    #[test]
    fn test_lock_info_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let lock = LockInfo {
            holder: "agent-1".to_string(),
            resource: "session:test".to_string(),
            acquired_at: 1000,
            expires_at: 2000,
        };

        let json = serde_json::to_string(&lock)?;
        let parsed: LockInfo = serde_json::from_str(&json)?;

        assert_eq!(parsed.holder, "agent-1");
        assert_eq!(parsed.expires_at, 2000);
        Ok(())
    }

    // ============================================================================
    // Martin Fowler Style Behavior Tests
    // These tests describe the BEHAVIOR of the claim/yield commands
    // ============================================================================

    mod claim_behavior {
        use super::*;

        /// GIVEN: Resource is not locked
        /// WHEN: Agent claims it
        /// THEN: claimed=true, holder=requesting agent
        #[test]
        fn successful_claim_shows_ownership() {
            let result = ClaimResult {
                claimed: true,
                resource: "session:my-task".to_string(),
                holder: Some("agent-abc".to_string()),
                expires_at: Some(1_234_567_890),
                previous_holder: None,
                error: None,
                is_double_claim: Some(false),
                claim_count: Some(1),
            };

            assert!(result.claimed, "Should be claimed");
            assert_eq!(
                result.holder,
                Some("agent-abc".to_string()),
                "Holder should be requester"
            );
            assert!(result.error.is_none(), "No error on success");
            assert!(
                result.previous_holder.is_none(),
                "No previous holder for fresh claim"
            );
            assert_eq!(
                result.is_double_claim,
                Some(false),
                "Should not be double claim"
            );
            assert_eq!(result.claim_count, Some(1), "First claim");
        }

        /// GIVEN: Resource is locked by another agent
        /// WHEN: This agent tries to claim it
        /// THEN: claimed=false, holder=other agent, error explains
        #[test]
        fn blocked_claim_shows_current_holder() {
            let result = ClaimResult {
                claimed: false,
                resource: "session:contested".to_string(),
                holder: Some("agent-xyz".to_string()),
                expires_at: Some(1_234_567_890),
                previous_holder: None,
                error: Some("Resource is locked by agent-xyz".to_string()),
                is_double_claim: Some(false),
                claim_count: Some(0),
            };

            assert!(!result.claimed, "Should not be claimed");
            assert_eq!(
                result.holder,
                Some("agent-xyz".to_string()),
                "Shows who holds it"
            );
            assert!(result.error.is_some(), "Should explain why");
        }

        /// GIVEN: Lock expired
        /// WHEN: New agent claims resource
        /// THEN: claimed=true, `previous_holder=old` agent
        #[test]
        fn expired_lock_shows_previous_holder() {
            let result = ClaimResult {
                claimed: true,
                resource: "session:expired-lock".to_string(),
                holder: Some("agent-new".to_string()),
                expires_at: Some(9_999_999_999),
                previous_holder: Some("agent-old".to_string()),
                error: None,
                is_double_claim: Some(false),
                claim_count: Some(1),
            };

            assert!(result.claimed, "Should claim expired lock");
            assert_eq!(
                result.previous_holder,
                Some("agent-old".to_string()),
                "Shows who had it"
            );
        }

        /// GIVEN: Agent re-claims their own resource
        /// WHEN: Claim is made
        /// THEN: Should extend expiration AND flag as double claim
        #[test]
        fn reclaim_detects_double_claim() {
            let result = ClaimResult {
                claimed: true,
                resource: "session:my-lock".to_string(),
                holder: Some("agent-me".to_string()),
                expires_at: Some(2_000_000_000), // Extended
                previous_holder: None,           // Still us, no "previous"
                error: None,
                is_double_claim: Some(true), // NEW: Detects double claim
                claim_count: Some(3),        // NEW: Shows claim count
            };

            assert!(result.claimed);
            assert!(result.expires_at.is_some(), "Should have new expiration");
            assert_eq!(
                result.is_double_claim,
                Some(true),
                "Should detect double claim"
            );
            assert_eq!(result.claim_count, Some(3), "Should count total claims");
        }
    }

    mod yield_behavior {
        use super::*;

        /// GIVEN: Agent holds a resource
        /// WHEN: Agent yields it
        /// THEN: yielded=true, resource is released
        #[test]
        fn successful_yield_releases_resource() {
            let result = YieldResult {
                yielded: true,
                resource: "session:my-task".to_string(),
                agent_id: Some("agent-abc".to_string()),
                error: None,
            };

            assert!(result.yielded, "Should be yielded");
            assert!(result.error.is_none(), "No error on success");
        }

        /// GIVEN: Agent does not hold the resource
        /// WHEN: Agent tries to yield it
        /// THEN: yielded=false, error explains
        #[test]
        fn cannot_yield_others_lock() {
            let result = YieldResult {
                yielded: false,
                resource: "session:not-mine".to_string(),
                agent_id: Some("agent-me".to_string()),
                error: Some("Resource held by agent-other, not us".to_string()),
            };

            assert!(!result.yielded, "Cannot yield others' lock");
            assert!(result.error.is_some(), "Should explain why");
            assert!(
                result.error.as_ref().is_some_and(|e| e.contains("not us")),
                "Error should mention ownership issue"
            );
        }

        /// GIVEN: Resource is not locked
        /// WHEN: Agent tries to yield it
        /// THEN: yielded=true (idempotent) or false depending on semantics
        #[test]
        fn yield_unlocked_is_idempotent() {
            // Yielding an unlocked resource should be a no-op
            let result = YieldResult {
                yielded: true, // Already unlocked is "successfully yielded"
                resource: "session:unlocked".to_string(),
                agent_id: Some("agent-abc".to_string()),
                error: None,
            };

            assert!(
                result.yielded || result.error.is_none(),
                "Should handle gracefully"
            );
        }
    }

    mod lock_info_behavior {
        use super::*;

        /// GIVEN: Lock info
        /// WHEN: Created
        /// THEN: Should have holder, resource, `acquired_at`, `expires_at`
        #[test]
        fn lock_has_all_required_fields() {
            let lock = LockInfo {
                holder: "agent-123".to_string(),
                resource: "session:feature-x".to_string(),
                acquired_at: 1_609_459_200, // 2021-01-01 00:00:00 UTC
                expires_at: 1_609_462_800,  // 2021-01-01 01:00:00 UTC
            };

            assert!(!lock.holder.is_empty(), "Must have holder");
            assert!(!lock.resource.is_empty(), "Must have resource");
            assert!(lock.acquired_at > 0, "Must have acquired time");
            assert!(lock.expires_at > lock.acquired_at, "Expires after acquired");
        }

        /// GIVEN: Lock with timestamps
        /// WHEN: Checked for expiration
        /// THEN: `expires_at` determines if expired
        #[test]
        fn expiration_is_based_on_expires_at() {
            let now = 1_609_461_000; // Some time

            // Not expired
            let active_lock = LockInfo {
                holder: "agent".to_string(),
                resource: "res".to_string(),
                acquired_at: 1_609_459_200,
                expires_at: 1_609_462_800,
            };
            assert!(active_lock.expires_at > now, "Should not be expired");

            // Expired
            let expired_lock = LockInfo {
                holder: "agent".to_string(),
                resource: "res".to_string(),
                acquired_at: 1_609_459_200,
                expires_at: 1_609_459_300,
            };
            assert!(expired_lock.expires_at < now, "Should be expired");
        }

        /// GIVEN: Lock info is serialized
        /// WHEN: Read back
        /// THEN: All fields should match
        #[test]
        fn lock_roundtrips_through_json() -> Result<(), Box<dyn std::error::Error>> {
            let original = LockInfo {
                holder: "agent-roundtrip".to_string(),
                resource: "session:test".to_string(),
                acquired_at: 1_234_567_890,
                expires_at: 1_234_567_900,
            };

            let json = serde_json::to_string(&original)?;
            let parsed: LockInfo = serde_json::from_str(&json)?;

            assert_eq!(parsed.holder, original.holder);
            assert_eq!(parsed.resource, original.resource);
            assert_eq!(parsed.acquired_at, original.acquired_at);
            assert_eq!(parsed.expires_at, original.expires_at);
            Ok(())
        }
    }

    mod resource_naming_behavior {
        /// GIVEN: Resource names
        /// WHEN: Used in claims
        /// THEN: Should follow consistent naming convention
        #[test]
        fn resource_names_are_descriptive() {
            let resources = [
                "session:feature-auth",
                "session:bugfix-123",
                "workspace:main",
            ];

            for resource in resources {
                assert!(
                    resource.contains(':'),
                    "Resources should use prefix:name format"
                );
                let parts: Vec<&str> = resource.split(':').collect();
                assert_eq!(parts.len(), 2, "Should have exactly one colon");
                assert!(!parts[0].is_empty(), "Prefix should not be empty");
                assert!(!parts[1].is_empty(), "Name should not be empty");
            }
        }
    }

    mod json_output_behavior {
        use super::*;

        /// GIVEN: `ClaimResult` is serialized
        /// WHEN: AI parses it
        /// THEN: Should have enough info for decision making
        #[test]
        fn claim_result_json_is_decision_ready() -> Result<(), Box<dyn std::error::Error>> {
            let result = ClaimResult {
                claimed: true,
                resource: "session:task".to_string(),
                holder: Some("agent-1".to_string()),
                expires_at: Some(9_999_999_999),
                previous_holder: None,
                error: None,
                is_double_claim: Some(false),
                claim_count: Some(1),
            };

            let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&result)?)?;

            // Decision field
            assert!(json.get("claimed").is_some(), "Need claimed for decision");
            assert!(json["claimed"].is_boolean(), "claimed must be boolean");

            // Context fields
            assert!(json.get("resource").is_some(), "Need resource for context");
            assert!(json.get("holder").is_some(), "Need holder for context");
            Ok(())
        }

        /// GIVEN: Failed claim
        /// WHEN: Serialized
        /// THEN: Error should explain why
        #[test]
        fn failed_claim_json_is_debuggable() -> Result<(), Box<dyn std::error::Error>> {
            let result = ClaimResult {
                claimed: false,
                resource: "session:contested".to_string(),
                holder: Some("other-agent".to_string()),
                expires_at: Some(1_234_567_890),
                previous_holder: None,
                error: Some("Resource locked by other-agent".to_string()),
                is_double_claim: Some(false),
                claim_count: Some(0),
            };

            let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&result)?)?;

            assert_eq!(json["claimed"].as_bool(), Some(false));
            assert!(json.get("error").is_some(), "Failed claims need error");
            assert!(json["error"]
                .as_str()
                .ok_or("error not string")?
                .contains("other-agent"));
            Ok(())
        }

        /// GIVEN: Double claim
        /// WHEN: Serialized
        /// THEN: Should include double claim flag and count
        #[test]
        fn double_claim_json_includes_detection() -> Result<(), Box<dyn std::error::Error>> {
            let result = ClaimResult {
                claimed: true,
                resource: "session:double".to_string(),
                holder: Some("agent-1".to_string()),
                expires_at: Some(9_999_999_999),
                previous_holder: None,
                error: None,
                is_double_claim: Some(true),
                claim_count: Some(5),
            };

            let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&result)?)?;

            assert_eq!(json["claimed"].as_bool(), Some(true));
            assert_eq!(json["is_double_claim"].as_bool(), Some(true));
            assert_eq!(json["claim_count"], 5);
            Ok(())
        }

        /// GIVEN: `YieldResult` is serialized
        /// WHEN: AI parses it
        /// THEN: Should have clear success indicator
        #[test]
        fn yield_result_json_has_success_indicator() -> Result<(), Box<dyn std::error::Error>> {
            let result = YieldResult {
                yielded: true,
                resource: "session:task".to_string(),
                agent_id: Some("agent-1".to_string()),
                error: None,
            };

            let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&result)?)?;

            assert!(json.get("yielded").is_some());
            assert!(json["yielded"].is_boolean());
            Ok(())
        }
    }

    // ============================================================================
    // Audit Trail Tests
    // Tests for claim history tracking and double claim detection
    // ============================================================================

    mod audit_trail_behavior {
        use super::*;

        /// GIVEN: Audit log entry
        /// WHEN: Created
        /// THEN: Should have all required fields
        #[test]
        fn audit_entry_has_required_fields() {
            let entry = ClaimAuditEntry {
                agent_id: "agent-123".to_string(),
                resource: "session:feature-x".to_string(),
                timestamp: 1_609_459_200,
                action: ClaimAction::Initial,
                success: true,
                expires_at: Some(1_609_462_800),
                previous_holder: None,
            };

            assert!(!entry.agent_id.is_empty());
            assert!(!entry.resource.is_empty());
            assert!(entry.timestamp > 0);
            assert!(entry.success);
            assert!(entry.expires_at.is_some());
        }

        /// GIVEN: Empty audit log
        /// WHEN: Entry added
        /// THEN: Should contain the entry
        #[test]
        fn audit_log_tracks_entries() {
            let audit = ClaimAudit::new();
            let entry = ClaimAuditEntry {
                agent_id: "agent-1".to_string(),
                resource: "session:test".to_string(),
                timestamp: 1000,
                action: ClaimAction::Initial,
                success: true,
                expires_at: Some(2000),
                previous_holder: None,
            };

            let updated = audit.add_entry(entry);
            assert_eq!(updated.entries.len(), 1);
            assert_eq!(updated.entries[0].agent_id, "agent-1");
        }

        /// GIVEN: Multiple claims
        /// WHEN: Counted by agent
        /// THEN: Should return correct count
        #[tokio::test]
        async fn count_claims_by_agent_works() {
            let mut audit = ClaimAudit::new();

            // Add 3 successful claims by agent-1
            for i in 0..3 {
                audit = audit.add_entry(ClaimAuditEntry {
                    agent_id: "agent-1".to_string(),
                    resource: "session:test".to_string(),
                    timestamp: 1000 + i,
                    action: ClaimAction::Initial,
                    success: true,
                    expires_at: Some(2000 + i),
                    previous_holder: None,
                });
            }

            // Add 1 failed claim
            audit = audit.add_entry(ClaimAuditEntry {
                agent_id: "agent-1".to_string(),
                resource: "session:test".to_string(),
                timestamp: 1003,
                action: ClaimAction::Failed,
                success: false,
                expires_at: None,
                previous_holder: Some("agent-2".to_string()),
            });

            let count = count_claims_by_agent(&audit, "agent-1");
            assert_eq!(count, 3, "Should count only successful claims");
        }

        /// GIVEN: Claim actions
        /// WHEN: Serialized
        /// THEN: Should have correct JSON representation
        #[test]
        fn claim_action_serializes_correctly() -> Result<(), Box<dyn std::error::Error>> {
            let actions = [
                (ClaimAction::Initial, "initial_claim"),
                (ClaimAction::Double, "double_claim"),
                (ClaimAction::Expired, "expired_claim"),
                (ClaimAction::Failed, "failed_claim"),
            ];

            for (action, expected) in actions {
                let json = serde_json::to_string(&action)?;
                assert!(
                    json.contains(expected),
                    "Action should serialize to {expected}"
                );
            }

            Ok(())
        }

        /// GIVEN: Audit entry with double claim
        /// WHEN: Created
        /// THEN: Should indicate double claim action
        #[test]
        fn double_claim_audit_entry() {
            let entry = ClaimAuditEntry {
                agent_id: "agent-1".to_string(),
                resource: "session:test".to_string(),
                timestamp: 2000,
                action: ClaimAction::Double,
                success: true,
                expires_at: Some(3000),
                previous_holder: None,
            };

            assert!(matches!(entry.action, ClaimAction::Double));
            assert!(entry.success);
        }
    }
}
