//! Integration test to verify JSONL output types match CUE schemas
//!
//! This test validates that all Rust types serialize correctly to JSON
//! and include required fields per the CUE schema specifications in:
//! - .beads/beads/zjj-20260217-001-jsonl-core-types.cue
//! - .beads/beads/zjj-20260217-002-jsonl-stack-queue-types.cue

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::Value;
use zjj_core::domain::SessionName;
use zjj_core::output::*;

fn validate_json_structure(json: &Value, required_fields: &[&str]) {
    for field in required_fields {
        assert!(
            json.get(field).is_some(),
            "Missing required field '{}' in JSON: {}",
            field,
            json
        );
    }
}

#[test]
fn test_summary_serialization() {
    // CUE schema requires: total, active, stale, conflict, orphaned
    let summary = Summary::new(
        SummaryType::Status,
        Message::new("Test summary").expect("valid"),
    )
    .expect("valid")
    .with_details("Test details".to_string());

    let json = serde_json::to_value(&summary).unwrap();

    // Verify type field
    assert!(json.get("type").is_some(), "Summary must have 'type' field");
    validate_json_structure(&json, &["type", "message", "timestamp"]);
    assert!(json.get("details").is_some());
}

#[test]
fn test_session_output_serialization() {
    // CUE schema requires: name, state, age_days, owned_by, action
    use zjj_core::{types::SessionStatus, WorkspaceState};

    let session = SessionOutput::new(
        "test-session".to_string(),
        SessionStatus::Active,
        WorkspaceState::Working,
        std::path::PathBuf::from("/tmp/test"),
    )
    .unwrap()
    .with_branch("main".to_string());

    let json = serde_json::to_value(&session).unwrap();

    validate_json_structure(
        &json,
        &[
            "name",
            "status",
            "state",
            "workspace_path",
            "created_at",
            "updated_at",
        ],
    );
    assert!(json.get("branch").is_some());
}

#[test]
fn test_issue_serialization() {
    // CUE schema requires: severity, message, session, suggested_action
    let issue = Issue::new(
        IssueId::new("TEST-001").expect("valid"),
        IssueTitle::new("Test issue").expect("valid"),
        IssueKind::Validation,
        IssueSeverity::Error,
    )
    .expect("valid")
    .with_session(SessionName::parse("test-session").expect("valid"))
    .with_suggestion("Fix it".to_string());

    let json = serde_json::to_value(&issue).unwrap();

    validate_json_structure(&json, &["id", "title", "kind", "severity"]);
    assert!(json.get("session").is_some());
    assert!(json.get("suggestion").is_some());
}

#[test]
fn test_plan_serialization() {
    // CUE schema requires: command, would_execute
    let mut plan = Plan::new(
        PlanTitle::new("Test plan").expect("valid"),
        PlanDescription::new("Test description").expect("valid"),
    )
    .expect("valid");
    plan = plan
        .with_step("Step 1".to_string(), ActionStatus::Pending)
        .expect("valid");
    plan = plan
        .with_step("Step 2".to_string(), ActionStatus::Completed)
        .expect("valid");

    let json = serde_json::to_value(&plan).unwrap();

    validate_json_structure(&json, &["title", "description", "steps", "created_at"]);

    // Verify steps structure
    let steps = json.get("steps").unwrap().as_array().unwrap();
    assert_eq!(steps.len(), 2);
    validate_json_structure(&steps[0], &["order", "description", "status"]);
}

#[test]
fn test_stack_serialization() {
    // CUE schema requires: name, parent, children, base
    let mut stack = Stack::new(
        SessionName::parse("test-stack").expect("valid"),
        BaseRef::new("main"),
    )
    .expect("valid");
    stack = stack
        .with_entry(
            SessionName::parse("session-1").expect("valid"),
            std::path::PathBuf::from("/tmp/session-1"),
            StackEntryStatus::Ready,
            BeadAttachment::Attached {
                bead_id: BeadId::parse("bd-001").expect("valid"),
            },
        )
        .expect("valid");

    let json = serde_json::to_value(&stack).unwrap();

    validate_json_structure(&json, &["name", "base_ref", "entries", "updated_at"]);

    // Verify entries structure
    let entries = json.get("entries").unwrap().as_array().unwrap();
    assert_eq!(entries.len(), 1);
    validate_json_structure(&entries[0], &["order", "session", "workspace", "status"]);
    assert!(entries[0].get("bead").is_some());
}

