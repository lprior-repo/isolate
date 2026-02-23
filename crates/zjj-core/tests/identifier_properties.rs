//! Property-based tests for domain identifier types using proptest.
//!
//! Tests identifier invariants:
//! 1. Roundtrip properties (parse -> display -> parse)
//! 2. Valid format generation
//! 3. Invalid format rejection
//! 4. Boundary conditions (max length, empty, etc.)

// Integration tests have relaxed clippy settings for test ergonomics.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    clippy::uninlined_format_args,
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
    clippy::await_holding_lock,
    clippy::significant_drop_tightening,
    clippy::needless_continue
)]

use proptest::prelude::*;

/// Optimized proptest config for fast identifier property tests.
/// Uses 64 cases for simple invariants like roundtrip tests.
fn fast_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        max_shrink_iters: 256,
        ..ProptestConfig::default()
    }
}

/// Standard proptest config for identifier property tests.
/// Uses 100 cases for validation tests.
#[allow(dead_code)]
fn standard_config() -> ProptestConfig {
    ProptestConfig {
        cases: 100,
        ..ProptestConfig::default()
    }
}

use zjj_core::domain::{
    identifiers::{
        AbsolutePath, AgentId, BeadId, IdentifierError, SessionId, SessionName, TaskId,
        WorkspaceName,
    },
    agent::AgentState,
    queue::ClaimState,
    session::{BranchState, ParentState},
};

// =============================================================================
// STRATEGIES - Valid Identifier Generation
// =============================================================================

/// Strategy for generating valid session names
fn valid_session_name_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Simple names: letter followed by alphanumeric/hyphen/underscore
        "[a-zA-Z][a-zA-Z0-9_-]{0,62}",
        // Single letter (minimum valid)
        "[a-zA-Z]",
        // Maximum length (63 chars)
        "[a-zA-Z][a-zA-Z0-9_-]{0,61}[a-zA-Z0-9_]",
    ]
}

/// Strategy for generating valid agent IDs
fn valid_agent_id_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Simple agent IDs
        "[a-zA-Z0-9][a-zA-Z0-9_.:-]{0,127}",
        // Single character
        "[a-zA-Z0-9]",
        // Process-style IDs
        "pid-[0-9]{1,10}",
        // Agent name with colon
        "[a-zA-Z][a-zA-Z0-9_-]*:[a-zA-Z0-9_-]+",
    ]
}

/// Strategy for generating valid workspace names
fn valid_workspace_name_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Simple workspace names
        "[a-zA-Z0-9_-]{1,100}",
        // Longer names
        "[a-zA-Z][a-zA-Z0-9_-]{0,200}[a-zA-Z0-9_]",
        // Single character
        "[a-zA-Z0-9]",
    ]
}

/// Strategy for generating valid task IDs (bead IDs)
fn valid_task_id_strategy() -> impl Strategy<Value = String> {
    // Generate hex string of varying lengths
    "[0-9a-fA-F]{1,50}".prop_map(|hex| format!("bd-{hex}"))
}

/// Strategy for generating valid session IDs
fn valid_session_id_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-zA-Z0-9_-]{1,50}",
        "[a-zA-Z][a-zA-Z0-9_-]{0,100}",
    ]
}

/// Strategy for generating valid absolute paths (Unix)
fn valid_absolute_path_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Root
        "/",
        // Simple paths
        "/[a-zA-Z0-9_-]{1,20}",
        // Nested paths
        "/[a-zA-Z0-9_-]{1,10}/[a-zA-Z0-9_-]{1,10}",
        // Multi-level paths
        "/[a-zA-Z0-9_-]{1,5}/[a-zA-Z0-9_-]{1,5}/[a-zA-Z0-9_-]{1,5}",
        // Paths with numbers
        "/[a-zA-Z]{1,10}/[0-9]{1,5}",
    ]
}

/// Strategy for generating invalid session names
fn invalid_session_name_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Empty string
        Just("".to_string()),
        // Starts with number - at least 2 chars to avoid confusion with single digits
        "[0-9][a-zA-Z0-9_-]{1,10}",
        // Starts with special character - at least 2 chars
        "[._-][a-zA-Z0-9_-]{1,10}",
        // Too long (>63 chars)
        "[a-zA-Z0-9_-]{64,100}",
        // Contains spaces - requires at least one space
        "[a-zA-Z0-9_-]+ [a-zA-Z0-9_-]+",
    ]
}

