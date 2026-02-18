//! JSONL output module tests

use std::path::PathBuf;

use super::*;
use crate::{types::SessionStatus, WorkspaceState};

#[test]
fn test_summary_new_validates_empty_message() {
    let result = Summary::new(SummaryType::Info, String::new());
    assert!(result.is_err());
}

#[test]
fn test_summary_new_accepts_valid_message() {
    let result = Summary::new(SummaryType::Info, "test message".to_string());
    assert!(result.is_ok());
}

#[test]
fn test_summary_with_details() {
    let summary = Summary::new(SummaryType::Info, "test".to_string())
        .expect("valid summary")
        .with_details("additional info".to_string());
    assert_eq!(summary.details, Some("additional info".to_string()));
}

#[test]
fn test_session_output_new_validates_empty_name() {
    let result = SessionOutput::new(
        String::new(),
        SessionStatus::Active,
        WorkspaceState::Created,
        PathBuf::from("/tmp"),
    );
    assert!(result.is_err());
}

#[test]
fn test_session_output_with_branch() {
    let session = SessionOutput::new(
        "feature-x".to_string(),
        SessionStatus::Active,
        WorkspaceState::Working,
        PathBuf::from("/tmp/ws"),
    )
    .expect("valid session")
    .with_branch("feature-branch".to_string());
    assert_eq!(session.branch, Some("feature-branch".to_string()));
}

#[test]
fn test_issue_new_validates_empty_title() {
    let result = Issue::new(
        "ISS-1".to_string(),
        String::new(),
        IssueKind::Validation,
        IssueSeverity::Error,
    );
    assert!(result.is_err());
}

#[test]
fn test_issue_with_session_and_suggestion() {
    let issue = Issue::new(
        "ISS-1".to_string(),
        "Test issue".to_string(),
        IssueKind::Validation,
        IssueSeverity::Warning,
    )
    .expect("valid issue")
    .with_session("session-1".to_string())
    .with_suggestion("Try this fix".to_string());
    assert_eq!(issue.session, Some("session-1".to_string()));
    assert_eq!(issue.suggestion, Some("Try this fix".to_string()));
}

#[test]
fn test_plan_new_validates_empty_title() {
    let result = Plan::new(String::new(), "description".to_string());
    assert!(result.is_err());
}

#[test]
fn test_plan_new_validates_empty_description() {
    let result = Plan::new("title".to_string(), String::new());
    assert!(result.is_err());
}

#[test]
fn test_plan_with_step() {
    let plan = Plan::new("My Plan".to_string(), "Description".to_string())
        .expect("valid plan")
        .with_step("Step 1".to_string(), ActionStatus::Pending)
        .with_step("Step 2".to_string(), ActionStatus::InProgress);
    assert_eq!(plan.steps.len(), 2);
    assert_eq!(plan.steps[0].order, 0);
    assert_eq!(plan.steps[1].order, 1);
}

#[test]
fn test_action_with_result() {
    let action = Action::new(
        "create".to_string(),
        "session-x".to_string(),
        ActionStatus::Completed,
    )
    .with_result("Created successfully".to_string());
    assert_eq!(action.result, Some("Created successfully".to_string()));
}

#[test]
fn test_warning_new_validates_empty_message() {
    let result = Warning::new("W001".to_string(), String::new());
    assert!(result.is_err());
}

#[test]
fn test_warning_with_context() {
    let warning = Warning::new("W001".to_string(), "Test warning".to_string())
        .expect("valid warning")
        .with_context("session-1".to_string(), PathBuf::from("/workspace"));
    assert!(warning.context.is_some());
    let ctx = warning.context.expect("context present");
    assert_eq!(ctx.session, "session-1");
}

#[test]
fn test_result_output_success() {
    let result = ResultOutput::success(ResultKind::Command, "Command succeeded".to_string());
    assert!(result.is_ok());
    let output = result.expect("valid result");
    assert!(output.success);
}

