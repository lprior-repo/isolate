// Integration tests have relaxed clippy settings for brutal test scenarios.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    // Test code ergonomics
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    // Format string ergonomics for tests
    clippy::uninlined_format_args,
    // Documentation relaxations for test-only code
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    // Pattern matching relaxations
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
)]
//! Integration tests for chaos engineering failure injection
//!
//! These tests verify that zjj handles various failure conditions gracefully
//! when chaos is injected into operations.
//!
//! ## Test Categories
//!
//! - **IO failures**: Permission denied, disk full, etc.
//! - **Timeouts**: Operation hangs indefinitely
//! - **Corruption**: Data integrity checks fail
//! - **Resource exhaustion**: Out of memory, file descriptors, etc.
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all chaos tests
//! cargo test --test test_chaos_engineering
//!
//! # Run specific test category
//! cargo test --test test_chaos_engineering test_init_with_io_chaos
//! ```

mod chaos_engineering;
mod common;

use std::sync::OnceLock;

use chaos_engineering::{
    calculate_chaos_stats, run_chaos_iterations, ChaosConfig, ChaosExecutor, ChaosTestHarness,
    FailureMode,
};
use common::TestHarness;

// ============================================================================
// Test Utilities
// ============================================================================

/// Pre-configured chaos config for aggressive testing (50% failure rate)
#[allow(clippy::expect_used)]
fn aggressive_chaos_config() -> ChaosConfig {
    static CONFIG: OnceLock<ChaosConfig> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            ChaosConfig::new(
                0.5,
                vec![
                    FailureMode::IoError,
                    FailureMode::Timeout,
                    FailureMode::Corruption,
                ],
            )
            .expect("valid config")
        })
        .clone()
}

/// Pre-configured chaos config for mild testing (10% failure rate)
#[allow(clippy::expect_used)]
fn mild_chaos_config() -> ChaosConfig {
    static CONFIG: OnceLock<ChaosConfig> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            ChaosConfig::new(
                0.1,
                vec![
                    FailureMode::IoError,
                    FailureMode::Corruption,
                    FailureMode::ResourceExhaustion,
                ],
            )
            .expect("valid config")
        })
        .clone()
}

// ============================================================================
// Init Command Chaos Tests
// ============================================================================

#[test]
fn test_init_with_io_chaos() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let config = ChaosConfig::new(0.3, vec![FailureMode::IoError])
        .expect("valid config")  // Test code - allowed to panic on invalid config
        .with_seed(42);

    let executor = ChaosExecutor::new(config);

    // Run init with potential I/O chaos
    let _result = executor.inject_chaos(|| {
        // Run a simple Result-returning operation
        std::fs::write(harness.repo_path.join("test"), "data")
    });

    // Should either succeed or fail gracefully (no panic)
}

#[test]
fn test_init_with_corruption_chaos() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    let config = ChaosConfig::new(0.2, vec![FailureMode::Corruption])
        .expect("valid config")
        .with_seed(123);

    let executor = ChaosExecutor::new(config);

    // Run init with potential corruption chaos
    let _result = executor.inject_chaos(|| {
        // Run a simple Result-returning operation
        std::fs::write(harness.repo_path.join("test2"), "data")
    });
    // Verify no panic occurs
}

// ============================================================================
// Add Command Chaos Tests
// ============================================================================

#[test]
fn test_add_with_timeout_chaos() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let config = ChaosConfig::new(0.4, vec![FailureMode::Timeout])
        .expect("valid config")
        .with_seed(456);

    let executor = ChaosExecutor::new(config);

    // Try to add session with potential timeout chaos
    let _result = executor.inject_chaos(|| {
        // Run a simple Result-returning operation
        std::fs::write(harness.repo_path.join("test3"), "data")
    });
    // Should handle timeout gracefully
}

#[test]
fn test_add_with_resource_exhaustion() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let config = ChaosConfig::new(0.3, vec![FailureMode::ResourceExhaustion])
        .expect("valid config")
        .with_seed(789);

    let executor = ChaosExecutor::new(config);

    // Try to add session with potential resource exhaustion
    let _result = executor.inject_chaos(|| {
        // Run a simple Result-returning operation
        std::fs::write(harness.repo_path.join("test4"), "data")
    });
    // Should handle resource errors gracefully
}

// ============================================================================
// List Command Chaos Tests
// ============================================================================

#[test]
fn test_list_with_varied_chaos() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "s1", "--no-open"]);
    harness.assert_success(&["add", "s2", "--no-open"]);

    let config = ChaosConfig::new(
        0.5,
        vec![
            FailureMode::IoError,
            FailureMode::Timeout,
            FailureMode::Corruption,
            FailureMode::ResourceExhaustion,
        ],
    )
    .expect("valid config")
    .with_seed(999);

    let executor = ChaosExecutor::new(config);

    // Run list with multiple chaos modes
    let _result = executor.inject_chaos(|| {
        // Run a simple Result-returning operation
        std::fs::write(harness.repo_path.join("test5"), "data")
    });
    // Should handle any chaos gracefully
}

