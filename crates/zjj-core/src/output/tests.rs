//! JSONL output module tests

use std::{io::Cursor, path::PathBuf};

use super::{
    emit,
    types::{
        Action, ActionStatus, Assessment, ErrorSeverity, Issue, IssueKind, IssueSeverity, Plan,
        Recovery, ResultKind, ResultOutput, SessionOutput, Summary, SummaryType, Warning,
    },
    JsonlWriter, OutputLine,
};
use crate::{types::SessionStatus, workspace_state::WorkspaceState};

macro_rules! ok_or_return {
    ($expr:expr) => {{
        let result = $expr;
        assert!(result.is_ok());
        let Ok(value) = result else {
            return;
        };
        value
    }};
}

macro_rules! some_or_return {
    ($expr:expr) => {{
        let option = $expr;
        assert!(option.is_some());
        let Some(value) = option else {
            return;
        };
        value
    }};
}

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
    let summary = ok_or_return!(Summary::new(SummaryType::Info, "test".to_string()))
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
    .map(|value| value.with_branch("feature-branch".to_string()));
    let session = ok_or_return!(session);
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
    .map(|value| {
        value
            .with_session("session-1".to_string())
            .with_suggestion("Try this fix".to_string())
    });
    let issue = ok_or_return!(issue);
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
    let plan = Plan::new("My Plan".to_string(), "Description".to_string()).map(|value| {
        value
            .with_step("Step 1".to_string(), ActionStatus::Pending)
            .with_step("Step 2".to_string(), ActionStatus::InProgress)
    });
    let plan = ok_or_return!(plan);
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
        .map(|value| value.with_context("session-1".to_string(), PathBuf::from("/workspace")));
    let warning = ok_or_return!(warning);
    assert!(warning.context.is_some());
    let ctx = some_or_return!(warning.context);
    assert_eq!(ctx.session, "session-1");
}

#[test]
fn test_result_output_success() {
    let result = ResultOutput::success(ResultKind::Command, "Command succeeded".to_string());
    assert!(result.is_ok());
    let output = ok_or_return!(result);
    assert!(output.success);
}

#[test]
fn test_result_output_failure() {
    let result = ResultOutput::failure(ResultKind::Operation, "Operation failed".to_string());
    assert!(result.is_ok());
    let output = ok_or_return!(result);
    assert!(!output.success);
}

#[test]
fn test_result_output_validates_empty_message() {
    let result = ResultOutput::success(ResultKind::Command, String::new());
    assert!(result.is_err());
}

#[test]
fn test_output_line_kind() {
    let summary = OutputLine::Summary(ok_or_return!(Summary::new(
        SummaryType::Info,
        "test".to_string(),
    )));
    assert_eq!(summary.kind(), "summary");

    let issue = OutputLine::Issue(ok_or_return!(Issue::new(
        "1".to_string(),
        "t".to_string(),
        IssueKind::Validation,
        IssueSeverity::Error,
    )));
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
    let json = ok_or_return!(serde_json::to_string(&severity));
    assert_eq!(json, "\"warning\"");
}

#[test]
fn test_action_status_serialization() {
    let status = ActionStatus::InProgress;
    let json = ok_or_return!(serde_json::to_string(&status));
    assert_eq!(json, "\"in_progress\"");
}

#[test]
fn test_jsonl_writer_new() {
    let cursor = Cursor::new(Vec::new());
    let _writer = JsonlWriter::new(cursor);
}

#[test]
fn test_jsonl_writer_emit_summary() {
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);
    let summary = ok_or_return!(Summary::new(SummaryType::Info, "test message".to_string()));
    let output_line = OutputLine::Summary(summary);

    ok_or_return!(writer.emit(&output_line));

    let output = ok_or_return!(String::from_utf8(cursor.into_inner()));
    let parsed: serde_json::Value = ok_or_return!(serde_json::from_str(output.trim()));
    assert!(parsed.is_object());
}