#[test]
fn test_result_output_failure() {
    let result = ResultOutput::failure(ResultKind::Operation, "Operation failed".to_string());
    assert!(result.is_ok());
    let output = result.expect("valid result");
    assert!(!output.success);
}

#[test]
fn test_result_output_validates_empty_message() {
    let result = ResultOutput::success(ResultKind::Command, String::new());
    assert!(result.is_err());
}

#[test]
fn test_output_line_kind() {
    let summary =
        OutputLine::Summary(Summary::new(SummaryType::Info, "test".to_string()).expect("valid"));
    assert_eq!(summary.kind(), "summary");

    let issue = OutputLine::Issue(
        Issue::new(
            "1".to_string(),
            "t".to_string(),
            IssueKind::Validation,
            IssueSeverity::Error,
        )
        .expect("valid"),
    );
    assert_eq!(issue.kind(), "issue");
}

#[test]
fn test_recovery_with_action() {
    let assessment = Assessment {
        severity: ErrorSeverity::Medium,
        recoverable: true,
        recommended_action: "Retry the operation".to_string(),
    };
    let recovery = Recovery::new("ISS-1".to_string(), assessment).with_action(
        "Run fix command".to_string(),
        Some("fix --auto".to_string()),
        true,
    );
    assert_eq!(recovery.actions.len(), 1);
    assert!(recovery.actions[0].automatic);
}

#[test]
fn test_issue_severity_serialization() {
    let severity = IssueSeverity::Warning;
    let json = serde_json::to_string(&severity).expect("serialize");
    assert_eq!(json, "\"warning\"");
}

#[test]
fn test_action_status_serialization() {
    let status = ActionStatus::InProgress;
    let json = serde_json::to_string(&status).expect("serialize");
    assert_eq!(json, "\"in_progress\"");
}

#[test]
fn test_stack_new_validates_empty_name() {
    let result = Stack::new(String::new(), "main".to_string());
    assert!(result.is_err());
}

#[test]
fn test_stack_with_entry() {
    let stack = Stack::new("feature-stack".to_string(), "main".to_string())
        .expect("valid stack")
        .with_entry(
            "session-1".to_string(),
            PathBuf::from("/ws/1"),
            StackEntryStatus::Ready,
            Some("bd-123".to_string()),
        );
    assert_eq!(stack.entries.len(), 1);
    assert_eq!(stack.entries[0].order, 0);
}

#[test]
fn test_queue_summary_default() {
    let summary = QueueSummary::default();
    assert!(summary.is_empty());
    assert!(!summary.has_blockers());
}

#[test]
fn test_queue_summary_with_counts() {
    let summary = QueueSummary::new().with_counts(QueueCounts {
        total: 10,
        pending: 3,
        ready: 4,
        blocked: 2,
        in_progress: 1,
    });
    assert_eq!(summary.total, 10);
    assert!(summary.has_blockers());
    assert!(!summary.is_empty());
}

#[test]
fn test_queue_entry_new_validates_empty_session() {
    let result = QueueEntry::new("id-1".to_string(), String::new(), 5);
    assert!(result.is_err());
}

#[test]
fn test_queue_entry_with_bead_and_agent() {
    let entry = QueueEntry::new("id-1".to_string(), "session-1".to_string(), 5)
        .expect("valid entry")
        .with_bead("bd-456".to_string())
        .with_agent("agent-1".to_string())
        .with_status(QueueEntryStatus::InProgress);
    assert_eq!(entry.bead, Some("bd-456".to_string()));
    assert_eq!(entry.agent, Some("agent-1".to_string()));
    assert_eq!(entry.status, QueueEntryStatus::InProgress);
}

#[test]
fn test_train_new_validates_empty_name() {
    let result = Train::new("train-1".to_string(), String::new());
    assert!(result.is_err());
}