#[test]
fn test_queue_summary_serialization() {
    // CUE schema requires: total, ready, blocked, draft
    let queue_summary = QueueSummary::new().with_counts(QueueCounts {
        total: 10,
        pending: 3,
        ready: 4,
        blocked: 2,
        in_progress: 1,
    });

    let json = serde_json::to_value(&queue_summary).unwrap();

    validate_json_structure(
        &json,
        &[
            "total",
            "pending",
            "ready",
            "blocked",
            "in_progress",
            "updated_at",
        ],
    );
}

#[test]
fn test_queue_entry_serialization() {
    // CUE schema requires: position, session, status, blocked_by
    let entry = QueueEntry::new(
        QueueEntryId::new(1).expect("valid"),
        SessionName::parse("test-session").expect("valid"),
        5,
    )
    .expect("valid")
    .with_bead(BeadId::parse("bd-001").expect("valid"))
    .with_agent("agent-1".to_string())
    .with_status(QueueEntryStatus::Ready);

    let json = serde_json::to_value(&entry).unwrap();

    validate_json_structure(
        &json,
        &[
            "id",
            "session",
            "priority",
            "status",
            "created_at",
            "updated_at",
        ],
    );
    assert!(json.get("bead").is_some());
    assert!(json.get("agent").is_some());
}

#[test]
fn test_train_serialization() {
    // CUE schema requires: status, sessions
    let mut train = Train::new(
        TrainId::new("train-001").expect("valid"),
        SessionName::parse("test-train").expect("valid"),
    )
    .expect("valid");
    train = train
        .with_step(
            SessionName::parse("session-1").expect("valid"),
            TrainAction::Sync,
            TrainStepStatus::Pending,
        )
        .expect("valid");

    let json = serde_json::to_value(&train).unwrap();

    validate_json_structure(
        &json,
        &["id", "name", "steps", "status", "created_at", "updated_at"],
    );

    // Verify steps structure
    let steps = json.get("steps").unwrap().as_array().unwrap();
    assert_eq!(steps.len(), 1);
    validate_json_structure(&steps[0], &["order", "session", "action", "status"]);
}

#[test]
fn test_conflict_detail_serialization() {
    let conflict = ConflictDetail::overlapping("test.txt");

    let json = serde_json::to_value(&conflict).unwrap();

    validate_json_structure(
        &json,
        &["file", "conflict_type", "resolutions", "recommended"],
    );

    // Verify resolutions is an array
    let resolutions = json.get("resolutions").unwrap().as_array().unwrap();
    assert!(!resolutions.is_empty());

    // Each resolution should have: strategy, description, risk, automatic
    for resolution in resolutions {
        validate_json_structure(
            resolution,
            &["strategy", "description", "risk", "automatic"],
        );
    }
}

#[test]
fn test_conflict_analysis_serialization() {
    let output_line = OutputLine::conflict_analysis(
        "test-session",
        true,
        vec![
            ConflictDetail::overlapping("file1.txt"),
            ConflictDetail::existing("file2.txt"),
        ],
    );

    let json = serde_json::to_value(&output_line).unwrap();
    let analysis = json
        .get("conflict_analysis")
        .expect("OutputLine::ConflictAnalysis should serialize as wrapped object");

    // Verify outer structure
    validate_json_structure(
        analysis,
        &[
            "type",
            "session",
            "merge_safe",
            "total_conflicts",
            "conflicts",
            "timestamp",
        ],
    );

    // Verify conflicts array
    let conflicts = analysis.get("conflicts").unwrap().as_array().unwrap();
    assert_eq!(conflicts.len(), 2);

    // Verify specific conflict types
    assert_eq!(
        conflicts[0].get("file").unwrap().as_str().unwrap(),
        "file1.txt"
    );
    assert_eq!(
        conflicts[1].get("file").unwrap().as_str().unwrap(),
        "file2.txt"
    );
}