#[test]
fn test_jsonl_writer_emit_session() {
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);
    let session = SessionOutput::new(
        "feature-x".to_string(),
        SessionStatus::Active,
        WorkspaceState::Working,
        PathBuf::from("/tmp/ws"),
    );
    let session = ok_or_return!(session);
    let output_line = OutputLine::Session(session);

    ok_or_return!(writer.emit(&output_line));

    let output = ok_or_return!(String::from_utf8(cursor.into_inner()));
    let parsed: serde_json::Value = ok_or_return!(serde_json::from_str(output.trim()));
    assert!(parsed.is_object());
}

#[test]
fn test_jsonl_writer_emit_issue() {
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);
    let issue = Issue::new(
        "ISS-1".to_string(),
        "Test issue".to_string(),
        IssueKind::Validation,
        IssueSeverity::Error,
    );
    let issue = ok_or_return!(issue);
    let output_line = OutputLine::Issue(issue);

    ok_or_return!(writer.emit(&output_line));

    let output = ok_or_return!(String::from_utf8(cursor.into_inner()));
    let parsed: serde_json::Value = ok_or_return!(serde_json::from_str(output.trim()));
    assert!(parsed.is_object());
}

#[test]
fn test_jsonl_writer_emit_plan() {
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);
    let plan = Plan::new("My Plan".to_string(), "Description".to_string());
    let plan = ok_or_return!(plan);
    let output_line = OutputLine::Plan(plan);

    ok_or_return!(writer.emit(&output_line));

    let output = ok_or_return!(String::from_utf8(cursor.into_inner()));
    let parsed: serde_json::Value = ok_or_return!(serde_json::from_str(output.trim()));
    assert!(parsed.is_object());
}

#[test]
fn test_jsonl_writer_emit_action() {
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);
    let action = Action::new(
        "create".to_string(),
        "session-x".to_string(),
        ActionStatus::Completed,
    );
    let output_line = OutputLine::Action(action);

    ok_or_return!(writer.emit(&output_line));

    let output = ok_or_return!(String::from_utf8(cursor.into_inner()));
    let parsed: serde_json::Value = ok_or_return!(serde_json::from_str(output.trim()));
    assert!(parsed.is_object());
}

#[test]
fn test_jsonl_writer_emit_warning() {
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);
    let warning = Warning::new("W001".to_string(), "Test warning".to_string());
    let warning = ok_or_return!(warning);
    let output_line = OutputLine::Warning(warning);

    ok_or_return!(writer.emit(&output_line));

    let output = ok_or_return!(String::from_utf8(cursor.into_inner()));
    let parsed: serde_json::Value = ok_or_return!(serde_json::from_str(output.trim()));
    assert!(parsed.is_object());
}

#[test]
fn test_jsonl_writer_emit_result() {
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);
    let result = ResultOutput::success(ResultKind::Command, "Command succeeded".to_string());
    let result = ok_or_return!(result);
    let output_line = OutputLine::Result(result);

    ok_or_return!(writer.emit(&output_line));

    let output = ok_or_return!(String::from_utf8(cursor.into_inner()));
    let parsed: serde_json::Value = ok_or_return!(serde_json::from_str(output.trim()));
    assert!(parsed.is_object());
}

#[test]
fn test_jsonl_writer_emit_all() {
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);

    let summary1 = Summary::new(SummaryType::Info, "message 1".to_string());
    let summary2 = Summary::new(SummaryType::Status, "message 2".to_string());
    let summary1 = ok_or_return!(summary1);
    let summary2 = ok_or_return!(summary2);

    let lines = vec![OutputLine::Summary(summary1), OutputLine::Summary(summary2)];

    ok_or_return!(writer.emit_all(&lines));

    let output = ok_or_return!(String::from_utf8(cursor.into_inner()));
    let lines_output: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines_output.len(), 2);

    for line in &lines_output {
        let parsed: Result<serde_json::Value, serde_json::Error> = serde_json::from_str(line);
        assert!(parsed.is_ok());
    }
}