#[test]
fn test_train_with_step() {
    let train = Train::new("train-1".to_string(), "merge-train".to_string())
        .expect("valid train")
        .with_step(
            "session-1".to_string(),
            TrainAction::Sync,
            TrainStepStatus::Success,
        )
        .with_step(
            "session-2".to_string(),
            TrainAction::Rebase,
            TrainStepStatus::Running,
        )
        .with_status(TrainStatus::Running);
    assert_eq!(train.steps.len(), 2);
    assert_eq!(train.status, TrainStatus::Running);
}

#[test]
fn test_output_line_new_variants() {
    let stack = OutputLine::Stack(Stack::new("s".to_string(), "main".to_string()).expect("valid"));
    assert_eq!(stack.kind(), "stack");

    let queue_summary = OutputLine::QueueSummary(QueueSummary::new());
    assert_eq!(queue_summary.kind(), "queue_summary");

    let queue_entry = OutputLine::QueueEntry(
        QueueEntry::new("id".to_string(), "s".to_string(), 1).expect("valid"),
    );
    assert_eq!(queue_entry.kind(), "queue_entry");

    let train = OutputLine::Train(Train::new("t".to_string(), "train".to_string()).expect("valid"));
    assert_eq!(train.kind(), "train");
}

#[test]
fn test_stack_entry_status_serialization() {
    let status = StackEntryStatus::Merging;
    let json = serde_json::to_string(&status).expect("serialize");
    assert_eq!(json, "\"merging\"");
}

#[test]
fn test_queue_entry_status_serialization() {
    let status = QueueEntryStatus::Claimed;
    let json = serde_json::to_string(&status).expect("serialize");
    assert_eq!(json, "\"claimed\"");
}

#[test]
fn test_train_status_serialization() {
    let status = TrainStatus::Running;
    let json = serde_json::to_string(&status).expect("serialize");
    assert_eq!(json, "\"running\"");
}

#[test]
fn test_train_action_serialization() {
    let action = TrainAction::Rebase;
    let json = serde_json::to_string(&action).expect("serialize");
    assert_eq!(json, "\"rebase\"");
}

#[test]
fn test_jsonl_writer_emit() {
    use std::io::Cursor;

    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);

    let summary = Summary::new(SummaryType::Info, "test".to_string()).expect("valid");
    writer.emit(&OutputLine::Summary(summary)).expect("emit");

    let output = String::from_utf8(cursor.into_inner()).expect("utf8");
    assert!(output.contains("\"type\":\"info\""));
    assert!(output.contains("\"message\":\"test\""));
    assert!(output.ends_with('\n'));
}

#[test]
fn test_jsonl_writer_emit_all() {
    use std::io::Cursor;

    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);

    let lines = vec![
        OutputLine::Summary(Summary::new(SummaryType::Info, "first".to_string()).expect("valid")),
        OutputLine::Summary(Summary::new(SummaryType::Info, "second".to_string()).expect("valid")),
    ];

    writer.emit_all(&lines).expect("emit all");

    let output = String::from_utf8(cursor.into_inner()).expect("utf8");
    assert_eq!(output.lines().count(), 2);
}

#[test]
fn test_jsonl_config_default() {
    let config = JsonlConfig::default();
    assert!(!config.pretty);
    assert!(config.flush_on_emit);
}

#[test]
fn test_jsonl_config_with_options() {
    let config = JsonlConfig::new()
        .with_pretty(true)
        .with_flush_on_emit(false);
    assert!(config.pretty);
    assert!(!config.flush_on_emit);
}

#[test]
fn test_jsonl_writer_with_config() {
    use std::io::Cursor;

    let mut cursor = Cursor::new(Vec::new());
    let config = JsonlConfig::new().with_flush_on_emit(false);
    let mut writer = JsonlWriter::with_config(&mut cursor, config);

    let summary = Summary::new(SummaryType::Info, "test".to_string()).expect("valid");
    writer.emit(&OutputLine::Summary(summary)).expect("emit");

    let output = String::from_utf8(cursor.into_inner()).expect("utf8");
    assert!(output.contains("\"message\":\"test\""));
}

