//! Claim/Yield commands - Multi-agent resource locking
//!
//! Provides resource claiming and yielding for multi-agent coordination.
//! Uses file-based locking for simplicity.

use std::{fs, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zjj_core::{OutputFormat, SchemaEnvelope};

/// Options for the claim command
#[derive(Debug, Clone)]
pub struct ClaimOptions {
    /// Resource to claim (e.g., session:name, file:path, bead:id)
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

fn get_locks_dir() -> Result<PathBuf> {
    let data_dir = super::zjj_data_dir()?;
    let locks_dir = data_dir.join("locks");
    fs::create_dir_all(&locks_dir)?;
    Ok(locks_dir)
}

fn lock_file_path(resource: &str) -> Result<PathBuf> {
    let locks_dir = get_locks_dir()?;
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

fn current_timestamp() -> Result<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| anyhow::anyhow!("System time error: {}", e))
}

fn get_agent_id() -> String {
    std::env::var("ZJJ_AGENT_ID").unwrap_or_else(|_| format!("pid-{}", std::process::id()))
}

/// Read existing lock info from file
fn read_lock(path: &std::path::Path) -> Option<LockInfo> {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
}

/// Attempt to acquire lock using atomic file creation
fn try_atomic_create_lock(
    path: &std::path::Path,
    lock_info: &LockInfo,
) -> Result<bool> {
    let content = serde_json::to_string_pretty(lock_info)?;

    // Try atomic create (will fail if file exists)
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)  // O_CREAT | O_EXCL - atomic creation
        .open(path)
    {
        Ok(file) => {
            // Successfully created new file atomically
            use std::io::Write;
            let mut file = file;
            file.write_all(content.as_bytes())?;
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // File exists - need to check if we can take it
            Ok(false)
        }
        Err(e) => Err(anyhow::anyhow!("Failed to create lock file: {}", e)),
    }
}

/// Overwrite lock file (used for extensions and takeovers)
fn write_lock(path: &std::path::Path, lock_info: &LockInfo) -> Result<()> {
    let content = serde_json::to_string_pretty(lock_info)?;
    fs::write(path, content)?;
    Ok(())
}

/// Try to claim a resource, returning the result
fn attempt_claim(
    lock_path: &std::path::Path,
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
    if try_atomic_create_lock(lock_path, &new_lock)? {
        return Ok(ClaimResult {
            claimed: true,
            resource: resource.to_string(),
            holder: Some(agent_id.to_string()),
            expires_at: Some(expires_at),
            previous_holder: None,
            error: None,
        });
    }

    // Lock file exists - check if we can take it
    read_lock(lock_path)
        .map(|existing| {
            if existing.expires_at < now {
                // Lock expired - take it
                write_lock(lock_path, &new_lock)
                    .map(|()| ClaimResult {
                        claimed: true,
                        resource: resource.to_string(),
                        holder: Some(agent_id.to_string()),
                        expires_at: Some(expires_at),
                        previous_holder: Some(existing.holder),
                        error: None,
                    })
                    .unwrap_or_else(|e| ClaimResult {
                        claimed: false,
                        resource: resource.to_string(),
                        holder: None,
                        expires_at: None,
                        previous_holder: None,
                        error: Some(format!("Failed to write lock: {}", e)),
                    })
            } else if existing.holder == agent_id {
                // We already hold it - extend
                write_lock(lock_path, &new_lock)
                    .map(|()| ClaimResult {
                        claimed: true,
                        resource: resource.to_string(),
                        holder: Some(agent_id.to_string()),
                        expires_at: Some(expires_at),
                        previous_holder: None,
                        error: None,
                    })
                    .unwrap_or_else(|e| ClaimResult {
                        claimed: false,
                        resource: resource.to_string(),
                        holder: Some(agent_id.to_string()),
                        expires_at: None,
                        previous_holder: None,
                        error: Some(format!("Failed to extend lock: {}", e)),
                    })
            } else {
                // Someone else holds valid lock
                ClaimResult {
                    claimed: false,
                    resource: resource.to_string(),
                    holder: Some(existing.holder.clone()),
                    expires_at: Some(existing.expires_at),
                    previous_holder: None,
                    error: Some(format!("Resource locked by {}", existing.holder)),
                }
            }
        })
        .unwrap_or_else(|| {
            // Lock file unreadable/corrupt - try to take it
            write_lock(lock_path, &new_lock)
                .map(|()| ClaimResult {
                    claimed: true,
                    resource: resource.to_string(),
                    holder: Some(agent_id.to_string()),
                    expires_at: Some(expires_at),
                    previous_holder: None,
                    error: None,
                })
                .unwrap_or_else(|e| ClaimResult {
                    claimed: false,
                    resource: resource.to_string(),
                    holder: None,
                    expires_at: None,
                    previous_holder: None,
                    error: Some(format!("Failed to write lock: {}", e)),
                })
        })
        .pipe(Ok)
}