/// Strategy for generating invalid agent IDs
fn invalid_agent_id_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Empty
        Just("".to_string()),
        // Too long (>128 chars)
        "[a-zA-Z0-9_.:-]{129,200}",
        // Contains spaces - requires at least one space
        "[a-zA-Z0-9_.:-]+ [a-zA-Z0-9_.:-]+",
    ]
}

/// Strategy for generating invalid workspace names
fn invalid_workspace_name_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Empty
        "",
        // Contains path separator
        "[a-zA-Z0-9_]{1,10}/[a-zA-Z0-9_]{1,10}",
        // Too long (>255 chars)
        "[a-zA-Z0-9_-]{256,300}",
    ]
}

/// Strategy for generating invalid task IDs
fn invalid_task_id_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Empty
        "",
        // Missing prefix
        "[0-9a-fA-F]{1,50}",
        // Wrong prefix
        "[a-z]{2}-[0-9a-fA-F]{1,50}",
    ]
}

/// Strategy for generating invalid session IDs
fn invalid_session_id_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Empty
        "",
        // Non-ASCII UTF-8 strings are hard to generate, so skip for now
    ]
}

/// Strategy for generating invalid absolute paths
fn invalid_absolute_path_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Empty
        "",
        // Relative paths
        "[a-zA-Z0-9_]{1,10}",
        "[a-zA-Z0-9_]{1,10}/[a-zA-Z0-9_]{1,10}",
    ]
}

// =============================================================================
// SESSION NAME PROPERTIES
// =============================================================================

proptest! {
    #![proptest_config(fast_config())]

    /// Property 1: Roundtrip - parse -> display -> parse should preserve identity
    #[test]
    fn session_name_roundtrip(name in valid_session_name_strategy()) {
        let first = SessionName::parse(name.clone()).expect("valid name should parse");
        let displayed = first.to_string();
        let second = SessionName::parse(displayed).expect("displayed name should re-parse");
        prop_assert_eq!(first, second);
    }

    /// Property 2: All valid session names parse successfully
    #[test]
    fn session_name_valid_parse(name in valid_session_name_strategy()) {
        let result = SessionName::parse(name);
        prop_assert!(result.is_ok(), "Valid session name should parse: {:?}", result);
    }

    /// Property 3: Invalid session names are rejected
    #[test]
    fn session_name_invalid_rejected(name in invalid_session_name_strategy()) {
        let result = SessionName::parse(name);
        prop_assert!(result.is_err(), "Invalid session name should be rejected");
    }

    /// Property 4: Session name length bounds
    #[test]
    fn session_name_length_bounds(len in 1usize..200) {
        let name = "a".repeat(len);
        let result = SessionName::parse(&name);
        if len <= 63 {
            prop_assert!(result.is_ok(), "Length {} should be valid", len);
        } else {
            prop_assert!(result.is_err(), "Length {} should be rejected", len);
        }
    }
}

// =============================================================================
// AGENT ID PROPERTIES
// =============================================================================

proptest! {
    #![proptest_config(fast_config())]

    /// Property 1: Roundtrip for agent IDs
    #[test]
    fn agent_id_roundtrip(id in valid_agent_id_strategy()) {
        let first = AgentId::parse(id.clone()).expect("valid ID should parse");
        let displayed = first.to_string();
        let second = AgentId::parse(displayed).expect("displayed ID should re-parse");
        prop_assert_eq!(first, second);
    }

    /// Property 2: All valid agent IDs parse successfully
    #[test]
    fn agent_id_valid_parse(id in valid_agent_id_strategy()) {
        let result = AgentId::parse(id);
        prop_assert!(result.is_ok(), "Valid agent ID should parse: {:?}", result);
    }

    /// Property 3: Invalid agent IDs are rejected
    #[test]
    fn agent_id_invalid_rejected(id in invalid_agent_id_strategy()) {
        let result = AgentId::parse(id);
        prop_assert!(result.is_err(), "Invalid agent ID should be rejected");
    }

    /// Property 4: Agent ID length bounds
    #[test]
    fn agent_id_length_bounds(len in 1usize..200) {
        let id = "a".repeat(len);
        let result = AgentId::parse(&id);
        if len <= 128 {
            prop_assert!(result.is_ok(), "Length {} should be valid", len);
        } else {
            prop_assert!(result.is_err(), "Length {} should be rejected", len);
        }
    }
}