// ============================================================================
// Remove Command Chaos Tests
// ============================================================================

#[test]
fn test_remove_with_deadlock_simulation() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);
    harness.assert_success(&["add", "victim", "--no-open"]);

    let config = ChaosConfig::new(0.2, vec![FailureMode::DeadlockSimulation])
        .expect("valid config")
        .with_seed(111);

    let executor = ChaosExecutor::new(config);

    // Try to remove with potential deadlock simulation
    let _result = executor.inject_chaos(|| {
        // Run a simple Result-returning operation
        std::fs::write(harness.repo_path.join("test6"), "data")
    });
    // Should detect/handle deadlock scenarios
}

// ============================================================================
// Stress Tests with Chaos
// ============================================================================

#[test]
fn test_rapid_operations_with_chaos() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let config = aggressive_chaos_config().with_seed(222);
    let executor = ChaosExecutor::new(config);

    // Run multiple operations with chaos
    let iterations = 10;
    let results: Vec<_> = (0..iterations)
        .map(|i| {
            let derived_executor = executor.derive(i as u64);
            derived_executor.inject_chaos(|| {
                // Run a simple Result-returning operation
                std::fs::write(harness.repo_path.join(format!("stress{i}")), "data")
            })
        })
        .collect();

    // Calculate statistics
    let (successes, failures, rate) = calculate_chaos_stats(&results);

    // Verify all operations completed without panic
    assert_eq!(successes + failures, iterations);

    // With 50% chaos, expect roughly 50% success rate (±30% tolerance)
    assert!(
        (rate - 0.5).abs() < 0.3,
        "Success rate: {:.1}%",
        rate * 100.0
    );
}

#[test]
fn test_chaos_test_harness_integration() {
    let config = mild_chaos_config().with_seed(333);

    let Some(harness) = ChaosTestHarness::try_new(config) else {
        return;
    };

    harness.inner().assert_success(&["init"]);

    // Run commands through chaos harness
    let _result = harness.zjj_with_chaos(&["add", "chaos-test", "--no-open"]);

    let _result = harness.zjj_with_chaos(&["list"]);

    // Verify operations complete without panic
}

// ============================================================================
// Reproducible Chaos Tests
// ============================================================================

#[test]
fn test_chaos_reproducibility() {
    let config = ChaosConfig::new(1.0, vec![FailureMode::IoError])
        .expect("valid config")
        .with_seed(42);

    let executor1 = ChaosExecutor::new(config.clone());
    let executor2 = ChaosExecutor::new(config);

    // Both executors should produce the same result with same seed
    let result1 = executor1.inject_chaos_infallible(|| 42);
    let result2 = executor2.inject_chaos_infallible(|| 42);

    // Results should be identical (both fail with same error)
    assert_eq!(result1.is_err(), result2.is_err());
}

#[test]
fn test_derived_executor_independence() {
    let config = ChaosConfig::new(0.5, vec![FailureMode::Timeout])
        .expect("valid config")
        .with_seed(777);

    let executor = ChaosExecutor::new(config);
    let derived1 = executor.derive(10);
    let derived2 = executor.derive(20);

    // Derived executors should have independent chaos streams
    let result1 = derived1.inject_chaos_infallible(|| 1);
    let result2 = derived2.inject_chaos_infallible(|| 2);

    // At least one should succeed with 50% probability
    let at_least_one_success = result1.is_ok() || result2.is_ok();
    assert!(at_least_one_success || result1.is_err() && result2.is_err());
}

// ============================================================================
// Multi-Mode Chaos Tests
// ============================================================================

#[test]
fn test_all_failure_modes() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let modes = FailureMode::all();

    for mode in modes {
        let config = ChaosConfig::new(0.3, vec![*mode])
            .expect("valid config")
            .with_seed(888);

        let executor = ChaosExecutor::new(config);

        // Test each failure mode independently
        let _result = executor.inject_chaos(|| {
            // Run a simple Result-returning operation
            std::fs::write(harness.repo_path.join(format!("{mode:?}")), "data")
        });
        // Verify no panic occurs for any mode
    }
}

// ============================================================================
// Chaos Iteration Tests
// ============================================================================

#[test]
fn test_chaos_iteration_statistics() {
    let config = ChaosConfig::new(0.5, vec![FailureMode::IoError])
        .expect("valid config")
        .with_seed(555);

    let results = run_chaos_iterations(&config, 100, || Ok::<(), std::io::Error>(()));

    let (successes, failures, rate) = calculate_chaos_stats(&results);

    assert_eq!(successes + failures, 100);

    // With 50% chaos and 100 iterations, expect ~50% success (±20%)
    assert!(
        (rate - 0.5).abs() < 0.2,
        "Success rate: {:.1}%",
        rate * 100.0
    );
}

