//! Property-based tests for status object invariants using proptest.
//!
//! This is the RED phase - these tests MUST FAIL initially until implementation is complete.
//!
//! # Invariants tested:
//! - JSON validity: All status output must be valid JSON
//! - Field completeness: Required fields must always be present
//! - State consistency: State transitions must be valid
//!
//! Run with: cargo test --test status_properties
//! Reproducible: Set PROPTEST_SEED environment variable for deterministic runs

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use isolate_core::output::{OutputLine, SessionOutput, Summary, SummaryType};
use isolate_core::types::SessionStatus;
use isolate_core::WorkspaceState;
use proptest::prelude::*;
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// CUSTOM STRATEGIES FOR GENERATING TEST DATA
// ═══════════════════════════════════════════════════════════════════════════

/// Generate valid session names according to SessionName contract
fn session_name_strategy() -> impl Strategy<Value = String> {
    // Session name must:
    // - Start with ASCII letter
    // - Contain only alphanumeric, dash, underscore
    // - Max 64 characters
    "[a-zA-Z][a-zA-Z0-9_-]{0,63}".prop_map(|s| s)
}

/// Generate any string (including invalid ones)
fn any_string_strategy() -> impl Strategy<Value = String> {
    ".*"
}

/// Generate valid session statuses
fn session_status_strategy() -> impl Strategy<Value = SessionStatus> {
    prop_oneof![
        Just(SessionStatus::Creating),
        Just(SessionStatus::Active),
        Just(SessionStatus::Paused),
        Just(SessionStatus::Completed),
        Just(SessionStatus::Failed),
    ]
}

/// Generate workspace states
fn workspace_state_strategy() -> impl Strategy<Value = WorkspaceState> {
    prop_oneof![
        Just(WorkspaceState::Created),
        Just(WorkspaceState::Working),
        Just(WorkspaceState::Ready),
        Just(WorkspaceState::Merged),
        Just(WorkspaceState::Abandoned),
        Just(WorkspaceState::Conflict),
    ]
}

/// Generate absolute paths
fn absolute_path_strategy() -> impl Strategy<Value = PathBuf> {
    // Generate absolute paths starting with /tmp/
    "[a-zA-Z0-9_-]{1,20}".prop_map(|s| PathBuf::from(format!("/tmp/isolate-test-{}", s)))
}

