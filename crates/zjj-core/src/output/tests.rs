//! JSONL output module tests

use super::*;
use crate::{SessionStatus, WorkspaceState};
use std::path::PathBuf;

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