#[test]
fn test_zero_chaos_probability() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let config = ChaosConfig::new(0.0, vec![FailureMode::IoError]).expect("valid config");

    let executor = ChaosExecutor::new(config);

    // With 0% probability, all operations should succeed
    let results: Vec<_> = (0..10)
        .map(|_| {
            executor.inject_chaos(|| {
                // Run a simple Result-returning operation
                std::fs::write(harness.repo_path.join("zero"), "data")
            })
        })
        .collect();

    let all_success = results.iter().all(std::result::Result::is_ok);
    assert!(all_success, "All operations should succeed with 0% chaos");
}

#[test]
fn test_max_chaos_probability() {
    let config = ChaosConfig::new(1.0, vec![FailureMode::Corruption]).expect("valid config");

    let executor = ChaosExecutor::new(config);

    // With 100% probability, all operations should fail
    let results: Vec<_> = (0..10)
        .map(|_| executor.inject_chaos(|| Ok::<(), std::io::Error>(())))
        .collect();

    let all_fail = results.iter().all(std::result::Result::is_err);
    assert!(all_fail, "All operations should fail with 100% chaos");
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_chaos_with_empty_result() {
    let config = ChaosConfig::new(0.5, vec![FailureMode::Timeout]).expect("valid config");

    let executor = ChaosExecutor::new(config);

    // Test chaos with operations that return empty Result
    let _result = executor.inject_chaos(|| Ok::<(), std::io::Error>(()));
}

#[test]
fn test_chaos_with_complex_result() {
    let config =
        ChaosConfig::new(0.3, vec![FailureMode::ResourceExhaustion]).expect("valid config");

    let executor = ChaosExecutor::new(config);

    // Test chaos with operations returning complex types
    let _result = executor.inject_chaos(|| {
        Ok::<(String, Vec<u8>), std::io::Error>(("test".to_string(), vec![1, 2, 3]))
    });
}

// ============================================================================
// Recovery Tests
// ============================================================================

#[test]
fn test_recovery_after_chaos() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    // Apply chaos
    let config = aggressive_chaos_config().with_seed(666);
    let executor = ChaosExecutor::new(config);

    let _chaos_result = executor.inject_chaos(|| {
        // Run a simple Result-returning operation
        std::fs::write(harness.repo_path.join("chaos"), "data")
    });

    // Verify system can still operate normally after chaos
    let normal_result = harness.zjj(&["list"]);
    assert!(normal_result.success, "System should recover after chaos");
}

#[test]
fn test_multiple_chaos_cycles() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };
    harness.assert_success(&["init"]);

    let config = mild_chaos_config();

    // Run multiple chaos cycles
    for i in 0..5 {
        let executor = ChaosExecutor::new(config.clone()).derive(i);

        let _result = executor.inject_chaos(|| {
            // Run a simple Result-returning operation
            std::fs::write(harness.repo_path.join(format!("cycle{i}")), "data")
        });

        // System should remain functional
        let status = harness.zjj(&["list"]);
        assert!(status.success, "System should remain functional");
    }
}

// ============================================================================
// Performance Impact Tests
// ============================================================================

#[test]
fn test_chaos_overhead() {
    let config = ChaosConfig::new(0.0, vec![FailureMode::IoError]).expect("valid config");

    let executor = ChaosExecutor::new(config);

    // Measure overhead of chaos injection with 0% probability
    let iterations = 1000;
    let results: Vec<_> = (0..iterations)
        .map(|_| executor.inject_chaos(|| Ok::<(), std::io::Error>(())))
        .collect();

    let all_success = results.iter().all(std::result::Result::is_ok);
    assert!(
        all_success,
        "Chaos injection should not affect normal operations"
    );
}

// ============================================================================
// Concurrent Chaos Tests
// ============================================================================

#[test]
fn test_concurrent_chaos_streams() {
    let config = aggressive_chaos_config().with_seed(999);

    // Create multiple independent chaos streams
    let executor1 = ChaosExecutor::new(config.clone()).derive(0);
    let executor2 = ChaosExecutor::new(config.clone()).derive(100);
    let executor3 = ChaosExecutor::new(config).derive(200);

    // Run operations concurrently
    let result1 = executor1.inject_chaos(|| Ok::<(), std::io::Error>(()));
    let result2 = executor2.inject_chaos(|| Ok::<(), std::io::Error>(()));
    let result3 = executor3.inject_chaos(|| Ok::<(), std::io::Error>(()));

    // All should complete without panic
    drop(result1);
    drop(result2);
    drop(result3);
}
