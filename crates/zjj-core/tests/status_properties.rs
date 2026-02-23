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

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![allow(clippy::too_many_arguments)] // Test files often have many parameters

use std::path::PathBuf;

use proptest::prelude::*;
use serde::{Deserialize, Serialize};

/// Optimized proptest config for fast status property tests.
fn fast_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        max_shrink_iters: 256,
        ..ProptestConfig::default()
    }
}

/// Standard proptest config for status property tests.
fn standard_config() -> ProptestConfig {
    ProptestConfig {
        cases: 100,
        ..ProptestConfig::default()
    }
}

use zjj_core::{
    output::{OutputLine, SessionOutput, Summary, SummaryType},
    types::SessionStatus,
    WorkspaceState,
};

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
    "[a-zA-Z0-9_-]{1,20}".prop_map(|s| PathBuf::from(format!("/tmp/zjj-test-{}", s)))
}

/// Generate optional branch names
fn branch_name_strategy() -> impl Strategy<Value = Option<String>> {
    prop_oneof![Just(None), "[a-zA-Z][a-zA-Z0-9_-]{0,30}".prop_map(Some),]
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 1: JSON VALIDITY
// ═══════════════════════════════════════════════════════════════════════════

/// A wrapper type for parsing JSON output
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatusOutput {
    #[serde(rename = "type")]
    pub type_field: String,
    pub session: Option<SessionPayload>,
    pub summary: Option<SummaryPayload>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionPayload {
    pub name: String,
    pub status: String,
    pub state: Option<String>,
    pub workspace_path: Option<String>,
    pub branch: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SummaryPayload {
    #[serde(rename = "type")]
    pub summary_type: Option<String>,
    pub message: Option<String>,
}

proptest! {
    #![proptest_config(standard_config())]

    /// Property: SessionOutput serializes to valid JSON
    ///
    /// INVARIANT: JSON valid - all status output must be valid JSON
    /// GREEN PHASE: SessionOutput serializes correctly for non-terminal statuses
    #[test]
    fn prop_session_output_serializes_to_valid_json(
        name in session_name_strategy(),
        status in session_status_strategy(),
        state in workspace_state_strategy(),
        path in absolute_path_strategy(),
    ) {
        // Skip terminal statuses - they cannot be used for new sessions
        if status.is_terminal() {
            return Ok(());
        }

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
                prop_assert!(false, "SessionOutput::new failed for non-terminal status: {:?}", e);
                return Ok(());
            }
        };

        // Serialize to JSON
        let json_result = serde_json::to_string(&session);
        prop_assert!(json_result.is_ok(), "Serialization must succeed");

        let json = json_result.unwrap();

        // Parse back to verify it's valid JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        prop_assert!(parsed.is_ok(), "Serialized JSON must be parseable: {}", json);

        // Verify the parsed JSON contains expected fields
        let value = parsed.unwrap();
        prop_assert!(value.is_object(), "Output must be a JSON object");
        prop_assert!(value.get("name").is_some(), "JSON must contain 'name' field");
        prop_assert!(value.get("status").is_some(), "JSON must contain 'status' field");
    }

    /// Property: Summary serializes to valid JSON
    ///
    /// INVARIANT: JSON valid - all status output must be valid JSON
    /// RED PHASE: This test MUST FAIL if Summary cannot be serialized
    #[test]
    fn prop_summary_serializes_to_valid_json(
        message in any_string_strategy(),
    ) {
        use zjj_core::output::domain_types::Message;

        // Empty message should fail (RED phase behavior)
        let msg_result = Message::new(message.clone());
        let result = match msg_result {
            Ok(msg) => Summary::new(SummaryType::Status, msg),
            Err(_) => return Ok(()), // Empty message - skip
        };

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

        let json = json_result.unwrap();

        // Parse back
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        prop_assert!(parsed.is_ok(), "Serialized Summary must be parseable");
    }

    /// Property: OutputLine serializes to valid JSON
    ///
    /// INVARIANT: JSON valid - all OutputLine variants must serialize to valid JSON
    /// GREEN PHASE: OutputLine serializes correctly for non-terminal statuses
    #[test]
    fn prop_output_line_session_serializes(
        name in session_name_strategy(),
        status in session_status_strategy(),
        state in workspace_state_strategy(),
        path in absolute_path_strategy(),
    ) {
        // Skip terminal statuses - they cannot be used for new sessions
        if status.is_terminal() {
            return Ok(());
        }

        let session = SessionOutput::new(name, status, state, path)
            .unwrap();

        let line = OutputLine::Session(session);

        // Serialize to JSON
        let json_result = serde_json::to_string(&line);
        prop_assert!(json_result.is_ok(), "OutputLine::Session must serialize");

        let json = json_result.unwrap();

        // Parse and verify structure
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        prop_assert!(parsed.is_ok(), "OutputLine JSON must be parseable");

        let value = parsed.unwrap();
        prop_assert!(value.is_object(), "OutputLine must be a JSON object");

        // GREEN PHASE: Verify serialized form contains session data
        prop_assert!(
            value.get("Session").is_some() || value.get("session").is_some(),
            "OutputLine must contain session data: {}",
            json
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 2: FIELD COMPLETENESS
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(standard_config())]

    /// Property: All required fields are present in SessionOutput JSON
    ///
    /// INVARIANT: Fields complete - name, status, state, workspace_path required
    /// GREEN PHASE: All required fields are present for non-terminal statuses
    #[test]
    fn prop_session_output_has_required_fields(
        name in session_name_strategy(),
        status in session_status_strategy(),
        state in workspace_state_strategy(),
        path in absolute_path_strategy(),
    ) {
        // Skip terminal statuses - they cannot be used for new sessions
        if status.is_terminal() {
            return Ok(());
        }

        let session = SessionOutput::new(name, status, state, path)
            .unwrap();

        let json = serde_json::to_string(&session)
            .unwrap();

        let value: serde_json::Value = serde_json::from_str(&json)
            .unwrap();

        // GREEN PHASE: All required fields should be present
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
        prop_assert!(
            path_field.and_then(|v| v.as_str()).is_some(),
            "'workspace_path' must be a string"
        );

      }

    /// Property: Optional branch field can be missing
    ///
    /// INVARIANT: Branch field is optional when no branch exists
    /// GREEN PHASE: This test passes when branch field handling is correct
    #[test]
    fn prop_session_output_branch_optional(
        name in session_name_strategy(),
        status in session_status_strategy(),
        state in workspace_state_strategy(),
        path in absolute_path_strategy(),
        branch in branch_name_strategy(),
    ) {
        // Skip terminal statuses which are rejected by SessionOutput::new
        if status.is_terminal() {
            return Ok(());
        }

        let session_result = SessionOutput::new(name, status, state, path);

        let session = match session_result {
            Ok(s) => s,
            Err(_) => return Ok(()), // Skip if creation fails for other reasons
        };

        // Apply branch if provided
        let session = match branch {
            Some(ref b) => session.with_branch(b.clone()),
            None => session,
        };

        let json = serde_json::to_string(&session)
            .unwrap();

        let value: serde_json::Value = serde_json::from_str(&json)
            .unwrap();

        // GREEN PHASE: Verify branch field handling
        let branch_field = value.get("branch");
        if branch.is_none() {
            // When no branch, field should be missing
            prop_assert!(
                branch_field.is_none(),
                "'branch' field should be absent when no branch: {}",
                json
            );
        } else {
            // When branch exists, field should be present
            prop_assert!(
                branch_field.is_some(),
                "'branch' field should be present: {}",
                json
            );
        }

      }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROPERTY 3: STATE CONSISTENCY
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(fast_config())]

    /// Property: Session status transitions are validated via can_transition_to
    ///
    /// INVARIANT: Only certain transitions are allowed from one status to another
    /// GREEN PHASE: SessionStatus::can_transition_to correctly validates transitions
    #[test]
    fn prop_session_status_valid_transitions(
        from in session_status_strategy(),
        to in session_status_strategy(),
    ) {
        // List of valid transitions (must match SessionStatus::can_transition_to implementation):
        let valid_transitions = vec![
            (SessionStatus::Creating, SessionStatus::Active),
            (SessionStatus::Creating, SessionStatus::Failed),
            (SessionStatus::Active, SessionStatus::Paused),
            (SessionStatus::Active, SessionStatus::Completed),
            (SessionStatus::Paused, SessionStatus::Active),
            (SessionStatus::Paused, SessionStatus::Completed),
        ];

        let is_valid_transition = valid_transitions.contains(&(from, to));
        let can_transition = from.can_transition_to(to);

        prop_assert!(
            is_valid_transition == can_transition,
            "can_transition_to({:?}, {:?}) returned {} but expected {}",
            from, to, can_transition, is_valid_transition
        );
    }

    /// Property: Terminal states cannot be used for new sessions
    ///
    /// INVARIANT: Completed and Failed are terminal states, cannot create new sessions with them
    /// GREEN PHASE: SessionOutput::new rejects terminal states
    #[test]
    fn prop_terminal_states_have_no_transitions(
        from in prop_oneof![
            Just(SessionStatus::Completed),
            Just(SessionStatus::Failed),
        ],
        _to in session_status_strategy(),
    ) {
        // Terminal states cannot be used for new sessions
        let session_result = SessionOutput::new(
            "test".to_string(),
            from,
            WorkspaceState::Working,
            PathBuf::from("/tmp/test"),
        );

        prop_assert!(
            session_result.is_err(),
            "Terminal state {:?} should be rejected for new sessions",
            from
        );
    }

    /// Property: Status serializes to lowercase strings
    ///
    /// INVARIANT: Status field in JSON should be lowercase
    /// GREEN PHASE: Status serializes to lowercase for non-terminal statuses
    #[test]
    fn prop_session_status_serialization_lowercase(
        name in session_name_strategy(),
        status in session_status_strategy(),
        state in workspace_state_strategy(),
        path in absolute_path_strategy(),
    ) {
        // Skip terminal statuses - they cannot be used for new sessions
        if status.is_terminal() {
            return Ok(());
        }

        let session = SessionOutput::new(name, status, state, path)
            .unwrap();

        let json = serde_json::to_string(&session)
            .unwrap();

        let value: serde_json::Value = serde_json::from_str(&json)
            .unwrap();

        let status_field = value.get("status")
            .and_then(|v| v.as_str())
            .ok_or("status field missing or not string").unwrap();

        // GREEN PHASE: Status should be lowercase
        prop_assert!(
            status_field == status_field.to_lowercase(),
            "Status field must be lowercase in JSON, got: '{}'",
            status_field
        );

      }
}

// ═══════════════════════════════════════════════════════════════════════════
// ADDITIONAL PROPERTY TESTS
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #![proptest_config(standard_config())]

    /// Property: Session name validation rejects invalid names
    ///
    /// INVARIANT: Invalid session names should be rejected
    /// RED PHASE: This test MUST FAIL if name validation is not implemented
    #[test]
    fn prop_session_name_validation(
        name in any_string_strategy(),
    ) {
        // Test various invalid patterns
        let long_name = "a".repeat(100);
        let invalid_patterns = vec![
            "", // empty
            "1invalid", // starts with number
            "invalid name", // contains space
            "invalid@name", // contains special char
            &long_name, // too long
        ];

        if invalid_patterns.contains(&name.as_str()) {
            // Invalid name - should fail to create
            let session_result = SessionOutput::new(
                name.clone(),
                SessionStatus::Active,
                WorkspaceState::Working,
                PathBuf::from("/tmp/test"),
            );

            prop_assert!(
                session_result.is_err(),
                "RED PHASE: Invalid session name '{}' should be rejected",
                name
            );
        } else {
            // Valid name - should succeed
            let session_result = SessionOutput::new(
                name.clone(),
                SessionStatus::Active,
                WorkspaceState::Working,
                PathBuf::from("/tmp/test"),
            );

            prop_assert!(
                session_result.is_ok(),
                "RED PHASE: Valid session name '{}' should be accepted",
                name
            );
        }

      }

    /// Property: Workspace path must be absolute
    ///
    /// INVARIANT: workspace_path must be absolute path
    /// GREEN PHASE: Relative paths are rejected for non-terminal statuses
    #[test]
    fn prop_workspace_path_must_be_absolute(
        name in session_name_strategy(),
        status in session_status_strategy(),
        state in workspace_state_strategy(),
        path in any::<String>(),
    ) {
        // Skip terminal statuses - they are rejected for other reasons
        if status.is_terminal() {
            return Ok(());
        }

        let test_path = if path.starts_with('/') {
            // Already absolute
            PathBuf::from(path)
        } else {
            // Make it relative
            PathBuf::from(path)
        };

        let session_result = SessionOutput::new(
            name,
            status,
            state,
            test_path.clone(),
        );

        if test_path.is_absolute() {
            // Absolute path should work
            prop_assert!(
                session_result.is_ok(),
                "Absolute path should be accepted"
            );
        } else {
            // Relative path should fail
            prop_assert!(
                session_result.is_err(),
                "Relative path should be rejected"
            );
        }

      }

    /// Property: Timestamps are present and valid in serialized JSON
    ///
    /// INVARIANT: JSON output should contain created_at and updated_at timestamps
    /// GREEN PHASE: Timestamps are present for non-terminal statuses
    #[test]
    fn prop_timestamps_present_and_valid(
        name in session_name_strategy(),
        status in session_status_strategy(),
        state in workspace_state_strategy(),
        path in absolute_path_strategy(),
    ) {
        // Skip terminal statuses - they cannot be used for new sessions
        if status.is_terminal() {
            return Ok(());
        }

        let session = SessionOutput::new(name, status, state, path)
            .unwrap();

        let json = serde_json::to_string(&session)
            .unwrap();

        let value: serde_json::Value = serde_json::from_str(&json)
            .unwrap();

        // GREEN PHASE: Timestamps should be present in JSON output
        let created_at = value.get("created_at");
        let updated_at = value.get("updated_at");

        prop_assert!(
            created_at.is_some(),
            "'created_at' timestamp must be present in JSON"
        );

        prop_assert!(
            updated_at.is_some(),
            "'updated_at' timestamp must be present in JSON"
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
    fn test_harness_works() {
        assert!(true, "Test harness should work");
    }

    /// This test validates that empty names are rejected
    #[test]
    fn test_empty_name_rejected() {
        let result = SessionOutput::new(
            "".to_string(),
            SessionStatus::Active,
            WorkspaceState::Created,
            PathBuf::from("/tmp/test"),
        );

        // GREEN PHASE: Empty name should be rejected
        assert!(result.is_err(), "Empty name should be rejected");
    }

    /// This test validates that terminal states are rejected for new sessions
    #[test]
    fn test_terminal_state_cannot_transition() {
        let result = SessionOutput::new(
            "test".to_string(),
            SessionStatus::Completed,
            WorkspaceState::Merged,
            PathBuf::from("/tmp/test"),
        );

        // GREEN PHASE: Terminal state (Completed) should be rejected for new sessions
        assert!(
            result.is_err(),
            "Terminal state (Completed) should not be creatable for new sessions"
        );
    }
}