#[test]
fn test_emit_function() {
    use std::io::Cursor;

    let mut cursor = Cursor::new(Vec::new());
    let summary = Summary::new(SummaryType::Info, "test".to_string()).expect("valid");

    emit(&mut cursor, &OutputLine::Summary(summary)).expect("emit");

    let output = String::from_utf8(cursor.into_inner()).expect("utf8");
    assert!(output.contains("\"message\":\"test\""));
}

#[test]
fn test_summary_round_trip_serialization() {
    let original = Summary::new(SummaryType::Status, "test message".to_string())
        .expect("valid")
        .with_details("extra info".to_string());

    let json = serde_json::to_string(&original).expect("serialize");
    let deserialized: Summary = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(original.type_field, deserialized.type_field);
    assert_eq!(original.message, deserialized.message);
    assert_eq!(original.details, deserialized.details);
}

#[test]
fn test_session_output_round_trip() {
    let original = SessionOutput::new(
        "feature-auth".to_string(),
        SessionStatus::Active,
        WorkspaceState::Working,
        PathBuf::from("/home/user/workspaces/feature-auth"),
    )
    .expect("valid")
    .with_branch("feature/auth-implementation".to_string());

    let json = serde_json::to_string(&original).expect("serialize");
    let deserialized: SessionOutput = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(original.name, deserialized.name);
    assert_eq!(original.status, deserialized.status);
    assert_eq!(original.state, deserialized.state);
    assert_eq!(original.branch, deserialized.branch);
}

#[test]
fn test_issue_round_trip() {
    let original = Issue::new(
        "ISS-12345".to_string(),
        "Validation failed on input".to_string(),
        IssueKind::Validation,
        IssueSeverity::Error,
    )
    .expect("valid")
    .with_session("session-abc".to_string())
    .with_suggestion("Check input format".to_string());

    let json = serde_json::to_string(&original).expect("serialize");
    let deserialized: Issue = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(original.id, deserialized.id);
    assert_eq!(original.title, deserialized.title);
    assert_eq!(original.kind, deserialized.kind);
    assert_eq!(original.severity, deserialized.severity);
}

#[test]
fn test_stack_round_trip() {
    let original = Stack::new("feature-stack".to_string(), "main".to_string())
        .expect("valid")
        .with_entry(
            "session-1".to_string(),
            PathBuf::from("/ws/1"),
            StackEntryStatus::Ready,
            Some("bd-123".to_string()),
        )
        .with_entry(
            "session-2".to_string(),
            PathBuf::from("/ws/2"),
            StackEntryStatus::Pending,
            None,
        );

    let json = serde_json::to_string(&original).expect("serialize");
    let deserialized: Stack = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(original.name, deserialized.name);
    assert_eq!(original.base_ref, deserialized.base_ref);
    assert_eq!(original.entries.len(), deserialized.entries.len());
}

#[test]
fn test_train_round_trip() {
    let original = Train::new("train-abc".to_string(), "merge-train".to_string())
        .expect("valid")
        .with_step(
            "session-1".to_string(),
            TrainAction::Sync,
            TrainStepStatus::Success,
        )
        .with_step(
            "session-2".to_string(),
            TrainAction::Rebase,
            TrainStepStatus::Running,
        )
        .with_status(TrainStatus::Running);

    let json = serde_json::to_string(&original).expect("serialize");
    let deserialized: Train = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(original.id, deserialized.id);
    assert_eq!(original.name, deserialized.name);
    assert_eq!(original.status, deserialized.status);
    assert_eq!(original.steps.len(), deserialized.steps.len());
}

