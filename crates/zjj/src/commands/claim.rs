//! Claim/Yield commands - Multi-agent resource locking
//!
//! Provides resource claiming and yielding for multi-agent coordination.
//! Uses file-based locking for simplicity.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

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
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
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
    std::env::var("ZJJ_AGENT_ID")
        .or_else(|_| Ok(format!("pid-{}", std::process::id())))
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

fn create_lock(path: &PathBuf, holder: &str, resource: &str, acquired_at: u64, expires_at: u64) -> Result<()> {
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
            Err(e) => {
                YieldResult {
                    yielded: false,
                    resource: options.resource.clone(),
                    agent_id: Some(agent_id),
                    error: Some(e.to_string()),
                }
            }
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
}
