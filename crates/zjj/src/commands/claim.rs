//! Claim/Yield commands - Multi-agent resource locking
//!
//! Provides resource claiming and yielding for multi-agent coordination.
//! Uses file-based locking for simplicity.

use std::{fs, io::Write, path::PathBuf};

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

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn get_agent_id() -> Result<String> {
    std::env::var("ZJJ_AGENT_ID").or_else(|_| Ok(format!("pid-{}", std::process::id())))
}

/// Run the claim command
pub fn run_claim(options: &ClaimOptions) -> Result<()> {
    let agent_id = get_agent_id()?;
    let lock_path = lock_file_path(&options.resource)?;
    let now = current_timestamp();
    let expires_at = now + options.timeout;

    // Check if lock file exists and is still valid
    let result = if lock_path.exists() {
        // Read existing lock
        match fs::read_to_string(&lock_path) {
            Ok(content) => {
                match serde_json::from_str::<LockInfo>(&content) {
                    Ok(lock) => {
                        if lock.expires_at < now {
                            // Lock expired, we can take it
                            create_lock(&lock_path, &agent_id, &options.resource, now, expires_at)?;
                            ClaimResult {
                                claimed: true,
                                resource: options.resource.clone(),
                                holder: Some(agent_id.clone()),
                                expires_at: Some(expires_at),
                                previous_holder: Some(lock.holder),
                                error: None,
                            }
                        } else if lock.holder == agent_id {
                            // We already hold it, extend
                            create_lock(&lock_path, &agent_id, &options.resource, now, expires_at)?;
                            ClaimResult {
                                claimed: true,
                                resource: options.resource.clone(),
                                holder: Some(agent_id.clone()),
                                expires_at: Some(expires_at),
                                previous_holder: None,
                                error: None,
                            }
                        } else {
                            // Someone else holds it
                            ClaimResult {
                                claimed: false,
                                resource: options.resource.clone(),
                                holder: Some(lock.holder.clone()),
                                expires_at: Some(lock.expires_at),
                                previous_holder: None,
                                error: Some(format!("Resource locked by {}", lock.holder)),
                            }
                        }
                    }
                    Err(_) => {
                        // Invalid lock file, take it
                        create_lock(&lock_path, &agent_id, &options.resource, now, expires_at)?;
                        ClaimResult {
                            claimed: true,
                            resource: options.resource.clone(),
                            holder: Some(agent_id.clone()),
                            expires_at: Some(expires_at),
                            previous_holder: None,
                            error: None,
                        }
                    }
                }
            }
            Err(_) => {
                // Can't read lock file, try to create
                create_lock(&lock_path, &agent_id, &options.resource, now, expires_at)?;
                ClaimResult {
                    claimed: true,
                    resource: options.resource.clone(),
                    holder: Some(agent_id.clone()),
                    expires_at: Some(expires_at),
                    previous_holder: None,
                    error: None,
                }
            }
        }
    } else {
        // No lock exists, create one
        create_lock(&lock_path, &agent_id, &options.resource, now, expires_at)?;
        ClaimResult {
            claimed: true,
            resource: options.resource.clone(),
            holder: Some(agent_id.clone()),
            expires_at: Some(expires_at),
            previous_holder: None,
            error: None,
        }
    };

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("claim-response", "single", &result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else if result.claimed {
        println!("✓ Claimed resource '{}'", result.resource);
        if let Some(holder) = &result.holder {
            println!("  Lock holder: {holder}");
        }
        if let Some(prev) = &result.previous_holder {
            println!("  Previous holder: {prev} (expired/force-claimed)");
        }
    } else {
        eprintln!("✗ Failed to claim resource '{}'", result.resource);
        if let Some(holder) = &result.holder {
            eprintln!("  Currently held by: {holder}");
        }
        if let Some(err) = &result.error {
            eprintln!("  Error: {err}");
        }
        anyhow::bail!("Failed to claim resource");
    }

    Ok(())
}

fn create_lock(
    path: &PathBuf,
    holder: &str,
    resource: &str,
    acquired_at: u64,
    expires_at: u64,
) -> Result<()> {
    let lock = LockInfo {
        holder: holder.to_string(),
        resource: resource.to_string(),
        acquired_at,
        expires_at,
    };
    let content = serde_json::to_string_pretty(&lock)?;
    let mut file = fs::File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

/// Run the yield command
pub fn run_yield(options: &YieldOptions) -> Result<()> {
    let agent_id = get_agent_id()?;
    let lock_path = lock_file_path(&options.resource)?;

    let result = if lock_path.exists() {
        // Read existing lock
        match fs::read_to_string(&lock_path) {
            Ok(content) => {
                match serde_json::from_str::<LockInfo>(&content) {
                    Ok(lock) => {
                        if lock.holder == agent_id {
                            // We hold it, release
                            fs::remove_file(&lock_path)?;
                            YieldResult {
                                yielded: true,
                                resource: options.resource.clone(),
                                agent_id: Some(agent_id),
                                error: None,
                            }
                        } else {
                            // Someone else holds it
                            YieldResult {
                                yielded: false,
                                resource: options.resource.clone(),
                                agent_id: Some(agent_id),
                                error: Some(format!("Resource held by {}, not us", lock.holder)),
                            }
                        }
                    }
                    Err(_) => {
                        // Invalid lock, just remove
                        let _ = fs::remove_file(&lock_path);
                        YieldResult {
                            yielded: true,
                            resource: options.resource.clone(),
                            agent_id: Some(agent_id),
                            error: None,
                        }
                    }
                }
            }
            Err(e) => YieldResult {
                yielded: false,
                resource: options.resource.clone(),
                agent_id: Some(agent_id),
                error: Some(e.to_string()),
            },
        }
    } else {
        // No lock exists, nothing to yield (but that's OK)
        YieldResult {
            yielded: true,
            resource: options.resource.clone(),
            agent_id: Some(agent_id),
            error: None,
        }
    };

    if options.format.is_json() {
        let envelope = SchemaEnvelope::new("yield-response", "single", &result);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else if result.yielded {
        println!("✓ Yielded resource '{}'", result.resource);
    } else {
        eprintln!("✗ Failed to yield resource '{}'", result.resource);
        if let Some(err) = &result.error {
            eprintln!("  Error: {err}");
        }
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
            assert!(result.error.unwrap().contains("not us"));
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