#[test]
fn test_queue_entry_round_trip() {
    let original = QueueEntry::new("q-123".to_string(), "session-x".to_string(), 5)
        .expect("valid")
        .with_bead("bd-789".to_string())
        .with_agent("agent-001".to_string())
        .with_status(QueueEntryStatus::InProgress);

    let json = serde_json::to_string(&original).expect("serialize");
    let deserialized: QueueEntry = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(original.id, deserialized.id);
    assert_eq!(original.session, deserialized.session);
    assert_eq!(original.priority, deserialized.priority);
    assert_eq!(original.status, deserialized.status);
}

#[test]
fn test_output_line_round_trip_all_variants() {
    let test_cases: Vec<OutputLine> = vec![
        OutputLine::Summary(Summary::new(SummaryType::Info, "test".to_string()).expect("valid")),
        OutputLine::Session(
            SessionOutput::new(
                "s".to_string(),
                SessionStatus::Active,
                WorkspaceState::Working,
                PathBuf::from("/tmp"),
            )
            .expect("valid"),
        ),
        OutputLine::Issue(
            Issue::new(
                "1".to_string(),
                "t".to_string(),
                IssueKind::Validation,
                IssueSeverity::Error,
            )
            .expect("valid"),
        ),
        OutputLine::Plan(Plan::new("p".to_string(), "d".to_string()).expect("valid")),
        OutputLine::Action(Action::new(
            "create".to_string(),
            "target".to_string(),
            ActionStatus::Completed,
        )),
        OutputLine::Warning(Warning::new("W001".to_string(), "msg".to_string()).expect("valid")),
        OutputLine::Result(
            ResultOutput::success(ResultKind::Command, "ok".to_string()).expect("valid"),
        ),
        OutputLine::Stack(Stack::new("s".to_string(), "main".to_string()).expect("valid")),
        OutputLine::QueueSummary(QueueSummary::new()),
        OutputLine::QueueEntry(
            QueueEntry::new("q".to_string(), "s".to_string(), 1).expect("valid"),
        ),
        OutputLine::Train(Train::new("t".to_string(), "train".to_string()).expect("valid")),
    ];

    for original in test_cases {
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: OutputLine = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original.kind(), deserialized.kind());
    }
}

#[test]
fn test_summary_type_all_variants() {
    assert_eq!(
        serde_json::to_string(&SummaryType::Status).expect("s"),
        "\"status\""
    );
    assert_eq!(
        serde_json::to_string(&SummaryType::Count).expect("s"),
        "\"count\""
    );
    assert_eq!(
        serde_json::to_string(&SummaryType::Info).expect("s"),
        "\"info\""
    );
}

#[test]
fn test_issue_kind_all_variants() {
    assert_eq!(
        serde_json::to_string(&IssueKind::Validation).expect("s"),
        "\"validation\""
    );
    assert_eq!(
        serde_json::to_string(&IssueKind::StateConflict).expect("s"),
        "\"state_conflict\""
    );
    assert_eq!(
        serde_json::to_string(&IssueKind::ResourceNotFound).expect("s"),
        "\"resource_not_found\""
    );
    assert_eq!(
        serde_json::to_string(&IssueKind::PermissionDenied).expect("s"),
        "\"permission_denied\""
    );
    assert_eq!(
        serde_json::to_string(&IssueKind::Timeout).expect("s"),
        "\"timeout\""
    );
    assert_eq!(
        serde_json::to_string(&IssueKind::Configuration).expect("s"),
        "\"configuration\""
    );
    assert_eq!(
        serde_json::to_string(&IssueKind::External).expect("s"),
        "\"external\""
    );
}

#[test]
fn test_issue_severity_all_variants() {
    assert_eq!(
        serde_json::to_string(&IssueSeverity::Hint).expect("s"),
        "\"hint\""
    );
    assert_eq!(
        serde_json::to_string(&IssueSeverity::Warning).expect("s"),
        "\"warning\""
    );
    assert_eq!(
        serde_json::to_string(&IssueSeverity::Error).expect("s"),
        "\"error\""
    );
    assert_eq!(
        serde_json::to_string(&IssueSeverity::Critical).expect("s"),
        "\"critical\""
    );
}

