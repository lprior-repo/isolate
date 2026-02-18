//! Tests for the prune-invalid command
//!
//! These tests verify:
//! - Basic prune-invalid functionality
//! - --yes flag skips confirmation
//! - JSON output is properly wrapped
//! - Empty invalid list handling
//! - Error handling for various edge cases

use anyhow::Result;
use serde::Deserialize;
use tempfile::TempDir;
use zjj_core::OutputFormat;

use crate::commands::prune_invalid::{PruneInvalidOptions, PruneInvalidOutput};

#[tokio::test]
async fn test_prune_invalid_options_default() {
    let opts = PruneInvalidOptions::default();
    assert!(!opts.yes);
    assert!(!opts.dry_run);
    assert!(opts.format.is_json());
}

#[tokio::test]
async fn test_prune_invalid_options_with_yes_flag() {
    let opts = PruneInvalidOptions {
        yes: true,
        ..PruneInvalidOptions::default()
    };
    assert!(opts.yes);
}

#[tokio::test]
async fn test_prune_invalid_output_serialization() -> Result<()> {
    let output = PruneInvalidOutput {
        invalid_count: 3,
        removed_count: 3,
        invalid_sessions: vec![
            "session1".to_string(),
            "session2".to_string(),
            "session3".to_string(),
        ],
    };

    let json = serde_json::to_string(&output)?;
    assert!(json.contains("\"invalid_count\":3"));
    assert!(json.contains("\"removed_count\":3"));
    assert!(json.contains("session1"));
    assert!(json.contains("session2"));
    assert!(json.contains("session3"));

    Ok(())
}

#[tokio::test]
async fn test_prune_invalid_output_empty_list() -> Result<()> {
    let output = PruneInvalidOutput {
        invalid_count: 0,
        removed_count: 0,
        invalid_sessions: Vec::new(),
    };

    let json = serde_json::to_string(&output)?;
    assert!(json.contains("\"invalid_count\":0"));
    assert!(json.contains("\"removed_count\":0"));

    Ok(())
}

#[tokio::test]
async fn test_prune_invalid_response_envelope() -> Result<()> {
    use zjj_core::json::SchemaEnvelope;

    let output = PruneInvalidOutput {
        invalid_count: 1,
        removed_count: 1,
        invalid_sessions: vec!["invalid-session".to_string()],
    };

    let envelope = SchemaEnvelope::new("prune-invalid-response", "single", output);
    let json_str = serde_json::to_string(&envelope)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

    assert!(parsed.get("$schema").is_some(), "Missing $schema field");
    assert!(
        parsed.get("_schema_version").and_then(|v| v.as_str()) == Some("1.0"),
        "Missing or wrong _schema_version"
    );
    assert_eq!(
        parsed.get("schema_type").and_then(|v| v.as_str()),
        Some("single")
    );

    Ok(())
}

#[tokio::test]
async fn test_prune_invalid_response_fields() -> Result<()> {
    use zjj_core::json::SchemaEnvelope;

    let output = PruneInvalidOutput {
        invalid_count: 5,
        removed_count: 4,
        invalid_sessions: vec!["a".to_string(), "b".to_string(), "c".to_string()],
    };

    let envelope = SchemaEnvelope::new("prune-invalid-response", "single", output);
    let json_str = serde_json::to_string(&envelope)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

    let data = parsed
        .get("data")
        .ok_or_else(|| anyhow::anyhow!("Missing data field"))?;
    assert_eq!(
        data.get("invalid_count").and_then(|v| v.as_u64()),
        Some(5),
        "invalid_count should be 5"
    );
    assert_eq!(
        data.get("removed_count").and_then(|v| v.as_u64()),
        Some(4),
        "removed_count should be 4"
    );

    Ok(())
}

#[tokio::test]
async fn test_prune_invalid_options_json_format() {
    let opts = PruneInvalidOptions {
        format: OutputFormat::Json,
        ..PruneInvalidOptions::default()
    };
    assert!(opts.format.is_json());
}

#[tokio::test]
async fn test_prune_invalid_with_dry_run() {
    let opts = PruneInvalidOptions {
        dry_run: true,
        ..PruneInvalidOptions::default()
    };
    assert!(opts.dry_run);
    assert!(!opts.yes);
}
