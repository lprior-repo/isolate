//! ATDD Test for bd-1ixz: Define StackError enum
//!
//! BEAD: bd-1ixz
//! REQUIREMENT: StackError enum for stack-related error conditions
//! EARS:
//!   - THE SYSTEM SHALL provide StackError enum for stack validation failures
//!   - WHEN cycle detected, THE SYSTEM SHALL provide CycleDetected variant with workspace and path
//!   - WHEN parent not found, THE SYSTEM SHALL provide ParentNotFound variant with parent name
//!   - WHEN depth exceeded, THE SYSTEM SHALL provide DepthExceeded variant with current and max
//!   - WHEN parent invalid, THE SYSTEM SHALL provide InvalidParent variant with workspace and
//!     reason
//!   - THE SYSTEM SHALL implement Display and Error traits
//!   - THE SYSTEM SHALL NOT use unwrap in error creation
//!
//! This test file should:
//!   1. COMPILE (type definitions are valid Rust)
//!   2. FAIL initially (StackError enum doesn't exist yet)
//!   3. PASS after implementation

#![allow(clippy::doc_markdown)]

use std::error::Error;

use zjj_core::coordination::stack_error::StackError;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: StackError::CycleDetected variant
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that StackError::CycleDetected contains workspace name and cycle path.
///
/// GIVEN: A cycle is detected in the workspace dependency graph
/// WHEN: Creating a CycleDetected error
/// THEN: The error should contain workspace name and cycle path
#[test]
fn test_stack_error_cycle_detected_has_context() {
    let error = StackError::CycleDetected {
        workspace: "feature-auth".to_string(),
        cycle_path: vec![
            "feature-auth".to_string(),
            "feature-db".to_string(),
            "feature-auth".to_string(),
        ],
    };

    // Verify workspace context
    assert_eq!(error.workspace(), "feature-auth");

    // Verify cycle path is captured
    let path = error.cycle_path();
    assert_eq!(path.len(), 3);
    assert_eq!(path[0], "feature-auth");
    assert_eq!(path[1], "feature-db");
    assert_eq!(path[2], "feature-auth");
}