#[test]
fn test_action_status_all_variants() {
    assert_eq!(
        serde_json::to_string(&ActionStatus::Pending).expect("s"),
        "\"pending\""
    );
    assert_eq!(
        serde_json::to_string(&ActionStatus::InProgress).expect("s"),
        "\"in_progress\""
    );
    assert_eq!(
        serde_json::to_string(&ActionStatus::Completed).expect("s"),
        "\"completed\""
    );
    assert_eq!(
        serde_json::to_string(&ActionStatus::Failed).expect("s"),
        "\"failed\""
    );
    assert_eq!(
        serde_json::to_string(&ActionStatus::Skipped).expect("s"),
        "\"skipped\""
    );
}

#[test]
fn test_result_kind_all_variants() {
    assert_eq!(
        serde_json::to_string(&ResultKind::Command).expect("s"),
        "\"command\""
    );
    assert_eq!(
        serde_json::to_string(&ResultKind::Operation).expect("s"),
        "\"operation\""
    );
    assert_eq!(
        serde_json::to_string(&ResultKind::Assessment).expect("s"),
        "\"assessment\""
    );
    assert_eq!(
        serde_json::to_string(&ResultKind::Recovery).expect("s"),
        "\"recovery\""
    );
}

#[test]
fn test_error_severity_all_variants() {
    assert_eq!(
        serde_json::to_string(&ErrorSeverity::Low).expect("s"),
        "\"low\""
    );
    assert_eq!(
        serde_json::to_string(&ErrorSeverity::Medium).expect("s"),
        "\"medium\""
    );
    assert_eq!(
        serde_json::to_string(&ErrorSeverity::High).expect("s"),
        "\"high\""
    );
    assert_eq!(
        serde_json::to_string(&ErrorSeverity::Critical).expect("s"),
        "\"critical\""
    );
}

#[test]
fn test_stack_entry_status_all_variants() {
    assert_eq!(
        serde_json::to_string(&StackEntryStatus::Pending).expect("s"),
        "\"pending\""
    );
    assert_eq!(
        serde_json::to_string(&StackEntryStatus::Ready).expect("s"),
        "\"ready\""
    );
    assert_eq!(
        serde_json::to_string(&StackEntryStatus::Merging).expect("s"),
        "\"merging\""
    );
    assert_eq!(
        serde_json::to_string(&StackEntryStatus::Merged).expect("s"),
        "\"merged\""
    );
    assert_eq!(
        serde_json::to_string(&StackEntryStatus::Failed).expect("s"),
        "\"failed\""
    );
}

#[test]
fn test_queue_entry_status_all_variants() {
    assert_eq!(
        serde_json::to_string(&QueueEntryStatus::Pending).expect("s"),
        "\"pending\""
    );
    assert_eq!(
        serde_json::to_string(&QueueEntryStatus::Ready).expect("s"),
        "\"ready\""
    );
    assert_eq!(
        serde_json::to_string(&QueueEntryStatus::Claimed).expect("s"),
        "\"claimed\""
    );
    assert_eq!(
        serde_json::to_string(&QueueEntryStatus::InProgress).expect("s"),
        "\"in_progress\""
    );
    assert_eq!(
        serde_json::to_string(&QueueEntryStatus::Completed).expect("s"),
        "\"completed\""
    );
    assert_eq!(
        serde_json::to_string(&QueueEntryStatus::Failed).expect("s"),
        "\"failed\""
    );
    assert_eq!(
        serde_json::to_string(&QueueEntryStatus::Blocked).expect("s"),
        "\"blocked\""
    );
}

