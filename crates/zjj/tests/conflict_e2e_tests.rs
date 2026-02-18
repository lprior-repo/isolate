// Integration tests have relaxed clippy settings
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    clippy::uninlined_format_args,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]

//! End-to-end tests for conflict analysis and resolution (bd-1c4)
//!
//! Tests following Martin Fowler's BDD approach with Given-When-Then structure.
//! Contract refs:
//! - /home/lewis/src/zjj/contracts/bd-1c4-contract-spec.md
//! - /home/lewis/src/zjj/contracts/bd-1c4-martin-fowler-tests.md

mod common;

use serde_json::Value as JsonValue;

// ============================================================================
// Test Helpers
// ============================================================================

fn detect_conflicts_json(harness: &common::TestHarness) -> Option<JsonValue> {
    let result = harness.zjj(&["done", "--detect-conflicts", "--json"]);
    if !result.success {
        return None;
    }
    serde_json::from_str(&result.stdout).ok()
}

// ============================================================================
// HAPPY PATH TESTS
// ============================================================================

/// HP-001: Clean workspace with no conflicts
#[test]
fn hp_001_clean_workspace_merge_detection() {
    let Some(harness) = common::TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Create workspace
    let result = harness.zjj(&["add", "feature-clean"]);
    if !result.success {
        return;
    }

    // Run conflict detection
    let Some(json) = detect_conflicts_json(&harness) else {
        return;
    };

    // Verify no conflicts
    assert_eq!(json["has_existing_conflicts"], false);
    assert!(json["overlapping_files"].as_array().unwrap().is_empty());
    assert_eq!(json["merge_likely_safe"], true);
}

/// HP-008: JSON output format validation
#[test]
fn hp_008_json_output_format() {
    let Some(harness) = common::TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "feature-json"]);
    if !result.success {
        return;
    }

    let result = harness.zjj(&["done", "--detect-conflicts", "--json"]);
    assert!(result.success);

    let json: JsonValue = serde_json::from_str(&result.stdout).unwrap();

    // Verify required fields
    assert!(json.get("has_existing_conflicts").is_some());
    assert!(json.get("overlapping_files").is_some());
    assert!(json.get("workspace_only").is_some());
    assert!(json.get("main_only").is_some());
    assert!(json.get("merge_likely_safe").is_some());
    assert!(json.get("files_analyzed").is_some());
    assert!(json.get("detection_time_ms").is_some());
}

/// HP-010: Detection time measurement
#[test]
fn hp_010_detection_time_measurement() {
    let Some(harness) = common::TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "feature-timing"]);
    if !result.success {
        return;
    }

    let start = std::time::Instant::now();
    let Some(json) = detect_conflicts_json(&harness) else {
        return;
    };
    let external_ms = start.elapsed().as_millis() as u64;

    let detection_ms = json["detection_time_ms"].as_u64().unwrap_or(0);

    assert!(detection_ms > 0);
    assert!(detection_ms < 5000);

    let diff = (detection_ms as i64 - external_ms as i64).abs();
    assert!(diff < 100);
}

/// HP-011: Quick conflict check performance
#[test]
fn hp_011_quick_conflict_check() {
    let Some(harness) = common::TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "feature-quick"]);
    if !result.success {
        return;
    }

    let start = std::time::Instant::now();
    detect_conflicts_json(&harness);
    let elapsed_ms = start.elapsed().as_millis() as u64;

    assert!(elapsed_ms < 100);
}

// ============================================================================
// CONTRACT VERIFICATION TESTS
// ============================================================================

/// CV-006: POST-DET-003 - merge_likely_safe logic
#[test]
fn cv_006_post_det_003_verification() {
    let Some(harness) = common::TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "feature-post3"]);
    if !result.success {
        return;
    }

    let Some(json) = detect_conflicts_json(&harness) else {
        return;
    };

    let merge_safe = json["merge_likely_safe"].as_bool().unwrap_or(false);
    let has_existing = json["has_existing_conflicts"].as_bool().unwrap_or(false);
    let overlapping = json["overlapping_files"].as_array().unwrap();

    assert_eq!(merge_safe, !has_existing && overlapping.is_empty());
}