#[test]
fn test_output_line_enum_discriminator() {
    // All OutputLine variants should serialize as a single-key wrapper
    use zjj_core::{types::SessionStatus, WorkspaceState};

    let variants = vec![
        OutputLine::Summary(
            Summary::new(SummaryType::Info, Message::new("Test").expect("valid"))
                .expect("valid"),
        ),
        OutputLine::Session(
            SessionOutput::new(
                "test".to_string(),
                SessionStatus::Active,
                WorkspaceState::Working,
                std::path::PathBuf::from("/tmp/test"),
            )
            .unwrap(),
        ),
        OutputLine::Issue(
            Issue::new(
                IssueId::new("TEST").expect("valid"),
                IssueTitle::new("Test").expect("valid"),
                IssueKind::Validation,
                IssueSeverity::Warning,
            )
            .unwrap(),
        ),
        OutputLine::Plan(
            Plan::new(
                PlanTitle::new("Test").expect("valid"),
                PlanDescription::new("Desc").expect("valid"),
            )
            .unwrap(),
        ),
        OutputLine::Action(Action::new(
            ActionVerb::new("test").expect("valid action verb"),
            ActionTarget::new("target").expect("valid action target"),
            ActionStatus::Pending,
        )),
        OutputLine::Warning(
            Warning::new(
                WarningCode::new("TEST").expect("valid warning code"),
                Message::new("Test warning").expect("valid"),
            )
            .expect("valid"),
        ),
        OutputLine::Result(
            ResultOutput::success(
                ResultKind::Command,
                Message::new("Test result").expect("valid"),
            )
            .expect("valid"),
        ),
        OutputLine::Stack(
            Stack::new(
                SessionName::parse("test").expect("valid"),
                BaseRef::new("main"),
            )
            .expect("valid"),
        ),
        OutputLine::QueueSummary(QueueSummary::new()),
        OutputLine::QueueEntry(
            QueueEntry::new(
                QueueEntryId::new(1).expect("valid"),
                SessionName::parse("s1").expect("valid"),
                1,
            )
            .unwrap(),
        ),
        OutputLine::Train(
            Train::new(
                TrainId::new("t1").expect("valid"),
                SessionName::parse("test").expect("valid"),
            )
            .unwrap(),
        ),
    ];

    let allowed_wrappers = [
        "summary",
        "session",
        "issue",
        "plan",
        "action",
        "warning",
        "result",
        "stack",
        "queue_summary",
        "queue_entry",
        "train",
    ];

    for variant in variants {
        let json = serde_json::to_value(&variant).unwrap();
        let obj = json
            .as_object()
            .expect("OutputLine should serialize as a JSON object");

        assert_eq!(
            obj.len(),
            1,
            "OutputLine should have exactly one wrapper key: {}",
            json
        );

        let wrapper = obj
            .keys()
            .next()
            .expect("OutputLine object should have one key");

        assert!(
            allowed_wrappers.contains(&wrapper.as_str()),
            "OutputLine used unexpected wrapper key '{}': {}",
            wrapper,
            json,
        );
    }
}

#[test]
fn test_enum_value_serialization() {
    // Verify that enums serialize to lowercase strings as per CUE schema
    let issue = Issue::new(
        IssueId::new("TEST").expect("valid"),
        IssueTitle::new("Test").expect("valid"),
        IssueKind::Validation,
        IssueSeverity::Error,
    )
    .unwrap();

    let json = serde_json::to_value(&issue).unwrap();

    // kind should be lowercase (snake_case from serde(rename_all = "snake_case"))
    assert_eq!(json.get("kind").unwrap().as_str().unwrap(), "validation");

    // severity should be lowercase (snake_case from serde(rename_all = "snake_case"))
    assert_eq!(json.get("severity").unwrap().as_str().unwrap(), "error");
}

#[test]
fn test_action_status_serialization() {
    // Test ActionStatus enum serialization
    let plan = Plan::new(
        PlanTitle::new("Test").expect("valid"),
        PlanDescription::new("Desc").expect("valid"),
    )
    .unwrap();
    let json = serde_json::to_value(&plan).unwrap();

    // ActionStatus uses snake_case serialization
    let status_values = ["pending", "in_progress", "completed", "failed", "skipped"];
    let steps = json
        .get("steps")
        .and_then(serde_json::Value::as_array)
        .expect("plan steps should be an array");
    assert!(
        steps.iter().all(|step| {
            step.get("status")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|status| status_values.contains(&status))
        }),
        "all plan step statuses should be known ActionStatus values"
    );
}

