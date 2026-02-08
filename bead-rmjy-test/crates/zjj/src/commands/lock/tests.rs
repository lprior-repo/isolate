//! Tests for lock command

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use chrono::Utc;
use zjj_core::coordination::locks::LockManager;

use super::{
    run_lock_async, run_unlock_async,
    types::{LockArgs, UnlockArgs},
};

/// Helper to create test database pool
async fn test_pool() -> Result<sqlx::SqlitePool, zjj_core::Error> {
    use sqlx::sqlite::SqlitePoolOptions;
    SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .map_err(|e| zjj_core::Error::DatabaseError(e.to_string()))
}

/// Helper to setup lock manager for tests
async fn setup_lock_manager() -> Result<LockManager, zjj_core::Error> {
    let pool = test_pool().await?;
    let mgr = LockManager::new(pool);
    mgr.init().await?;
    Ok(mgr)
}

// EARS 1: WHEN lock runs, system shall acquire exclusive lock on session for agent_id
#[tokio::test]
async fn test_lock_acquires_successfully() -> anyhow::Result<()> {
    let mgr = setup_lock_manager().await?;
    let args = LockArgs {
        session: "test-session".to_string(),
        agent_id: Some("agent1".to_string()),
        ttl: 300,
    };

    let output = run_lock_async(&args, &mgr).await?;

    assert!(output.success);
    assert!(output.locked);
    assert!(output.lock_id.is_some());
    assert_eq!(output.holder, "agent1");
    assert_eq!(output.session, "test-session");
    assert!(output.expires_at.is_some());

    Ok(())
}

// EARS 2: WHEN lock is held by another agent, system shall return SESSION_LOCKED error with holder
// info
#[tokio::test]
async fn test_lock_fails_if_held_by_another() -> anyhow::Result<()> {
    let mgr = setup_lock_manager().await?;

    let args1 = LockArgs {
        session: "test-session".to_string(),
        agent_id: Some("agent1".to_string()),
        ttl: 300,
    };
    let _ = run_lock_async(&args1, &mgr).await?;

    let args2 = LockArgs {
        session: "test-session".to_string(),
        agent_id: Some("agent2".to_string()),
        ttl: 300,
    };
    let result = run_lock_async(&args2, &mgr).await;

    assert!(result.is_err());
    let err = result
        .err()
        .ok_or_else(|| anyhow::anyhow!("Expected error"))?;
    assert!(err.to_string().contains("SESSION_LOCKED"));

    Ok(())
}

// EARS 2 cont: Output should show holder on conflict
#[tokio::test]
async fn test_lock_output_shows_holder_on_conflict() -> anyhow::Result<()> {
    let mgr = setup_lock_manager().await?;

    let args1 = LockArgs {
        session: "test-session".to_string(),
        agent_id: Some("agent1".to_string()),
        ttl: 300,
    };
    let _ = run_lock_async(&args1, &mgr).await?;

    let args2 = LockArgs {
        session: "test-session".to_string(),
        agent_id: Some("agent2".to_string()),
        ttl: 300,
    };
    let result = run_lock_async(&args2, &mgr).await;

    assert!(result.is_err());
    let err_msg = result
        .err()
        .ok_or_else(|| anyhow::anyhow!("Expected error"))?
        .to_string();
    assert!(err_msg.contains("agent1"));

    Ok(())
}

// EARS 3: WHEN unlock runs, system shall release lock if held by requesting agent
#[tokio::test]
async fn test_unlock_releases_lock() -> anyhow::Result<()> {
    let mgr = setup_lock_manager().await?;

    let lock_args = LockArgs {
        session: "test-session".to_string(),
        agent_id: Some("agent1".to_string()),
        ttl: 300,
    };
    let _ = run_lock_async(&lock_args, &mgr).await?;

    let unlock_args = UnlockArgs {
        session: "test-session".to_string(),
        agent_id: Some("agent1".to_string()),
    };
    let output = run_unlock_async(&unlock_args, &mgr).await?;

    assert!(output.success);
    assert!(output.released);
    assert_eq!(output.session, "test-session");

    let lock_args2 = LockArgs {
        session: "test-session".to_string(),
        agent_id: Some("agent2".to_string()),
        ttl: 300,
    };
    let output2 = run_lock_async(&lock_args2, &mgr).await?;
    assert_eq!(output2.holder, "agent2");

    Ok(())
}

// EARS 4: WHEN unlock by non-holder, system shall return NOT_LOCK_HOLDER error
#[tokio::test]
async fn test_unlock_fails_for_non_holder() -> anyhow::Result<()> {
    let mgr = setup_lock_manager().await?;

    let lock_args = LockArgs {
        session: "test-session".to_string(),
        agent_id: Some("agent1".to_string()),
        ttl: 300,
    };
    let _ = run_lock_async(&lock_args, &mgr).await?;

    let unlock_args = UnlockArgs {
        session: "test-session".to_string(),
        agent_id: Some("agent2".to_string()),
    };
    let result = run_unlock_async(&unlock_args, &mgr).await;

    assert!(result.is_err());
    let err_msg = result
        .err()
        .ok_or_else(|| anyhow::anyhow!("Expected error"))?
        .to_string();
    assert!(err_msg.contains("NOT_LOCK_HOLDER") || err_msg.contains("not the lock holder"));

    Ok(())
}

// EARS 5: WHEN agent_id not provided, use ZJJ_AGENT_ID env var
#[tokio::test]
async fn test_lock_uses_env_agent_id() -> anyhow::Result<()> {
    let mgr = setup_lock_manager().await?;

    std::env::set_var("ZJJ_AGENT_ID", "env-agent");

    let args = LockArgs {
        session: "test-session".to_string(),
        agent_id: None,
        ttl: 300,
    };

    let output = run_lock_async(&args, &mgr).await?;

    assert_eq!(output.holder, "env-agent");

    std::env::remove_var("ZJJ_AGENT_ID");

    Ok(())
}

// EARS 6: WHEN --ttl specified, system shall use custom TTL
#[tokio::test]
async fn test_lock_respects_custom_ttl() -> anyhow::Result<()> {
    let mgr = setup_lock_manager().await?;

    let args = LockArgs {
        session: "test-session".to_string(),
        agent_id: Some("agent1".to_string()),
        ttl: 60,
    };

    let output = run_lock_async(&args, &mgr).await?;

    assert_eq!(output.ttl_seconds, 60);

    let expires = output
        .expires_at
        .ok_or_else(|| anyhow::anyhow!("No expires_at"))?;
    let now = Utc::now();
    let diff = (expires - now).num_seconds();

    assert!((diff - 60).abs() < 5, "TTL difference too large: {diff}");

    Ok(())
}