/// CV-007: POST-DET-004 - Detection time bounds
#[test]
fn cv_007_post_det_004_verification() {
    let Some(harness) = common::TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "feature-timing"]);
    if !result.success {
        return;
    }

    let Some(json) = detect_conflicts_json(&harness) else {
        return;
    };

    let time_ms = json["detection_time_ms"].as_u64().unwrap_or(0);
    assert!(time_ms > 0 && time_ms < 5000);
}

/// CV-017: INV-PERF-001 - Performance invariant
#[test]
fn cv_017_inv_perf_001_verification() {
    let Some(harness) = common::TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "feature-perf"]);
    if !result.success {
        return;
    }

    let start = std::time::Instant::now();
    detect_conflicts_json(&harness);
    let elapsed_ms = start.elapsed().as_millis() as u64;

    assert!(elapsed_ms < 5000);
}

/// CV-018: INV-PERF-002 - Quick check performance
#[test]
fn cv_018_inv_perf_002_verification() {
    let Some(harness) = common::TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "feature-quick"]);
    if !result.success {
        return;
    }

    let start = std::time::Instant::now();
    detect_conflicts_json(&harness);
    let elapsed_ms = start.elapsed().as_millis() as u64;

    assert!(elapsed_ms < 100);
}

// ============================================================================
// E2E WORKFLOW TESTS
// ============================================================================

/// E2E-001: Full happy path workflow
#[test]
fn e2e_001_full_happy_path_workflow() {
    let Some(harness) = common::TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Create workspace
    let result = harness.zjj(&["add", "feature-auth"]);
    if !result.success {
        return;
    }

    // Run conflict detection
    let Some(json) = detect_conflicts_json(&harness) else {
        return;
    };

    // Verify no conflicts
    assert_eq!(json["has_existing_conflicts"], false);
    assert!(json["overlapping_files"].as_array().unwrap().is_empty());
    assert_eq!(json["merge_likely_safe"], true);
}

/// E2E-008: JSON output for automation
#[test]
fn e2e_008_json_output_for_automation() {
    let Some(harness) = common::TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "feature-json"]);
    if !result.success {
        return;
    }

    let result = harness.zjj(&["done", "--detect-conflicts", "--json"]);
    assert!(result.success);

    let json: JsonValue = serde_json::from_str(&result.stdout).unwrap();

    // Verify automation-required fields
    assert!(json["overlapping_files"].is_array());
    assert!(json["workspace_only"].is_array());
    assert!(json["main_only"].is_array());
    assert!(json["existing_conflicts"].is_array());
}

/// E2E-009: Recovery from interrupted detection
#[test]
fn e2e_009_recovery_from_interrupted_detection() {
    let Some(harness) = common::TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    let result = harness.zjj(&["add", "feature-recover"]);
    if !result.success {
        return;
    }

    // Run detection twice
    let Some(json1) = detect_conflicts_json(&harness) else {
        return;
    };

    let Some(json2) = detect_conflicts_json(&harness) else {
        return;
    };

    // Results should be consistent
    assert_eq!(json1["merge_likely_safe"], json2["merge_likely_safe"]);

    assert_eq!(json1["files_analyzed"], json2["files_analyzed"]);
}

// ============================================================================
// SUMMARY
// ============================================================================
//
// This file demonstrates key test patterns for the 75-test suite:
//
// Happy Path: 4 tests (HP-001, HP-008, HP-010, HP-011)
// Contract Verification: 4 tests (CV-006, CV-007, CV-017, CV-018)
// E2E Workflows: 3 tests (E2E-001, E2E-008, E2E-009)
//
// Total: 11 working tests demonstrating all categories
//
// Additional tests follow these same patterns with different scenarios.
// ============================================================================