// =============================================================================
// WORKSPACE NAME PROPERTIES
// =============================================================================

proptest! {
    #![proptest_config(fast_config())]

    /// Property 1: Roundtrip for workspace names
    #[test]
    fn workspace_name_roundtrip(name in valid_workspace_name_strategy()) {
        let first = WorkspaceName::parse(name.clone()).expect("valid name should parse");
        let displayed = first.to_string();
        let second = WorkspaceName::parse(displayed).expect("displayed name should re-parse");
        prop_assert_eq!(first, second);
    }

    /// Property 2: All valid workspace names parse successfully
    #[test]
    fn workspace_name_valid_parse(name in valid_workspace_name_strategy()) {
        let result = WorkspaceName::parse(name);
        prop_assert!(result.is_ok(), "Valid workspace name should parse: {:?}", result);
    }

    /// Property 3: Invalid workspace names are rejected
    #[test]
    fn workspace_name_invalid_rejected(name in invalid_workspace_name_strategy()) {
        let result = WorkspaceName::parse(name);
        prop_assert!(result.is_err(), "Invalid workspace name should be rejected");
    }

    /// Property 4: Workspace name length bounds
    #[test]
    fn workspace_name_length_bounds(len in 1usize..300) {
        let name = "a".repeat(len);
        let result = WorkspaceName::parse(&name);
        if len <= 255 {
            prop_assert!(result.is_ok(), "Length {} should be valid", len);
        } else {
            prop_assert!(result.is_err(), "Length {} should be rejected", len);
        }
    }
}

// =============================================================================
// TASK ID PROPERTIES
// =============================================================================

proptest! {
    #![proptest_config(fast_config())]

    /// Property 1: Roundtrip for task IDs
    #[test]
    fn task_id_roundtrip(id in valid_task_id_strategy()) {
        let first = TaskId::parse(id.clone()).expect("valid ID should parse");
        let displayed = first.to_string();
        let second = TaskId::parse(displayed).expect("displayed ID should re-parse");
        prop_assert_eq!(first, second);
    }

    /// Property 2: All valid task IDs parse successfully
    #[test]
    fn task_id_valid_parse(id in valid_task_id_strategy()) {
        let result = TaskId::parse(&id);
        prop_assert!(result.is_ok(), "Valid task ID should parse: {:?}", result);
    }

    /// Property 3: Invalid task IDs are rejected
    #[test]
    fn task_id_invalid_rejected(id in invalid_task_id_strategy()) {
        let result = TaskId::parse(&id);
        prop_assert!(result.is_err(), "Invalid task ID should be rejected");
    }

    /// Property 4: BeadId is TaskId alias
    #[test]
    fn bead_id_matches_task_id(id in valid_task_id_strategy()) {
        let task = TaskId::parse(&id).expect("valid task ID");
        let bead = BeadId::parse(&id).expect("valid bead ID");
        prop_assert_eq!(task.as_str(), bead.as_str());
    }
}

// =============================================================================
// SESSION ID PROPERTIES
// =============================================================================

proptest! {
    #![proptest_config(fast_config())]

    /// Property 1: Roundtrip for session IDs
    #[test]
    fn session_id_roundtrip(id in valid_session_id_strategy()) {
        let first = SessionId::parse(id.clone()).expect("valid ID should parse");
        let displayed = first.to_string();
        let second = SessionId::parse(displayed).expect("displayed ID should re-parse");
        prop_assert_eq!(first, second);
    }

    /// Property 2: All valid session IDs parse successfully
    #[test]
    fn session_id_valid_parse(id in valid_session_id_strategy()) {
        let result = SessionId::parse(&id);
        prop_assert!(result.is_ok(), "Valid session ID should parse: {:?}", result);
    }

    /// Property 3: Invalid session IDs are rejected
    #[test]
    fn session_id_invalid_rejected(id in invalid_session_id_strategy()) {
        let result = SessionId::parse(&id);
        prop_assert!(result.is_err(), "Invalid session ID should be rejected");
    }
}

// =============================================================================
// ABSOLUTE PATH PROPERTIES
// =============================================================================