/// Generate optional branch names
fn branch_name_strategy() -> impl Strategy<Value = Option<String>> {
    prop_oneof![Just(None), "[a-zA-Z][a-zA-Z0-9_-]{0,30}".prop_map(Some),]
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 1: JSON VALIDITY
// ═══════════════════════════════════════════════════════════════════════════

/// A wrapper type for parsing JSON output
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatusOutput {
    #[serde(rename = "type")]
    pub type_field: String,
    pub session: Option<SessionPayload>,
    pub summary: Option<SummaryPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionPayload {
    pub name: String,
    pub status: String,
    pub state: Option<String>,
    pub workspace_path: Option<String>,
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SummaryPayload {
    #[serde(rename = "type")]
    pub summary_type: Option<String>,
    pub message: Option<String>,
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: SessionOutput serializes to valid JSON
    ///
    /// INVARIANT: JSON valid - all status output must be valid JSON
    /// RED PHASE: This test MUST FAIL if SessionOutput cannot be serialized
    #[test]
    fn prop_session_output_serializes_to_valid_json(
        name in session_name_strategy(),
        status in session_status_strategy(),
        state in workspace_state_strategy(),
        path in absolute_path_strategy(),
    ) {
        let result = SessionOutput::new(
            name.clone(),
            status,
            state,
            path.clone(),
        );

        // Result should be Ok (valid construction)
        let session = match result {
            Ok(s) => s,
            Err(e) => {
                // This WILL FAIL in RED phase - implementation may not exist
                prop_assert!(false, "SessionOutput::new failed: {:?}", e);
                return Ok(());
            }
        };

        // Serialize to JSON
        let json_result = serde_json::to_string(&session);
        prop_assert!(json_result.is_ok(), "Serialization must succeed");

        let json = json_result.map_err(|_| "serialization failed")?;

        // Parse back to verify it's valid JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        prop_assert!(parsed.is_ok(), "Serialized JSON must be parseable: {}", json);

        // Verify the parsed JSON contains expected fields
        let value = parsed.map_err(|e| format!("parse error: {}", e))?;
        prop_assert!(value.is_object(), "Output must be a JSON object");
        prop_assert!(value.get("name").is_some(), "JSON must contain 'name' field");
        prop_assert!(value.get("status").is_some(), "JSON must contain 'status' field");

        Ok(())
    }

    /// Property: Summary serializes to valid JSON
    ///
    /// INVARIANT: JSON valid - all status output must be valid JSON
    /// RED PHASE: This test MUST FAIL if Summary cannot be serialized
    #[test]
    fn prop_summary_serializes_to_valid_json(
        message in any_string_strategy(),
    ) {
        // Empty message should fail (RED phase behavior)
        let result = Summary::new(SummaryType::Status, message.clone());

        if message.trim().is_empty() {
            // Empty message is invalid - should fail
            prop_assert!(result.is_err(), "Empty message should fail");
            return Ok(());
        }

        let summary = match result {
            Ok(s) => s,
            Err(e) => {
                prop_assert!(false, "Summary::new failed for valid message: {:?}", e);
                return Ok(());
            }
        };

        // Serialize to JSON
        let json_result = serde_json::to_string(&summary);
        prop_assert!(json_result.is_ok(), "Summary serialization must succeed");

        let json = json_result.map_err(|_| "serialization failed")?;

        // Parse back
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        prop_assert!(parsed.is_ok(), "Serialized Summary must be parseable");

        Ok(())
    }

    /// Property: OutputLine serializes to valid JSON
    ///
    /// INVARIANT: JSON valid - all OutputLine variants must serialize to valid JSON
    /// RED PHASE: This test MUST FAIL if OutputLine cannot serialize properly
    #[test]
    fn prop_output_line_session_serializes(
        name in session_name_strategy(),
        status in session_status_strategy(),
        state in workspace_state_strategy(),
        path in absolute_path_strategy(),
    ) {
        let session = SessionOutput::new(name, status, state, path)
            .map_err(|e| format!("SessionOutput creation failed: {:?}", e))?;

        let line = OutputLine::Session(session);

        // Serialize to JSON
        let json_result = serde_json::to_string(&line);
        prop_assert!(json_result.is_ok(), "OutputLine::Session must serialize");

        let json = json_result.map_err(|_| "serialization failed")?;

        // Parse and verify structure
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        prop_assert!(parsed.is_ok(), "OutputLine JSON must be parseable");

        let value = parsed.map_err(|e| format!("parse error: {}", e))?;
        prop_assert!(value.is_object(), "OutputLine must be a JSON object");

        // RED PHASE: This assertion will FAIL because the serialized form
        // may not match expected structure yet
        prop_assert!(
            value.get("Session").is_some() || value.get("session").is_some(),
            "OutputLine must contain session data: {}",
            json
        );

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 2: FIELD COMPLETENESS
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: All required fields are present in SessionOutput JSON
    ///
    /// INVARIANT: Fields complete - name, status, state, workspace_path required
    /// RED PHASE: This test MUST FAIL if any required field is missing
    #[test]
    fn prop_session_output_has_required_fields(
        name in session_name_strategy(),
        status in session_status_strategy(),
        state in workspace_state_strategy(),
        path in absolute_path_strategy(),
    ) {
        let session = SessionOutput::new(name, status, state, path)
            .map_err(|e| format!("SessionOutput creation failed: {:?}", e))?;

        let json = serde_json::to_string(&session)
            .map_err(|e| format!("Serialization failed: {}", e))?;

        let value: serde_json::Value = serde_json::from_str(&json)
            .map_err(|e| format!("Parse failed: {}", e))?;

        // RED PHASE: These assertions MUST fail until implementation is complete
        // Check all required fields are present

        let name_field = value.get("name");
        prop_assert!(
            name_field.is_some(),
            "JSON must contain 'name' field: {}",
            json
        );
        prop_assert!(
            name_field.and_then(|v| v.as_str()).is_some(),
            "'name' field must be a string"
        );

        let status_field = value.get("status");
        prop_assert!(
            status_field.is_some(),
            "JSON must contain 'status' field: {}",
            json
        );

        let state_field = value.get("state");
        prop_assert!(
            state_field.is_some(),
            "JSON must contain 'state' field: {}",
            json
        );

        let path_field = value.get("workspace_path");
        prop_assert!(
            path_field.is_some(),
            "JSON must contain 'workspace_path' field: {}",
            json
        );

        // RED PHASE: Assert timestamp fields exist
        // These MUST FAIL until timestamps are implemented
        prop_assert!(
            value.get("created_at").is_some(),
            "RED PHASE FAIL: JSON must contain 'created_at' timestamp"
        );

        prop_assert!(
            value.get("updated_at").is_some(),
            "RED PHASE FAIL: JSON must contain 'updated_at' timestamp"
        );

        Ok(())
    }

    /// Property: Branch field is optional but present in JSON when set
    ///
    /// INVARIANT: Fields complete - optional fields handled correctly
    /// RED PHASE: This test MUST FAIL if branch handling is not implemented
    #[test]
    fn prop_session_output_branch_optional(
        name in session_name_strategy(),
        status in session_status_strategy(),
        state in workspace_state_strategy(),
        path in absolute_path_strategy(),
        branch in branch_name_strategy(),
    ) {
        let mut session = SessionOutput::new(name.clone(), status, state, path)
            .map_err(|e| format!("SessionOutput creation failed: {:?}", e))?;

        // Add branch if provided
        if let Some(b) = branch.clone() {
            session = session.with_branch(b);
        }

        let json = serde_json::to_string(&session)
            .map_err(|e| format!("Serialization failed: {}", e))?;

        let value: serde_json::Value = serde_json::from_str(&json)
            .map_err(|e| format!("Parse failed: {}", e))?;

        // RED PHASE: This MUST FAIL until branch serialization is correct
        if let Some(ref b) = branch {
            let branch_field = value.get("branch");
            prop_assert!(
                branch_field.is_some(),
                "RED PHASE FAIL: 'branch' field must be present when set: {}",
                json
            );
            prop_assert!(
                branch_field.and_then(|v| v.as_str()) == Some(b.as_str()),
                "'branch' field must match the set value"
            );
        }

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 3: STATE CONSISTENCY
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: SessionStatus state transitions are valid
    ///
    /// INVARIANT: State consistency - only valid transitions allowed
    /// RED PHASE: This test MUST FAIL if transitions are not properly validated
    #[test]
    fn prop_session_status_valid_transitions(
        from_status in session_status_strategy(),
        to_status in session_status_strategy(),
    ) {
        let can_transition = from_status.can_transition_to(to_status);

        // Define the valid transition matrix
        let valid_transitions = match (from_status, to_status) {
            // Creating can go to Active or Failed
            (SessionStatus::Creating, SessionStatus::Active) => true,
            (SessionStatus::Creating, SessionStatus::Failed) => true,

            // Active can go to Paused or Completed
            (SessionStatus::Active, SessionStatus::Paused) => true,
            (SessionStatus::Active, SessionStatus::Completed) => true,

            // Paused can go to Active or Completed
            (SessionStatus::Paused, SessionStatus::Active) => true,
            (SessionStatus::Paused, SessionStatus::Completed) => true,

            // Terminal states (Completed, Failed) cannot transition
            (SessionStatus::Completed, _) => false,
            (SessionStatus::Failed, _) => false,

            // All other transitions are invalid
            _ => false,
        };

        prop_assert!(
            can_transition == valid_transitions,
            "Transition {:?} -> {:?}: expected {}, got {}",
            from_status,
            to_status,
            valid_transitions,
            can_transition
        );

        Ok(())
    }

    /// Property: Terminal states have no valid next states
    ///
    /// INVARIANT: State consistency - terminal states are final
    /// RED PHASE: This test MUST FAIL if terminal states are not properly identified
    #[test]
    fn prop_terminal_states_have_no_transitions(
        status in session_status_strategy(),
    ) {
        let is_terminal = status.is_terminal();

        match status {
            SessionStatus::Completed | SessionStatus::Failed => {
                prop_assert!(is_terminal, "{:?} should be terminal", status);

                // RED PHASE: valid_next_states MUST return empty for terminal states
                let next_states = status.valid_next_states();
                prop_assert!(
                    next_states.is_empty(),
                    "RED PHASE FAIL: Terminal state {:?} should have no next states, got {:?}",
                    status,
                    next_states
                );
            }
            SessionStatus::Creating | SessionStatus::Active | SessionStatus::Paused => {
                prop_assert!(!is_terminal, "{:?} should not be terminal", status);

                // Non-terminal states should have at least one valid next state
                let next_states = status.valid_next_states();
                prop_assert!(
                    !next_states.is_empty(),
                    "{:?} should have at least one valid next state",
                    status
                );
            }
        }

        Ok(())
    }

    /// Property: SessionStatus JSON serialization is lowercase
    ///
    /// INVARIANT: State consistency - status values are consistently formatted
    /// RED PHASE: This test MUST FAIL if serialization format is not enforced
    #[test]
    fn prop_session_status_serialization_lowercase(
        status in session_status_strategy(),
    ) {
        let json = serde_json::to_string(&status)
            .map_err(|e| format!("Serialization failed: {}", e))?;

        // Remove quotes for comparison
        let json_lower = json.to_lowercase();
        let expected = match status {
            SessionStatus::Creating => "\"creating\"",
            SessionStatus::Active => "\"active\"",
            SessionStatus::Paused => "\"paused\"",
            SessionStatus::Completed => "\"completed\"",
            SessionStatus::Failed => "\"failed\"",
        };

        // RED PHASE: This MUST FAIL if serialization is not lowercase
        prop_assert!(
            json == expected,
            "RED PHASE FAIL: Status {:?} should serialize as {}, got {}",
            status,
            expected,
            json
        );

        Ok(())
    }

    /// Property: Session name validation invariants
    ///
    /// INVARIANT: State consistency - names follow validation rules
    /// RED PHASE: This test MUST FAIL for invalid name acceptance
    #[test]
    fn prop_session_name_validation(
        name in "[a-zA-Z0-9_-]{0,64}",
    ) {
        use std::str::FromStr;
        use isolate_core::types::SessionName;

        let result = SessionName::from_str(&name);

        let starts_with_letter = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic());
        let valid_chars = name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        let valid_length = !name.is_empty() && name.len() <= 64;
        let should_be_valid = starts_with_letter && valid_chars && valid_length;

        if should_be_valid {
            prop_assert!(
                result.is_ok(),
                "Name '{}' should be valid (starts_with_letter={}, valid_chars={}, valid_length={})",
                name,
                starts_with_letter,
                valid_chars,
                valid_length
            );
        } else {
            // RED PHASE: Invalid names MUST be rejected
            prop_assert!(
                result.is_err(),
                "RED PHASE FAIL: Name '{}' should be rejected",
                name
            );
        }

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 4: WORKSPACE PATH INVARIANTS
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: Workspace path must be absolute
    ///
    /// INVARIANT: State consistency - workspace paths are absolute
    /// RED PHASE: This test MUST FAIL if path validation is not enforced
    #[test]
    fn prop_workspace_path_must_be_absolute(
        path_str in any_string_strategy(),
    ) {
        let path = PathBuf::from(&path_str);
        let is_absolute = path.is_absolute();

        // Try to create a session with this path
        let result = SessionOutput::new(
            "test-session".to_string(),
            SessionStatus::Active,
            WorkspaceState::Created,
            path.clone(),
        );

        if is_absolute {
            prop_assert!(
                result.is_ok(),
                "Absolute path '{}' should be accepted",
                path_str
            );
        } else {
            // RED PHASE: Relative paths MUST be rejected
            // This will FAIL until path validation is implemented in SessionOutput
            prop_assert!(
                result.is_err(),
                "RED PHASE FAIL: Relative path '{}' should be rejected",
                path_str
            );
        }

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 5: TIMESTAMP INVARIANTS
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: Timestamps are present and valid ISO 8601
    ///
    /// INVARIANT: Field completeness - timestamps are always present
    /// RED PHASE: This test MUST FAIL until timestamps are implemented
    #[test]
    fn prop_timestamps_present_and_valid(
        name in session_name_strategy(),
        status in session_status_strategy(),
        state in workspace_state_strategy(),
        path in absolute_path_strategy(),
    ) {
        let session = SessionOutput::new(name, status, state, path)
            .map_err(|e| format!("SessionOutput creation failed: {:?}", e))?;

        let json = serde_json::to_string(&session)
            .map_err(|e| format!("Serialization failed: {}", e))?;

        let value: serde_json::Value = serde_json::from_str(&json)
            .map_err(|e| format!("Parse failed: {}", e))?;

        // RED PHASE: These assertions MUST FAIL until timestamps are in JSON output
        let created_at = value.get("created_at");
        let updated_at = value.get("updated_at");

        prop_assert!(
            created_at.is_some(),
            "RED PHASE FAIL: 'created_at' timestamp must be present in JSON"
        );

        prop_assert!(
            updated_at.is_some(),
            "RED PHASE FAIL: 'updated_at' timestamp must be present in JSON"
        );

        // If timestamps are present, verify they are valid
        if let Some(ts) = created_at.and_then(|v| v.as_str()) {
            // Try to parse as ISO 8601 / RFC 3339
            let parsed = chrono::DateTime::parse_from_rfc3339(ts);
            prop_assert!(
                parsed.is_ok(),
                "'created_at' must be valid ISO 8601, got: {}",
                ts
            );
        }

        if let Some(ts) = updated_at.and_then(|v| v.as_str()) {
            let parsed = chrono::DateTime::parse_from_rfc3339(ts);
            prop_assert!(
                parsed.is_ok(),
                "'updated_at' must be valid ISO 8601, got: {}",
                ts
            );
        }

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS TO CONFIRM TEST HARNESS WORKS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// This test MUST PASS to confirm the test harness works
    #[test]
    fn test_harness_works() {}

    /// This test confirms SessionOutput can be created with valid inputs
    #[test]
    fn test_session_output_creation_with_valid_inputs() {
        let result = SessionOutput::new(
            "test-session".to_string(),
            SessionStatus::Active,
            WorkspaceState::Created,
            PathBuf::from("/tmp/test"),
        );

        assert!(result.is_ok(), "Valid session should be created");
    }

    /// This test MUST FAIL in RED phase - empty names should be rejected
    #[test]
    fn test_empty_name_rejected() {
        let result = SessionOutput::new(
            "".to_string(),
            SessionStatus::Active,
            WorkspaceState::Created,
            PathBuf::from("/tmp/test"),
        );

        // RED PHASE: This will fail until validation is implemented
        assert!(result.is_err(), "RED PHASE: Empty name should be rejected");
    }
}
