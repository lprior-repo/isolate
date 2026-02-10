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

// Characterization tests for hooks.rs refactoring
//
// These tests capture current behavior of hooks.rs to ensure
// refactoring doesn't break functionality.

use zjj_core::hooks::HookType;

#[cfg(test)]
mod hook_type_event_names {
    use super::*;

    #[test]
    fn test_post_create_event_name() {
        assert_eq!(HookType::PostCreate.event_name(), "post_create");
    }

    #[test]
    fn test_pre_remove_event_name() {
        assert_eq!(HookType::PreRemove.event_name(), "pre_remove");
    }

    #[test]
    fn test_post_merge_event_name() {
        assert_eq!(HookType::PostMerge.event_name(), "post_merge");
    }
}

// This demonstrates RED phase:
// We're writing characterization tests that capture existing behavior
// before refactoring hooks.rs into modular structure