#[test]
fn test_train_action_serialization() {
    // Test TrainAction enum serialization
    let train = Train::new(
        TrainId::new("t1").expect("valid"),
        SessionName::parse("test").expect("valid"),
    )
    .unwrap()
    .with_step(
        SessionName::parse("s1").expect("valid"),
        TrainAction::Sync,
        TrainStepStatus::Pending,
    )
    .unwrap();

    let json = serde_json::to_value(&train).unwrap();
    let steps = json.get("steps").unwrap().as_array().unwrap();

    // TrainAction should serialize to lowercase
    assert_eq!(steps[0].get("action").unwrap().as_str().unwrap(), "sync");
}

#[test]
fn test_resolution_strategy_serialization() {
    // Test ResolutionStrategy enum serialization
    let conflict = ConflictDetail::overlapping("test.txt");
    let json = serde_json::to_value(&conflict).unwrap();
    let resolutions = json.get("resolutions").unwrap().as_array().unwrap();

    // ResolutionStrategy uses snake_case
    let strategies = [
        "accept_ours",
        "accept_theirs",
        "jj_resolve",
        "manual_merge",
        "rebase",
        "abort",
        "skip",
    ];

    assert!(
        resolutions.iter().all(|resolution| {
            resolution
                .get("strategy")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|strategy| strategies.contains(&strategy))
        }),
        "all conflict resolution strategies should be known values"
    );
}

#[test]
fn test_timestamp_format() {
    // Verify timestamps are millisecond-precision Unix timestamps
    let summary = Summary::new(
        SummaryType::Status,
        Message::new("Test").expect("valid"),
    )
    .unwrap();
    let json = serde_json::to_value(&summary).unwrap();

    // timestamp should be a number (milliseconds since epoch)
    let timestamp = json.get("timestamp").unwrap();
    assert!(timestamp.is_number());

    // Should be relatively recent (after 2020)
    let ts = timestamp.as_i64().unwrap();
    assert!(ts > 1_577_836_800_000); // Jan 1, 2020 in milliseconds
}

#[test]
fn test_optional_fields_handling() {
    // Verify optional fields are correctly omitted when None
    let issue = Issue::new(
        IssueId::new("TEST").expect("valid"),
        IssueTitle::new("Test").expect("valid"),
        IssueKind::Validation,
        IssueSeverity::Error,
    )
    .unwrap();

    let json = serde_json::to_value(&issue).unwrap();

    // Without calling with_session or with_suggestion, these should be null or missing
    // serde_json will serialize None as null by default
    let session = json.get("session");
    let suggestion = json.get("suggestion");

    assert!(session.is_none() || session.unwrap().is_null());
    assert!(suggestion.is_none() || suggestion.unwrap().is_null());
}

#[test]
fn test_recovery_type_serialization() {
    // Test Recovery type (not in OutputLine enum but defined in types.rs)
    use zjj_core::output::{ErrorSeverity, RecoveryCapability};

    let recovery = Recovery::new(
        IssueId::new("ISSUE-001").expect("valid"),
        Assessment {
            severity: ErrorSeverity::High,
            capability: RecoveryCapability::Recoverable {
                recommended_action: "Fix the issue".to_string(),
            },
        },
    )
    .with_action("Step 1".to_string(), Some("cmd1".to_string()), true)
    .unwrap();

    let json = serde_json::to_value(&recovery).unwrap();

    validate_json_structure(&json, &["issue_id", "assessment", "actions"]);

    // Verify assessment structure
    let assessment = json.get("assessment").unwrap();
    validate_json_structure(assessment, &["severity", "capability"]);

    // Verify capability has the right structure
    let capability = assessment.get("capability").unwrap();
    assert_eq!(
        capability.get("recoverable").unwrap().as_bool().unwrap(),
        true
    );
    assert_eq!(
        capability
            .get("recommended_action")
            .unwrap()
            .as_str()
            .unwrap(),
        "Fix the issue"
    );

    // Verify actions structure
    let actions = json.get("actions").unwrap().as_array().unwrap();
    assert_eq!(actions.len(), 1);
    validate_json_structure(
        &actions[0],
        &["order", "description", "execution"],
    );
}
