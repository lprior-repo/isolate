//! Lock command implementation
//!
//! Manages resource locking for multi-agent coordination.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

use anyhow::Result;
use stak_core::LockManager;

/// Lock command options
#[derive(Debug, Clone)]
pub struct LockOptions {
    /// Acquire lock
    pub acquire: bool,
    /// Release lock
    pub release: bool,
    /// Show lock status
    pub status: bool,
    /// Resource to lock
    pub resource: Option<String>,
    /// Lock timeout in seconds
    pub ttl: i64,
}

/// Run the lock command
///
/// # Errors
///
/// Returns an error if:
/// - Resource name is invalid
/// - Resource is already locked (acquire)
/// - Lock is held by another agent (release)
pub fn run(options: &LockOptions, manager: &mut LockManager) -> Result<()> {
    if options.acquire {
        handle_acquire(options, manager)
    } else if options.release {
        handle_release(options, manager)
    } else if options.status {
        handle_status(options, manager)
    } else {
        anyhow::bail!("Lock subcommand required (acquire, release, or status)");
    }
}

/// Handle acquire command
fn handle_acquire(options: &LockOptions, manager: &mut LockManager) -> Result<()> {
    let resource = options
        .resource
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Resource required for acquire"))?;

    validate_resource_name(resource)?;

    let agent_id = get_agent_id();

    if let Some(lock) = manager.acquire(resource, &agent_id, options.ttl) {
        println!("✓ Acquired lock on '{resource}'");
        println!("  Holder: {}", lock.holder);
        println!("  Expires: {}", lock.expires_at.to_rfc3339());
        Ok(())
    } else {
        let holder = manager.get_holder(resource);
        if let Some(h) = holder {
            anyhow::bail!("Resource '{resource}' is locked by {h}");
        }
        anyhow::bail!("Failed to acquire lock on '{resource}'");
    }
}

/// Handle release command
fn handle_release(options: &LockOptions, manager: &mut LockManager) -> Result<()> {
    let resource = options
        .resource
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Resource required for release"))?;

    let agent_id = get_agent_id();

    if manager.release(resource, &agent_id) {
        println!("✓ Released lock on '{resource}'");
        Ok(())
    } else if manager.is_locked(resource) {
        let holder = manager.get_holder(resource).map_or("unknown", |s| s);
        anyhow::bail!("Lock on '{resource}' is held by {holder}, not you");
    } else {
        println!("Lock on '{resource}' was not held");
        Ok(())
    }
}

/// Handle status command
fn handle_status(_options: &LockOptions, manager: &LockManager) -> Result<()> {
    let locks = manager.active_locks();

    if locks.is_empty() {
        println!("No active locks");
        return Ok(());
    }

    println!("Active Locks ({}):", locks.len());
    for lock in locks {
        println!(
            "  {} held by {} (expires: {})",
            lock.resource,
            lock.holder,
            lock.expires_at.to_rfc3339()
        );
    }

    Ok(())
}

/// Get the current agent ID
fn get_agent_id() -> String {
    std::env::var("STAK_AGENT_ID").unwrap_or_else(|_| format!("pid-{}", std::process::id()))
}

/// Reserved keywords that cannot be used as resource names
const RESERVED_KEYWORDS: &[&str] = &["null", "undefined", "true", "false", "none", "nil", "void"];

/// Validate a resource name
fn validate_resource_name(resource: &str) -> Result<()> {
    let trimmed = resource.trim();

    if trimmed.is_empty() {
        anyhow::bail!("Resource name cannot be empty or whitespace-only");
    }

    // Check for reserved keywords (case-insensitive)
    let lower = trimmed.to_lowercase();
    if RESERVED_KEYWORDS.iter().any(|&keyword| keyword == lower) {
        anyhow::bail!("Resource name '{trimmed}' is a reserved keyword");
    }

    // Check if name contains at least one alphanumeric character
    if !trimmed.chars().any(char::is_alphanumeric) {
        anyhow::bail!("Resource name must contain at least one alphanumeric character");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_resource_name() {
        assert!(validate_resource_name("session:test").is_ok());
        assert!(validate_resource_name("valid-name").is_ok());
        assert!(validate_resource_name("").is_err());
        assert!(validate_resource_name("null").is_err());
        assert!(validate_resource_name(":::").is_err());
    }

    #[test]
    fn test_acquire_release_lock() -> Result<()> {
        let mut manager = LockManager::new();

        let acquire_options = LockOptions {
            acquire: true,
            release: false,
            status: false,
            resource: Some("test-resource".to_string()),
            ttl: 60,
        };

        run(&acquire_options, &mut manager)?;
        assert!(manager.is_locked("test-resource"));

        let release_options = LockOptions {
            acquire: false,
            release: true,
            status: false,
            resource: Some("test-resource".to_string()),
            ttl: 0,
        };

        run(&release_options, &mut manager)?;
        assert!(!manager.is_locked("test-resource"));

        Ok(())
    }
}