#[test]
fn test_train_status_all_variants() {
    assert_eq!(
        serde_json::to_string(&TrainStatus::Pending).expect("s"),
        "\"pending\""
    );
    assert_eq!(
        serde_json::to_string(&TrainStatus::Running).expect("s"),
        "\"running\""
    );
    assert_eq!(
        serde_json::to_string(&TrainStatus::Completed).expect("s"),
        "\"completed\""
    );
    assert_eq!(
        serde_json::to_string(&TrainStatus::Failed).expect("s"),
        "\"failed\""
    );
    assert_eq!(
        serde_json::to_string(&TrainStatus::Aborted).expect("s"),
        "\"aborted\""
    );
}

#[test]
fn test_train_action_all_variants() {
    assert_eq!(
        serde_json::to_string(&TrainAction::Sync).expect("s"),
        "\"sync\""
    );
    assert_eq!(
        serde_json::to_string(&TrainAction::Rebase).expect("s"),
        "\"rebase\""
    );
    assert_eq!(
        serde_json::to_string(&TrainAction::Merge).expect("s"),
        "\"merge\""
    );
    assert_eq!(
        serde_json::to_string(&TrainAction::Push).expect("s"),
        "\"push\""
    );
}

#[test]
fn test_train_step_status_all_variants() {
    assert_eq!(
        serde_json::to_string(&TrainStepStatus::Pending).expect("s"),
        "\"pending\""
    );
    assert_eq!(
        serde_json::to_string(&TrainStepStatus::Running).expect("s"),
        "\"running\""
    );
    assert_eq!(
        serde_json::to_string(&TrainStepStatus::Success).expect("s"),
        "\"success\""
    );
    assert_eq!(
        serde_json::to_string(&TrainStepStatus::Failed).expect("s"),
        "\"failed\""
    );
    assert_eq!(
        serde_json::to_string(&TrainStepStatus::Skipped).expect("s"),
        "\"skipped\""
    );
}

#[test]
fn test_summary_with_whitespace_message() {
    let result = Summary::new(SummaryType::Info, "   ".to_string());
    assert!(result.is_err());
}

#[test]
fn test_issue_with_whitespace_title() {
    let result = Issue::new(
        "id".to_string(),
        "   ".to_string(),
        IssueKind::Validation,
        IssueSeverity::Error,
    );
    assert!(result.is_err());
}

#[test]
fn test_warning_with_whitespace_message() {
    let result = Warning::new("CODE".to_string(), "   ".to_string());
    assert!(result.is_err());
}

#[test]
fn test_result_output_with_whitespace_message() {
    let result = ResultOutput::success(ResultKind::Command, "   ".to_string());
    assert!(result.is_err());
}

#[test]
fn test_jsonl_writer_produces_valid_jsonl() {
    use std::io::Cursor;

    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);

    let lines = vec![
        OutputLine::Summary(Summary::new(SummaryType::Info, "first".to_string()).expect("valid")),
        OutputLine::Summary(Summary::new(SummaryType::Info, "second".to_string()).expect("valid")),
        OutputLine::Summary(Summary::new(SummaryType::Info, "third".to_string()).expect("valid")),
    ];

    writer.emit_all(&lines).expect("emit all");

    let output = String::from_utf8(cursor.into_inner()).expect("utf8");

    for line in output.lines() {
        let parsed: serde_json::Value =
            serde_json::from_str(line).expect("each line is valid JSON");
        assert!(parsed.is_object());
    }
}

#[test]
fn test_queue_summary_edge_cases() {
    let empty = QueueSummary::new();
    assert!(empty.is_empty());
    assert!(!empty.has_blockers());

    let with_blockers = QueueSummary::new().with_counts(QueueCounts {
        total: 5,
        pending: 0,
        ready: 0,
        blocked: 5,
        in_progress: 0,
    });
    assert!(!with_blockers.is_empty());
    assert!(with_blockers.has_blockers());

    let full = QueueSummary::new().with_counts(QueueCounts {
        total: 100,
        pending: 25,
        ready: 25,
        blocked: 25,
        in_progress: 25,
    });
    assert!(!full.is_empty());
    assert!(full.has_blockers());
}