#[test]
fn test_jsonl_writer_emit_all_empty() {
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);

    let lines: Vec<OutputLine> = vec![];
    ok_or_return!(writer.emit_all(&lines));

    let output = ok_or_return!(String::from_utf8(cursor.into_inner()));
    assert!(output.is_empty());
}

#[test]
fn test_jsonl_writer_flush() {
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);

    let summary = Summary::new(SummaryType::Info, "test".to_string());
    let summary = ok_or_return!(summary);
    ok_or_return!(writer.emit(&OutputLine::Summary(summary)));
    ok_or_return!(writer.flush());

    assert!(!cursor.get_ref().is_empty());
}

#[test]
fn test_emit_function() {
    let mut cursor = Cursor::new(Vec::new());
    let summary = Summary::new(SummaryType::Info, "test message".to_string());
    let summary = ok_or_return!(summary);
    let output_line = OutputLine::Summary(summary);

    ok_or_return!(emit(&mut cursor, &output_line));

    let output = ok_or_return!(String::from_utf8(cursor.into_inner()));
    let parsed: serde_json::Value = ok_or_return!(serde_json::from_str(output.trim()));
    assert!(parsed.is_object());
}

#[test]
fn test_emit_produces_valid_jsonl() {
    let mut cursor = Cursor::new(Vec::new());

    for i in 0..100 {
        let summary = Summary::new(SummaryType::Count, format!("message {}", i));
        let summary = ok_or_return!(summary);
        ok_or_return!(emit(&mut cursor, &OutputLine::Summary(summary)));
    }

    let output = ok_or_return!(String::from_utf8(cursor.into_inner()));
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 100);

    for (i, line) in lines.iter().enumerate() {
        let message = format!("line {} valid json", i);
        let parsed: Result<serde_json::Value, serde_json::Error> = serde_json::from_str(line);
        assert!(parsed.is_ok(), "{message}");
    }
}

#[test]
fn test_jsonl_writer_handles_special_characters() {
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);

    let summary = Summary::new(
        SummaryType::Info,
        "test \"quotes\" and \n newlines".to_string(),
    );
    let summary = ok_or_return!(summary);
    ok_or_return!(writer.emit(&OutputLine::Summary(summary)));

    let output = ok_or_return!(String::from_utf8(cursor.into_inner()));
    let parsed: Result<serde_json::Value, serde_json::Error> = serde_json::from_str(output.trim());
    assert!(parsed.is_ok());
}

#[test]
fn test_jsonl_writer_handles_unicode() {
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);

    let summary = Summary::new(SummaryType::Info, "Hello 世界 🌍".to_string());
    let summary = ok_or_return!(summary);
    ok_or_return!(writer.emit(&OutputLine::Summary(summary)));

    let output = ok_or_return!(String::from_utf8(cursor.into_inner()));
    let parsed: Result<serde_json::Value, serde_json::Error> = serde_json::from_str(output.trim());
    assert!(parsed.is_ok());
}

#[test]
fn test_jsonl_writer_newlines_between_lines() {
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);

    let summary1 = Summary::new(SummaryType::Info, "first".to_string());
    let summary2 = Summary::new(SummaryType::Info, "second".to_string());
    let summary1 = ok_or_return!(summary1);
    let summary2 = ok_or_return!(summary2);

    ok_or_return!(writer.emit(&OutputLine::Summary(summary1)));
    ok_or_return!(writer.emit(&OutputLine::Summary(summary2)));

    let output = ok_or_return!(String::from_utf8(cursor.into_inner()));
    assert!(output.contains('\n'));
}

#[test]
fn test_emit_all_with_multiple_lines() {
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);

    let summary1 = Summary::new(SummaryType::Info, "first".to_string());
    let summary2 = Summary::new(SummaryType::Info, "second".to_string());
    let summary1 = ok_or_return!(summary1);
    let summary2 = ok_or_return!(summary2);

    let result = writer.emit_all(&[OutputLine::Summary(summary1), OutputLine::Summary(summary2)]);

    assert!(result.is_ok());
}