proptest! {
    #![proptest_config(fast_config())]

    /// Property 1: Roundtrip for absolute paths
    #[test]
    fn absolute_path_roundtrip(path in valid_absolute_path_strategy()) {
        let first = AbsolutePath::parse(path.clone()).expect("valid path should parse");
        let displayed = first.to_string();
        let second = AbsolutePath::parse(displayed).expect("displayed path should re-parse");
        prop_assert_eq!(first, second);
    }

    /// Property 2: All valid absolute paths parse successfully
    #[test]
    fn absolute_path_valid_parse(path in valid_absolute_path_strategy()) {
        let result = AbsolutePath::parse(&path);
        prop_assert!(result.is_ok(), "Valid absolute path should parse: {:?}", result);
    }

    /// Property 3: Invalid absolute paths are rejected
    #[test]
    fn absolute_path_invalid_rejected(path in invalid_absolute_path_strategy()) {
        let result = AbsolutePath::parse(&path);
        prop_assert!(result.is_err(), "Invalid absolute path should be rejected");
    }

    /// Property 4: Absolute paths must start with /
    #[test]
    fn absolute_path_requires_leading_slash(rest in "[a-zA-Z0-9_-]{1,50}") {
        let absolute = format!("/{rest}");
        prop_assert!(AbsolutePath::parse(&absolute).is_ok());

        let relative = rest.clone();
        prop_assert!(AbsolutePath::parse(&relative).is_err());
    }

    /// Property 5: to_path_buf preserves the path
    #[test]
    fn absolute_path_to_path_buf(path in valid_absolute_path_strategy()) {
        let abs_path = AbsolutePath::parse(&path).expect("valid path");
        let path_buf = abs_path.to_path_buf();
        prop_assert_eq!(path_buf.as_os_str().to_str(), Some(path.as_str()));
    }
}

// =============================================================================
// ERROR TYPE PROPERTIES (Regular tests, not proptest)
// =============================================================================

#[test]
fn test_identifier_error_displayable() {
    let errors = [
        IdentifierError::empty(),
        IdentifierError::too_long(10, 20),
        IdentifierError::invalid_characters("test"),
        IdentifierError::invalid_format("test"),
        IdentifierError::invalid_start('a'),
        IdentifierError::invalid_prefix("bd-", "test"),
        IdentifierError::invalid_hex("test"),
        IdentifierError::not_absolute_path("test"),
        IdentifierError::NullBytesInPath,
        IdentifierError::NotAscii { value: "test".to_string() },
        IdentifierError::ContainsPathSeparators,
    ];

    for error in errors {
        let displayed = format!("{error}");
        assert!(!displayed.is_empty(), "Error should display");
    }
}

#[test]
fn test_identifier_error_predicates() {
    assert!(IdentifierError::empty().is_empty());
    assert!(IdentifierError::too_long(10, 20).is_too_long());
    assert!(IdentifierError::invalid_characters("test").is_invalid_characters());
    assert!(IdentifierError::invalid_format("test").is_invalid_format());
}

// =============================================================================
// CROSS-TYPE PROPERTIES
// =============================================================================

proptest! {
    #![proptest_config(fast_config())]

    /// Property: Different identifier types don't accidentally parse each other's formats
    #[test]
    fn identifier_types_are_distinct(name in valid_session_name_strategy()) {
        let session_name = SessionName::parse(&name);
        prop_assert!(session_name.is_ok());

        let task_id = TaskId::parse(&name);
        // Most session names won't be valid task IDs (no bd- prefix)
        prop_assert!(task_id.is_err() || name.starts_with("bd-"));
    }
}

/// Property: Empty string is rejected by all identifier types
#[test]
fn empty_string_rejected_by_all_types() {
    assert!(SessionName::parse("").is_err());
    assert!(AgentId::parse("").is_err());
    assert!(WorkspaceName::parse("").is_err());
    assert!(TaskId::parse("").is_err());
    assert!(SessionId::parse("").is_err());
    assert!(AbsolutePath::parse("").is_err());
}

// =============================================================================
// SERDE PROPERTIES
// =============================================================================