/// Test CycleDetected Display shows clear message.
///
/// GIVEN: A CycleDetected error
/// WHEN: Converting to string via Display
/// THEN: The message should mention cycle and show the path
#[test]
fn test_stack_error_cycle_detected_display() {
    let error = StackError::CycleDetected {
        workspace: "feature-auth".to_string(),
        cycle_path: vec![
            "feature-auth".to_string(),
            "feature-db".to_string(),
            "feature-auth".to_string(),
        ],
    };

    let message = format!("{error}");

    // Message should be clear and actionable
    assert!(message.to_lowercase().contains("cycle"));
    assert!(message.contains("feature-auth"));
    assert!(message.contains("feature-db"));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: StackError::ParentNotFound variant
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that StackError::ParentNotFound contains parent workspace name.
///
/// GIVEN: A parent workspace reference cannot be resolved
/// WHEN: Creating a ParentNotFound error
/// THEN: The error should contain the missing parent's name
#[test]
fn test_stack_error_parent_not_found_has_context() {
    let error = StackError::ParentNotFound {
        parent_workspace: "feature-base".to_string(),
    };

    // Verify parent workspace context
    assert_eq!(error.parent_workspace(), "feature-base");
}

/// Test ParentNotFound Display shows clear message.
///
/// GIVEN: A ParentNotFound error
/// WHEN: Converting to string via Display
/// THEN: The message should mention the missing parent
#[test]
fn test_stack_error_parent_not_found_display() {
    let error = StackError::ParentNotFound {
        parent_workspace: "feature-base".to_string(),
    };

    let message = format!("{error}");

    // Message should be clear
    assert!(message.to_lowercase().contains("parent"));
    assert!(message.contains("feature-base"));
    assert!(
        message.to_lowercase().contains("not found") || message.to_lowercase().contains("missing")
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: StackError::DepthExceeded variant
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that StackError::DepthExceeded contains current and max depth.
///
/// GIVEN: Stack depth exceeds maximum allowed
/// WHEN: Creating a DepthExceeded error
/// THEN: The error should contain both current and max depth values
#[test]
fn test_stack_error_depth_exceeded_has_context() {
    let error = StackError::DepthExceeded {
        current_depth: 15,
        max_depth: 10,
    };

    // Verify depth context
    assert_eq!(error.current_depth(), 15);
    assert_eq!(error.max_depth(), 10);
}

/// Test DepthExceeded Display shows clear message.
///
/// GIVEN: A DepthExceeded error
/// WHEN: Converting to string via Display
/// THEN: The message should show both depth values
#[test]
fn test_stack_error_depth_exceeded_display() {
    let error = StackError::DepthExceeded {
        current_depth: 15,
        max_depth: 10,
    };

    let message = format!("{error}");

    // Message should show both values
    assert!(message.to_lowercase().contains("depth"));
    assert!(message.contains("15"));
    assert!(message.contains("10"));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: StackError::InvalidParent variant
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that StackError::InvalidParent contains workspace and reason.
///
/// GIVEN: A parent workspace exists but is in invalid state
/// WHEN: Creating an InvalidParent error
/// THEN: The error should contain workspace name and reason
#[test]
fn test_stack_error_invalid_parent_has_context() {
    let error = StackError::InvalidParent {
        workspace: "feature-auth".to_string(),
        reason: "parent workspace is in 'conflict' state".to_string(),
    };

    // Verify context
    assert_eq!(error.workspace(), "feature-auth");
    assert_eq!(error.reason(), "parent workspace is in 'conflict' state");
}

/// Test InvalidParent Display shows clear message.
///
/// GIVEN: An InvalidParent error
/// WHEN: Converting to string via Display
/// THEN: The message should mention workspace and reason
#[test]
fn test_stack_error_invalid_parent_display() {
    let error = StackError::InvalidParent {
        workspace: "feature-auth".to_string(),
        reason: "parent workspace is in 'conflict' state".to_string(),
    };

    let message = format!("{error}");

    // Message should be clear
    assert!(message.to_lowercase().contains("parent"));
    assert!(message.contains("feature-auth"));
    assert!(message.contains("conflict"));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: StackError implements std::error::Error
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that StackError implements std::error::Error trait.
///
/// GIVEN: StackError enum
/// WHEN: Using as a dyn Error
/// THEN: The trait should be implemented correctly
#[test]
fn test_stack_error_implements_std_error() {
    let error = StackError::CycleDetected {
        workspace: "test".to_string(),
        cycle_path: vec!["test".to_string()],
    };

    // This compiles only if StackError: std::error::Error
    let _: &dyn Error = &error;
}

/// Test that StackError can be used in Result types.
///
/// GIVEN: StackError enum
/// WHEN: Using in Result<T, StackError>
/// THEN: The type should work correctly with ? operator
#[test]
fn test_stack_error_in_result() {
    fn fallible_function() -> Result<(), StackError> {
        Err(StackError::DepthExceeded {
            current_depth: 5,
            max_depth: 3,
        })
    }

    let result = fallible_function();
    assert!(result.is_err());

    let error = result.err().unwrap_or_else(|| panic!("should have error"));
    let message = format!("{error}");
    assert!(message.contains('5'));
    assert!(message.contains('3'));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: StackError implements Display with clear messages
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that all StackError variants implement Display.
///
/// GIVEN: All StackError variants
/// WHEN: Converting to string via Display
/// THEN: Each variant should produce a non-empty, meaningful message
#[test]
fn test_all_stack_error_variants_have_display() {
    let variants: Vec<StackError> = vec![
        StackError::CycleDetected {
            workspace: "a".to_string(),
            cycle_path: vec!["a".to_string(), "b".to_string(), "a".to_string()],
        },
        StackError::ParentNotFound {
            parent_workspace: "parent".to_string(),
        },
        StackError::DepthExceeded {
            current_depth: 5,
            max_depth: 3,
        },
        StackError::InvalidParent {
            workspace: "child".to_string(),
            reason: "test reason".to_string(),
        },
    ];

    for error in variants {
        let message = format!("{error}");
        assert!(
            !message.is_empty(),
            "Display should produce non-empty message"
        );
    }
}

/// Test that StackError Display messages are human-readable.
///
/// GIVEN: StackError variants
/// WHEN: Converting to string
/// THEN: Messages should be clear and contain relevant context
#[test]
fn test_stack_error_display_messages_are_readable() {
    // CycleDetected should mention the cycle
    let cycle_error = StackError::CycleDetected {
        workspace: "ws".to_string(),
        cycle_path: vec!["ws".to_string(), "p".to_string(), "ws".to_string()],
    };
    let msg = format!("{cycle_error}");
    assert!(msg.len() > 10, "Message should be descriptive");

    // ParentNotFound should mention the parent
    let parent_error = StackError::ParentNotFound {
        parent_workspace: "parent".to_string(),
    };
    let msg = format!("{parent_error}");
    assert!(msg.len() > 5, "Message should be descriptive");

    // DepthExceeded should mention the limits
    let depth_error = StackError::DepthExceeded {
        current_depth: 10,
        max_depth: 5,
    };
    let msg = format!("{depth_error}");
    assert!(msg.len() > 5, "Message should be descriptive");

    // InvalidParent should mention the issue
    let invalid_error = StackError::InvalidParent {
        workspace: "ws".to_string(),
        reason: "conflict".to_string(),
    };
    let msg = format!("{invalid_error}");
    assert!(msg.len() > 5, "Message should be descriptive");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: No unwrap in error creation (compile-time check)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that StackError can be created without unwrap.
///
/// GIVEN: StackError constructors
/// WHEN: Creating errors from validated data
/// THEN: No unwrap should be needed
#[test]
fn test_stack_error_creation_no_unwrap() {
    // All variants should be constructible directly without unwrap
    let cycle = StackError::CycleDetected {
        workspace: "test".to_string(),
        cycle_path: vec!["test".to_string()],
    };

    let parent_not_found = StackError::ParentNotFound {
        parent_workspace: "parent".to_string(),
    };

    let depth_exceeded = StackError::DepthExceeded {
        current_depth: 5,
        max_depth: 3,
    };

    let invalid_parent = StackError::InvalidParent {
        workspace: "test".to_string(),
        reason: "test".to_string(),
    };

    // Suppress unused variable warnings
    drop(cycle);
    drop(parent_not_found);
    drop(depth_exceeded);
    drop(invalid_parent);

    // If this compiles, no unwrap was needed
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TEST: StackError Debug implementation
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Test that StackError implements Debug.
///
/// GIVEN: StackError variants
/// WHEN: Using Debug formatting
/// THEN: The output should show variant and fields
#[test]
fn test_stack_error_debug() {
    let error = StackError::CycleDetected {
        workspace: "test".to_string(),
        cycle_path: vec!["test".to_string()],
    };

    let debug_output = format!("{error:?}");

    // Debug should show the variant name
    assert!(debug_output.contains("CycleDetected"));
}

/// Test that StackError implements Clone.
///
/// GIVEN: StackError variant
/// WHEN: Cloning the error
/// THEN: A deep copy should be created
#[test]
fn test_stack_error_clone() {
    let error = StackError::DepthExceeded {
        current_depth: 5,
        max_depth: 3,
    };

    let cloned = error.clone();

    // Both should be independent
    assert_eq!(format!("{error}"), format!("{cloned}"));
}

/// Test that StackError implements PartialEq.
///
/// GIVEN: Two identical StackError instances
/// WHEN: Comparing them
/// THEN: They should be equal
#[test]
fn test_stack_error_partial_eq() {
    let error1 = StackError::DepthExceeded {
        current_depth: 5,
        max_depth: 3,
    };

    let error2 = StackError::DepthExceeded {
        current_depth: 5,
        max_depth: 3,
    };

    assert_eq!(error1, error2);

    let error3 = StackError::DepthExceeded {
        current_depth: 6,
        max_depth: 3,
    };

    assert_ne!(error1, error3);
}
