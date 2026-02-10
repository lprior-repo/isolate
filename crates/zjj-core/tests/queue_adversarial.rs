
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
//! Adversarial tests for queue - boundary conditions and edge cases
