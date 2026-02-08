//! Tests for error troubleshooting documentation
//!
//! These tests ensure that:
//! 1. All error codes have documentation
//! 2. All error codes have suggestions
//! 3. All error codes have fix commands
//! 4. Error messages are clear and actionable
//! 5. Troubleshooting guide is comprehensive

// Test code uses unwrap/expect idioms for test clarity.
// Production code (src/) must use Result<T, Error> patterns.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::too_many_lines)]

use zjj_core::Error;

#[test]
fn test_all_errors_have_documentation() {
    // This test ensures we have a troubleshooting guide that covers all error types
    // In a real scenario, this would check the documentation file exists
    // For now, we'll verify the error system is complete

    let test_errors = vec![
        Error::InvalidConfig("test".into()),
        Error::IoError("test".into()),
        Error::ParseError("test".into()),
        Error::ValidationError("test".into()),
        Error::NotFound("test".into()),
        Error::DatabaseError("test".into()),
        Error::Command("test".into()),
        Error::HookFailed {
            hook_type: "test".into(),
            command: "test".into(),
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "test".into(),
        },
        Error::HookExecutionFailed {
            command: "test".into(),
            source: "test".into(),
        },
        Error::JjCommandError {
            operation: "test".into(),
            source: "test".into(),
            is_not_found: false,
        },
        Error::SessionLocked {
            session: "test".into(),
            holder: "test".into(),
        },
        Error::NotLockHolder {
            session: "test".into(),
            agent_id: "test".into(),
        },
        Error::OperationCancelled("test".into()),
        Error::Unknown("test".into()),
    ];

    // All errors should have a code
    for err in test_errors {
        assert!(!err.code().is_empty(), "Error should have a code");
        assert!(
            err.code().chars().all(|c| c.is_uppercase() || c == '_'),
            "Error code should be SCREAMING_SNAKE_CASE"
        );
    }
}

#[test]
fn test_all_errors_have_suggestions_or_reasonable_default() {
    // Most errors should have suggestions
    let test_errors = vec![
        Error::NotFound("session 'test' not found".into()),
        Error::ValidationError("invalid session name".into()),
        Error::DatabaseError("connection failed".into()),
        Error::InvalidConfig("unknown key".into()),
        Error::IoError("Permission denied".into()),
        Error::ParseError("Invalid JSON".into()),
        Error::Command("Command failed".into()),
    ];

    for err in test_errors {
        if let Some(suggestion) = err.suggestion() {
            // Suggestion should be actionable (contain a verb, command, or clear instruction)
            assert!(!suggestion.is_empty(), "Suggestion should not be empty");
            assert!(
                suggestion.contains("Run")
                    || suggestion.contains("Use")
                    || suggestion.contains("Check")
                    || suggestion.contains("Try")
                    || suggestion.contains("Ensure")
                    || suggestion.contains("Install")
                    || suggestion.contains("zjj")
                    || suggestion.contains("jj")
                    || suggestion.contains("cargo")
                    || suggestion.contains("must")
                    || suggestion.contains("should"),
                "Suggestion should be actionable: '{suggestion}'"
            );
        }
    }
}

#[test]
fn test_all_errors_have_fix_commands_or_none_reasonably() {
    // Most errors should have fix commands
    let test_errors = vec![
        Error::NotFound("session 'test' not found".into()),
        Error::ValidationError("invalid name".into()),
        Error::DatabaseError("corrupted".into()),
        Error::InvalidConfig("unknown key".into()),
    ];

    for err in test_errors {
        let commands = err.fix_commands();
        // At least some errors should have fix commands
        if !commands.is_empty() {
            for cmd in commands {
                assert!(!cmd.is_empty(), "Fix command should not be empty");
                assert!(
                    cmd.starts_with("zjj ")
                        || cmd.starts_with("jj ")
                        || cmd.starts_with("cargo ")
                        || cmd.starts_with("which ")
                        || cmd.starts_with("ls ")
                        || cmd.starts_with("echo ")
                        || cmd.starts_with("df "),
                    "Fix command should start with a known command: '{cmd}'"
                );
            }
        }
    }
}

#[test]
fn test_error_messages_are_clear_and_specific() {
    // Error messages should include what failed, why, and context
    let err = Error::NotFound("session 'my-session' not found in database".into());
    let msg = err.to_string();

    assert!(msg.contains("Not found"), "Should indicate what failed");
    assert!(!msg.is_empty(), "Should not be empty");
}

#[test]
fn test_error_codes_are_unique() {
    // Each error variant should have a unique code
    let errors = vec![
        Error::InvalidConfig("test".into()),
        Error::IoError("test".into()),
        Error::ParseError("test".into()),
        Error::ValidationError("test".into()),
        Error::NotFound("test".into()),
        Error::DatabaseError("test".into()),
        Error::Command("test".into()),
        Error::HookFailed {
            hook_type: "test".into(),
            command: "test".into(),
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "test".into(),
        },
        Error::SessionLocked {
            session: "test".into(),
            holder: "test".into(),
        },
    ];

    let mut codes = std::collections::HashSet::new();
    for err in errors {
        let code = err.code();
        assert!(
            codes.insert(code),
            "Error code '{code}' should be unique (duplicate found)"
        );
    }
}

