//! ATDD Test for bd-1ued: Add StackMergeState enum
//!
//! BEAD: bd-1ued
//! REQUIREMENT: Add StackMergeState enum with 4 variants
//! EARS:
//!   - THE SYSTEM SHALL track stack merge state
//!   - WHEN state changes, THE SYSTEM SHALL use explicit transitions
//!   - IF invalid state string, THE SYSTEM SHALL NOT panic, must return error
//!
//! This test file should:
//!   1. COMPILE (enum definition is valid Rust)
//!   2. PASS after StackMergeState is implemented

#![allow(
    clippy::doc_markdown,
    clippy::clone_on_copy,
    clippy::no_effect_underscore_binding
)]

use std::str::FromStr;

use zjj_core::coordination::queue_status::StackMergeState;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: StackMergeState enum exists with 4 variants
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that all 4 variants exist and are distinct.
#[test]
fn test_stack_merge_state_variants() {
    let independent = StackMergeState::Independent;
    let blocked = StackMergeState::Blocked;
    let ready = StackMergeState::Ready;
    let merged = StackMergeState::Merged;

    assert_ne!(independent, blocked);
    assert_ne!(independent, ready);
    assert_ne!(independent, merged);
    assert_ne!(blocked, ready);
    assert_ne!(blocked, merged);
    assert_ne!(ready, merged);

    let _ = format!("{independent:?}");
    let _ = format!("{blocked:?}");
    let _ = format!("{ready:?}");
    let _ = format!("{merged:?}");
}

/// Test that Display outputs snake_case strings.
#[test]
fn test_stack_merge_state_display() {
    assert_eq!(StackMergeState::Independent.to_string(), "independent");
    assert_eq!(StackMergeState::Blocked.to_string(), "blocked");
    assert_eq!(StackMergeState::Ready.to_string(), "ready");
    assert_eq!(StackMergeState::Merged.to_string(), "merged");
}

/// Test FromStr parses all valid strings correctly.
#[test]
fn test_stack_merge_state_from_str_valid() -> Result<(), Box<dyn std::error::Error>> {
    let independent = StackMergeState::from_str("independent")?;
    let blocked = StackMergeState::from_str("blocked")?;
    let ready = StackMergeState::from_str("ready")?;
    let merged = StackMergeState::from_str("merged")?;

    assert_eq!(independent, StackMergeState::Independent);
    assert_eq!(blocked, StackMergeState::Blocked);
    assert_eq!(ready, StackMergeState::Ready);
    assert_eq!(merged, StackMergeState::Merged);

    Ok(())
}

/// Test FromStr returns error on invalid input.
#[test]
fn test_stack_merge_state_from_str_invalid() {
    let invalid_inputs = [
        "",
        "INDEPENDENT",
        "Independent",
        "INVALID",
        "independent ",
        " independent",
        "independent-blocked",
    ];

    for input in invalid_inputs {
        let result = StackMergeState::from_str(input);
        assert!(result.is_err(), "Expected error for input: '{input}'");
    }
}

/// Test Default trait returns Independent.
#[test]
fn test_stack_merge_state_default() {
    let default: StackMergeState = StackMergeState::default();
    assert_eq!(default, StackMergeState::Independent);
}

/// Test Copy and Clone traits work correctly.
#[test]
fn test_stack_merge_state_copy_clone() {
    let original = StackMergeState::Blocked;

    let copy: StackMergeState = original;
    assert_eq!(original, copy);
    assert_eq!(original, StackMergeState::Blocked);

    let cloned = original.clone();
    assert_eq!(original, cloned);
    assert_eq!(cloned, StackMergeState::Blocked);

    let _another_copy = original;
    assert_eq!(original, StackMergeState::Blocked);
}

/// Test roundtrip: Display -> FromStr -> back to enum.
#[test]
fn test_stack_merge_state_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let variants = [
        StackMergeState::Independent,
        StackMergeState::Blocked,
        StackMergeState::Ready,
        StackMergeState::Merged,
    ];

    for variant in variants {
        let display = variant.to_string();
        let parsed = StackMergeState::from_str(&display)?;
        assert_eq!(parsed, variant, "Roundtrip failed for {variant:?}");
    }

    Ok(())
}

/// Test PartialEq and Eq traits.
#[test]
fn test_stack_merge_state_equality() {
    assert_eq!(StackMergeState::Independent, StackMergeState::Independent);
    assert_eq!(StackMergeState::Blocked, StackMergeState::Blocked);
    assert_eq!(StackMergeState::Ready, StackMergeState::Ready);
    assert_eq!(StackMergeState::Merged, StackMergeState::Merged);

    assert_ne!(StackMergeState::Independent, StackMergeState::Blocked);
    assert_ne!(StackMergeState::Ready, StackMergeState::Merged);
}

/// Test as_str method returns correct strings.
#[test]
fn test_stack_merge_state_as_str() {
    assert_eq!(StackMergeState::Independent.as_str(), "independent");
    assert_eq!(StackMergeState::Blocked.as_str(), "blocked");
    assert_eq!(StackMergeState::Ready.as_str(), "ready");
    assert_eq!(StackMergeState::Merged.as_str(), "merged");
}

/// Test is_terminal method.
#[test]
fn test_stack_merge_state_is_terminal() {
    assert!(!StackMergeState::Independent.is_terminal());
    assert!(!StackMergeState::Blocked.is_terminal());
    assert!(!StackMergeState::Ready.is_terminal());
    assert!(StackMergeState::Merged.is_terminal());
}

/// Test is_blocked method.
#[test]
fn test_stack_merge_state_is_blocked() {
    assert!(!StackMergeState::Independent.is_blocked());
    assert!(StackMergeState::Blocked.is_blocked());
    assert!(!StackMergeState::Ready.is_blocked());
    assert!(!StackMergeState::Merged.is_blocked());
}

/// Test all() method returns all 4 variants.
#[test]
fn test_stack_merge_state_all() {
    let all = StackMergeState::all();
    assert_eq!(all.len(), 4);
    assert!(all.contains(&StackMergeState::Independent));
    assert!(all.contains(&StackMergeState::Blocked));
    assert!(all.contains(&StackMergeState::Ready));
    assert!(all.contains(&StackMergeState::Merged));
}

/// Test TryFrom<String> implementation.
#[test]
fn test_stack_merge_state_try_from() -> Result<(), Box<dyn std::error::Error>> {
    let status = StackMergeState::try_from("blocked".to_string())?;
    assert_eq!(status, StackMergeState::Blocked);

    let result = StackMergeState::try_from("invalid".to_string());
    assert!(result.is_err());

    Ok(())
}