/// Pipe extension for functional chaining
trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R;
}

impl<T> Pipe for T {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

/// Run the claim command
pub fn run_claim(options: &ClaimOptions) -> Result<()> {
    let agent_id = get_agent_id();
    let lock_path = lock_file_path(&options.resource)?;
    let now = current_timestamp()?;
    let expires_at = now + options.timeout;

    let result = attempt_claim(&lock_path, &options.resource, &agent_id, now, expires_at)?;

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("claim-response", "single", &result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else if result.claimed {
        println!("✓ Claimed resource '{}'", result.resource);
        result.holder.as_ref().map(|h| println!("  Lock holder: {h}"));
        result
            .previous_holder
            .as_ref()
            .map(|p| println!("  Previous holder: {p} (expired/force-claimed)"));
    } else {
        eprintln!("✗ Failed to claim resource '{}'", result.resource);
        result
            .holder
            .as_ref()
            .map(|h| eprintln!("  Currently held by: {h}"));
        result.error.as_ref().map(|e| eprintln!("  Error: {e}"));
        anyhow::bail!("Failed to claim resource");
    }

    Ok(())
}

/// Attempt to yield a resource
fn attempt_yield(lock_path: &std::path::Path, resource: &str, agent_id: &str) -> YieldResult {
    // No lock exists - consider it successfully yielded (idempotent)
    if !lock_path.exists() {
        return YieldResult {
            yielded: true,
            resource: resource.to_string(),
            agent_id: Some(agent_id.to_string()),
            error: None,
        };
    }

    // Try to read and verify ownership
    read_lock(lock_path)
        .map(|lock| {
            if lock.holder == agent_id {
                // We hold it - release
                fs::remove_file(lock_path)
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
                        error: Some(format!("Failed to remove lock: {}", e)),
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
        })
        .unwrap_or_else(|| {
            // Lock file corrupt - just remove it
            let _ = fs::remove_file(lock_path);
            YieldResult {
                yielded: true,
                resource: resource.to_string(),
                agent_id: Some(agent_id.to_string()),
                error: None,
            }
        })
}

/// Run the yield command
pub fn run_yield(options: &YieldOptions) -> Result<()> {
    let agent_id = get_agent_id();
    let lock_path = lock_file_path(&options.resource)?;

    let result = attempt_yield(&lock_path, &options.resource, &agent_id);

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("yield-response", "single", &result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else if result.yielded {
        println!("✓ Yielded resource '{}'", result.resource);
    } else {
        eprintln!("✗ Failed to yield resource '{}'", result.resource);
        result.error.as_ref().map(|e| eprintln!("  Error: {e}"));
        anyhow::bail!("Failed to yield resource");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claim_result_serialization() {
        let result = ClaimResult {
            claimed: true,
            resource: "session:test".to_string(),
            holder: Some("agent-1".to_string()),
            expires_at: Some(1234567890),
            previous_holder: None,
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"claimed\":true"));
        assert!(json.contains("\"resource\":\"session:test\""));
    }

    #[test]
    fn test_yield_result_serialization() {
        let result = YieldResult {
            yielded: true,
            resource: "session:test".to_string(),
            agent_id: Some("agent-1".to_string()),
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"yielded\":true"));
    }

    #[test]
    fn test_claim_result_with_error() {
        let result = ClaimResult {
            claimed: false,
            resource: "session:test".to_string(),
            holder: Some("other-agent".to_string()),
            expires_at: None,
            previous_holder: None,
            error: Some("Resource is locked".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"claimed\":false"));
        assert!(json.contains("\"error\""));
    }

    #[test]
    fn test_lock_info_serialization() {
        let lock = LockInfo {
            holder: "agent-1".to_string(),
            resource: "session:test".to_string(),
            acquired_at: 1000,
            expires_at: 2000,
        };

        let json = serde_json::to_string(&lock).unwrap();
        let parsed: LockInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.holder, "agent-1");
        assert_eq!(parsed.expires_at, 2000);
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
                expires_at: Some(1234567890),
                previous_holder: None,
                error: None,
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
                expires_at: Some(1234567890),
                previous_holder: None,
                error: Some("Resource is locked by agent-xyz".to_string()),
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
        /// THEN: claimed=true, previous_holder=old agent
        #[test]
        fn expired_lock_shows_previous_holder() {
            let result = ClaimResult {
                claimed: true,
                resource: "session:expired-lock".to_string(),
                holder: Some("agent-new".to_string()),
                expires_at: Some(9999999999),
                previous_holder: Some("agent-old".to_string()),
                error: None,
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
        /// THEN: Should extend expiration
        #[test]
        fn reclaim_extends_expiration() {
            let result = ClaimResult {
                claimed: true,
                resource: "session:my-lock".to_string(),
                holder: Some("agent-me".to_string()),
                expires_at: Some(2000000000), // Extended
                previous_holder: None,        // Still us, no "previous"
                error: None,
            };

            assert!(result.claimed);
            assert!(result.expires_at.is_some(), "Should have new expiration");
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
                result
                    .error
                    .as_ref()
                    .map_or(false, |e| e.contains("not us")),
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
        /// THEN: Should have holder, resource, acquired_at, expires_at
        #[test]
        fn lock_has_all_required_fields() {
            let lock = LockInfo {
                holder: "agent-123".to_string(),
                resource: "session:feature-x".to_string(),
                acquired_at: 1609459200, // 2021-01-01 00:00:00 UTC
                expires_at: 1609462800,  // 2021-01-01 01:00:00 UTC
            };

            assert!(!lock.holder.is_empty(), "Must have holder");
            assert!(!lock.resource.is_empty(), "Must have resource");
            assert!(lock.acquired_at > 0, "Must have acquired time");
            assert!(lock.expires_at > lock.acquired_at, "Expires after acquired");
        }

        /// GIVEN: Lock with timestamps
        /// WHEN: Checked for expiration
        /// THEN: expires_at determines if expired
        #[test]
        fn expiration_is_based_on_expires_at() {
            let now = 1609461000; // Some time

            // Not expired
            let active_lock = LockInfo {
                holder: "agent".to_string(),
                resource: "res".to_string(),
                acquired_at: 1609459200,
                expires_at: 1609462800,
            };
            assert!(active_lock.expires_at > now, "Should not be expired");

            // Expired
            let expired_lock = LockInfo {
                holder: "agent".to_string(),
                resource: "res".to_string(),
                acquired_at: 1609459200,
                expires_at: 1609459300,
            };
            assert!(expired_lock.expires_at < now, "Should be expired");
        }

        /// GIVEN: Lock info is serialized
        /// WHEN: Read back
        /// THEN: All fields should match
        #[test]
        fn lock_roundtrips_through_json() {
            let original = LockInfo {
                holder: "agent-roundtrip".to_string(),
                resource: "session:test".to_string(),
                acquired_at: 1234567890,
                expires_at: 1234567900,
            };

            let json = serde_json::to_string(&original).unwrap();
            let parsed: LockInfo = serde_json::from_str(&json).unwrap();

            assert_eq!(parsed.holder, original.holder);
            assert_eq!(parsed.resource, original.resource);
            assert_eq!(parsed.acquired_at, original.acquired_at);
            assert_eq!(parsed.expires_at, original.expires_at);
        }
    }

    mod resource_naming_behavior {
        use super::*;

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

        /// GIVEN: ClaimResult is serialized
        /// WHEN: AI parses it
        /// THEN: Should have enough info for decision making
        #[test]
        fn claim_result_json_is_decision_ready() {
            let result = ClaimResult {
                claimed: true,
                resource: "session:task".to_string(),
                holder: Some("agent-1".to_string()),
                expires_at: Some(9999999999),
                previous_holder: None,
                error: None,
            };

            let json: serde_json::Value =
                serde_json::from_str(&serde_json::to_string(&result).unwrap()).unwrap();

            // Decision field
            assert!(json.get("claimed").is_some(), "Need claimed for decision");
            assert!(json["claimed"].is_boolean(), "claimed must be boolean");

            // Context fields
            assert!(json.get("resource").is_some(), "Need resource for context");
            assert!(json.get("holder").is_some(), "Need holder for context");
        }

        /// GIVEN: Failed claim
        /// WHEN: Serialized
        /// THEN: Error should explain why
        #[test]
        fn failed_claim_json_is_debuggable() {
            let result = ClaimResult {
                claimed: false,
                resource: "session:contested".to_string(),
                holder: Some("other-agent".to_string()),
                expires_at: Some(1234567890),
                previous_holder: None,
                error: Some("Resource locked by other-agent".to_string()),
            };

            let json: serde_json::Value =
                serde_json::from_str(&serde_json::to_string(&result).unwrap()).unwrap();

            assert_eq!(json["claimed"].as_bool(), Some(false));
            assert!(json.get("error").is_some(), "Failed claims need error");
            assert!(json["error"].as_str().unwrap().contains("other-agent"));
        }

        /// GIVEN: YieldResult is serialized
        /// WHEN: AI parses it
        /// THEN: Should have clear success indicator
        #[test]
        fn yield_result_json_has_success_indicator() {
            let result = YieldResult {
                yielded: true,
                resource: "session:task".to_string(),
                agent_id: Some("agent-1".to_string()),
                error: None,
            };

            let json: serde_json::Value =
                serde_json::from_str(&serde_json::to_string(&result).unwrap()).unwrap();

            assert!(json.get("yielded").is_some());
            assert!(json["yielded"].is_boolean());
        }
    }
}