proptest! {
    #![proptest_config(fast_config())]

    /// Property: Identifiers can be serialized and deserialized via serde
    #[test]
    fn session_name_serde_roundtrip(name in valid_session_name_strategy()) {
        let original = SessionName::parse(name).expect("valid name");
        let serialized = serde_json::to_string(&original).expect("serialize");
        let deserialized: SessionName = serde_json::from_str(&serialized).expect("deserialize");
        prop_assert_eq!(original, deserialized);
    }

    #[test]
    fn agent_id_serde_roundtrip(id in valid_agent_id_strategy()) {
        let original = AgentId::parse(id).expect("valid ID");
        let serialized = serde_json::to_string(&original).expect("serialize");
        let deserialized: AgentId = serde_json::from_str(&serialized).expect("deserialize");
        prop_assert_eq!(original, deserialized);
    }

    #[test]
    fn task_id_serde_roundtrip(id in valid_task_id_strategy()) {
        let original = TaskId::parse(&id).expect("valid ID");
        let serialized = serde_json::to_string(&original).expect("serialize");
        let deserialized: TaskId = serde_json::from_str(&serialized).expect("deserialize");
        prop_assert_eq!(original, deserialized);
    }
}

// =============================================================================
// TRAIT IMPLEMENTATION PROPERTIES
// =============================================================================

proptest! {
    #![proptest_config(fast_config())]

    /// Property: AsRef<str> returns the inner value
    #[test]
    fn session_name_as_str(name in valid_session_name_strategy()) {
        let session_name = SessionName::parse(name.clone()).expect("valid name");
        prop_assert_eq!(session_name.as_ref(), name);
    }

    #[test]
    fn agent_id_as_str(id in valid_agent_id_strategy()) {
        let agent_id = AgentId::parse(id.clone()).expect("valid ID");
        prop_assert_eq!(agent_id.as_ref(), id);
    }

    /// Property: From<SessionName> for String works
    #[test]
    fn session_name_into_string(name in valid_session_name_strategy()) {
        let session_name = SessionName::parse(name.clone()).expect("valid name");
        let converted: String = session_name.into();
        prop_assert_eq!(converted, name);
    }
}

// =============================================================================
// STATE ENUM PROPERTIES (Regular tests)
// =============================================================================

#[test]
fn test_agent_state_transition_deterministic() {
    let states = [
        AgentState::Idle,
        AgentState::Active,
        AgentState::Offline,
        AgentState::Error,
    ];

    for from in &states {
        for to in &states {
            let can_transition = from.can_transition_to(to);
            let can_transition_again = from.can_transition_to(to);
            assert_eq!(can_transition, can_transition_again,
                "Transition from {:?} to {:?} should be deterministic", from, to);
        }
    }
}

#[test]
fn test_agent_state_no_self_loops() {
    let states = [
        AgentState::Idle,
        AgentState::Active,
        AgentState::Offline,
        AgentState::Error,
    ];

    for state in &states {
        assert!(!state.can_transition_to(state),
            "State {:?} should not allow self-loop transition", state);
    }
}

#[test]
fn test_claim_state_transition_deterministic() {
    let agent = AgentId::parse("test-agent").expect("valid agent ID");
    let now = chrono::Utc::now();
    let expires = now + chrono::Duration::seconds(300);

    let states = [
        ClaimState::Unclaimed,
        ClaimState::Claimed {
            agent: agent.clone(),
            claimed_at: now,
            expires_at: expires,
        },
        ClaimState::Expired {
            previous_agent: agent.clone(),
            expired_at: now,
        },
    ];

    for from in &states {
        for to in &states {
            let can_transition = from.can_transition_to(to);
            let can_transition_again = from.can_transition_to(to);
            assert_eq!(can_transition, can_transition_again,
                "ClaimState transition should be deterministic");
        }
    }
}

#[test]
fn test_branch_state_transition_deterministic() {
    let states = [
        BranchState::Detached,
        BranchState::OnBranch { name: "main".to_string() },
    ];

    for from in &states {
        for to in &states {
            let can_transition = from.can_transition_to(to);
            let can_transition_again = from.can_transition_to(to);
            assert_eq!(can_transition, can_transition_again,
                "BranchState transition should be deterministic");
        }
    }
}

#[test]
fn test_parent_state_immutability() {
    let parent = SessionName::parse("parent").expect("valid name");
    let child = ParentState::ChildOf { parent: parent.clone() };

    // Root cannot become child
    assert!(!ParentState::Root.can_transition_to(&child));

    // Child cannot become root
    assert!(!child.can_transition_to(&ParentState::Root));

    // Root cannot stay root (no self-loop)
    assert!(!ParentState::Root.can_transition_to(&ParentState::Root));
}