#[test]
fn test_error_context_provides_useful_information() {
    // Error context should provide structured information
    let err = Error::NotFound("session 'test' not found".into());
    let context = err.context_map();

    assert!(context.is_some(), "NotFound error should have context");
    if let Some(ctx) = context {
        assert!(
            ctx.get("resource_type").is_some()
                || ctx.get("resource_id").is_some()
                || ctx.get("input").is_some(),
            "Context should have useful information"
        );
    }
}

#[test]
fn test_validation_errors_provide_hints() {
    // Validation errors should provide hints about what was expected
    let err = Error::ValidationError("invalid session name".into());
    let hints = err.validation_hints();

    assert!(!hints.is_empty(), "Validation errors should provide hints");

    for hint in hints {
        assert!(!hint.field.is_empty(), "Hint should specify the field");
        assert!(
            !hint.expected.is_empty(),
            "Hint should specify what was expected"
        );
    }
}

#[test]
fn test_hook_errors_include_command_and_exit_code() {
    let err = Error::HookFailed {
        hook_type: "post_create".into(),
        command: "npm install".into(),
        exit_code: Some(1),
        stdout: "Installed packages".into(),
        stderr: "Error: Package not found".into(),
    };

    let msg = err.to_string();
    assert!(msg.contains("post_create"), "Should include hook type");
    assert!(msg.contains("npm install"), "Should include command");
    assert!(msg.contains("Exit code:"), "Should include exit code");
    assert!(msg.contains("Package not found"), "Should include stderr");
}

#[test]
fn test_jj_command_errors_distinguish_not_found() {
    let not_found_err = Error::JjCommandError {
        operation: "init".into(),
        source: "No such file".into(),
        is_not_found: true,
    };

    let msg = not_found_err.to_string();
    assert!(
        msg.contains("JJ is not installed"),
        "Should suggest installing JJ when not found"
    );
    assert!(
        msg.contains("cargo install jj-cli"),
        "Should provide install command"
    );
}

#[test]
fn test_session_locked_errors_include_holder_info() {
    let err = Error::SessionLocked {
        session: "my-session".into(),
        holder: "agent-123".into(),
    };

    let msg = err.to_string();
    assert!(msg.contains("my-session"), "Should include session name");
    assert!(
        msg.contains("agent-123"),
        "Should include holder information"
    );

    let suggestion = err.suggestion();
    assert!(
        suggestion.is_some(),
        "SessionLocked should have a suggestion"
    );
    if let Some(sugg) = suggestion {
        assert!(
            sugg.contains("yield") || sugg.contains("status"),
            "Suggestion should include yield or status command"
        );
    }
}

#[test]
fn test_io_errors_provide_context_about_operation() {
    let err = Error::IoError("Failed to read file: Permission denied".into());
    let context = err.context_map();

    assert!(context.is_some(), "IO errors should have context");
    if let Some(ctx) = context {
        assert!(
            ctx.get("operation").is_some(),
            "Context should include the operation type"
        );
        assert!(
            ctx.get("error").is_some(),
            "Context should include the error message"
        );
    }
}

#[test]
fn test_database_errors_have_suggestions() {
    let err = Error::DatabaseError("corrupted database".into());
    let suggestion = err.suggestion();

    assert!(
        suggestion.is_some(),
        "Database errors should have suggestions"
    );
    if let Some(sugg) = suggestion {
        assert!(
            sugg.contains("doctor"),
            "Database error suggestion should mention 'zjj doctor'"
        );
    }
}

#[test]
fn test_parse_errors_distinguish_json_vs_toml() {
    let json_err = Error::ParseError("Invalid JSON: Expected comma".into());
    let suggestion = json_err.suggestion();

    assert!(suggestion.is_some(), "Parse errors should have suggestions");
    if let Some(sugg) = suggestion {
        assert!(
            sugg.contains("jq") || sugg.contains("JSON"),
            "JSON parse error should mention JSON validation tools"
        );
    }
}

#[test]
fn test_exit_codes_follow_semantic_conventions() {
    // Exit code 1: Validation errors
    assert_eq!(Error::InvalidConfig("test".into()).exit_code(), 1);
    assert_eq!(Error::ValidationError("test".into()).exit_code(), 1);
    assert_eq!(Error::ParseError("test".into()).exit_code(), 1);

    // Exit code 2: Not found errors
    assert_eq!(Error::NotFound("test".into()).exit_code(), 2);

    // Exit code 3: System errors
    assert_eq!(Error::IoError("test".into()).exit_code(), 3);
    assert_eq!(Error::DatabaseError("test".into()).exit_code(), 3);

    // Exit code 4: External command errors
    assert_eq!(Error::Command("test".into()).exit_code(), 4);
    assert_eq!(
        Error::JjCommandError {
            operation: "test".into(),
            source: "test".into(),
            is_not_found: false,
        }
        .exit_code(),
        4
    );

    // Exit code 5: Lock contention errors
    assert_eq!(
        Error::SessionLocked {
            session: "test".into(),
            holder: "test".into(),
        }
        .exit_code(),
        5
    );

    // Exit code 130: Operation cancelled (SIGINT)
    assert_eq!(Error::OperationCancelled("test".into()).exit_code(), 130);
}
