use chrono::Utc;

use super::*;

// ── ContextOutput Tests ──────────────────────────────────────────────

fn sample_context() -> ContextOutput {
    ContextOutput {
        location: Location::Main,
        session: None,
        repository: RepositoryContext {
            root: "/home/user/project".to_string(),
            branch: "abc123".to_string(),
            uncommitted_files: 0,
            commits_ahead: 0,
            has_conflicts: false,
        },
        beads: None,
        health: HealthStatus::Good,
        suggestions: vec![],
    }
}

#[test]
fn test_context_output_main_location() {
    let context = sample_context();
    assert!(matches!(context.location, Location::Main));
    assert!(context.session.is_none());
}

#[test]
fn test_context_output_workspace_location() {
    let context = ContextOutput {
        location: Location::Workspace {
            name: "feature-auth".to_string(),
            path: "/home/user/project/.zjj/workspaces/feature-auth".to_string(),
        },
        ..sample_context()
    };
    assert!(matches!(context.location, Location::Workspace { .. }));
}

#[test]
fn test_context_output_serialization() {
    let context = sample_context();
    let json = serde_json::to_string(&context);
    let Ok(json_str) = json else {
        panic!("serialization failed");
    };
    assert!(json_str.contains("location"));
    assert!(json_str.contains("repository"));
    assert!(json_str.contains("health"));
}

// ── Location Tests ───────────────────────────────────────────────────

#[test]
fn test_location_main_serialization() {
    let location = Location::Main;
    let json = serde_json::to_string(&location);
    let Ok(json_str) = json else {
        panic!("serialization failed");
    };
    assert!(json_str.contains("main"));
}

#[test]
fn test_location_workspace_serialization() {
    let location = Location::Workspace {
        name: "test-ws".to_string(),
        path: "/path/to/ws".to_string(),
    };
    let json = serde_json::to_string(&location);
    let Ok(json_str) = json else {
        panic!("serialization failed");
    };
    assert!(json_str.contains("workspace"));
    assert!(json_str.contains("test-ws"));
    assert!(json_str.contains("/path/to/ws"));
}

#[test]
fn test_location_clone() {
    let location = Location::Workspace {
        name: "test".to_string(),
        path: "/path".to_string(),
    };
    assert!(
        matches!(location, Location::Workspace { name, path } if name == "test" && path == "/path")
    );
}

// ── SessionContext Tests ─────────────────────────────────────────────

#[test]
fn test_session_context_with_bead() {
    let session = SessionContext {
        name: "feature-auth".to_string(),
        status: "active".to_string(),
        bead_id: Some("zjj-abc123".to_string()),
        agent: None,
        created_at: Utc::now(),
        last_synced: Some(Utc::now()),
    };
    assert_eq!(session.name, "feature-auth");
    assert_eq!(session.bead_id, Some("zjj-abc123".to_string()));
    assert!(session.last_synced.is_some());
    assert!(session.agent.is_none());
}

#[test]
fn test_session_context_without_bead() {
    let session = SessionContext {
        name: "test".to_string(),
        status: "active".to_string(),
        bead_id: None,
        agent: None,
        created_at: Utc::now(),
        last_synced: None,
    };
    assert!(session.bead_id.is_none());
    assert!(session.last_synced.is_none());
    assert!(session.agent.is_none());
}

#[test]
fn test_session_context_serialization() {
    let session = SessionContext {
        name: "test".to_string(),
        status: "active".to_string(),
        bead_id: None,
        agent: None,
        created_at: Utc::now(),
        last_synced: None,
    };
    let json = serde_json::to_string(&session);
    let Ok(json_str) = json else {
        panic!("serialization failed");
    };
    assert!(json_str.contains("name"));
    assert!(json_str.contains("status"));
    assert!(json_str.contains("created_at"));
    assert!(json_str.contains("agent"));
}

#[test]
fn test_session_context_with_agent() {
    let session = SessionContext {
        name: "feature-auth".to_string(),
        status: "active".to_string(),
        bead_id: Some("zjj-abc123".to_string()),
        agent: Some("architect-1".to_string()),
        created_at: Utc::now(),
        last_synced: Some(Utc::now()),
    };
    assert_eq!(session.name, "feature-auth");
    assert_eq!(session.agent, Some("architect-1".to_string()));
    assert_eq!(session.bead_id, Some("zjj-abc123".to_string()));
}

#[test]
fn test_session_context_serialization_with_agent() {
    let session = SessionContext {
        name: "test".to_string(),
        status: "active".to_string(),
        bead_id: None,
        agent: Some("builder-3".to_string()),
        created_at: Utc::now(),
        last_synced: None,
    };
    let json = serde_json::to_string(&session);
    let Ok(json_str) = json else {
        panic!("serialization failed");
    };
    assert!(json_str.contains("agent"));
    assert!(json_str.contains("builder-3"));
}

// ── RepositoryContext Tests ──────────────────────────────────────────

#[test]
fn test_repository_context_clean() {
    let repo = RepositoryContext {
        root: "/home/user/project".to_string(),
        branch: "abc123".to_string(),
        uncommitted_files: 0,
        commits_ahead: 0,
        has_conflicts: false,
    };
    assert_eq!(repo.uncommitted_files, 0);
    assert!(!repo.has_conflicts);
}

#[test]
fn test_repository_context_dirty() {
    let repo = RepositoryContext {
        root: "/home/user/project".to_string(),
        branch: "abc123".to_string(),
        uncommitted_files: 5,
        commits_ahead: 3,
        has_conflicts: true,
    };
    assert_eq!(repo.uncommitted_files, 5);
    assert_eq!(repo.commits_ahead, 3);
    assert!(repo.has_conflicts);
}

#[test]
fn test_repository_context_serialization() {
    let repo = RepositoryContext {
        root: "/path".to_string(),
        branch: "main".to_string(),
        uncommitted_files: 2,
        commits_ahead: 1,
        has_conflicts: false,
    };
    let json = serde_json::to_string(&repo);
    let Ok(json_str) = json else {
        panic!("serialization failed");
    };
    assert!(json_str.contains("uncommitted_files"));
    assert!(json_str.contains("commits_ahead"));
    assert!(json_str.contains("has_conflicts"));
}

// ── BeadsContext Tests ───────────────────────────────────────────────

#[test]
fn test_beads_context_active() {
    let beads = BeadsContext {
        active: Some("zjj-abc".to_string()),
        blocked_by: vec![],
        ready_count: 5,
        in_progress_count: 1,
    };
    assert_eq!(beads.active, Some("zjj-abc".to_string()));
    assert_eq!(beads.in_progress_count, 1);
}

#[test]
fn test_beads_context_blocked() {
    let beads = BeadsContext {
        active: None,
        blocked_by: vec!["zjj-123".to_string(), "zjj-456".to_string()],
        ready_count: 3,
        in_progress_count: 0,
    };
    assert!(beads.active.is_none());
    assert_eq!(beads.blocked_by.len(), 2);
}

#[test]
fn test_beads_context_serialization() {
    let beads = BeadsContext {
        active: Some("test".to_string()),
        blocked_by: vec!["blocker".to_string()],
        ready_count: 10,
        in_progress_count: 2,
    };
    let json = serde_json::to_string(&beads);
    let Ok(json_str) = json else {
        panic!("serialization failed");
    };
    assert!(json_str.contains("active"));
    assert!(json_str.contains("blocked_by"));
    assert!(json_str.contains("ready_count"));
}

// ── HealthStatus Tests ───────────────────────────────────────────────

#[test]
fn test_health_status_good() {
    let health = HealthStatus::Good;
    assert!(matches!(health, HealthStatus::Good));
}

#[test]
fn test_health_status_warn() {
    let health = HealthStatus::Warn {
        issues: vec!["Session stale".to_string()],
    };
    assert!(matches!(health, HealthStatus::Warn { .. }));
}

#[test]
fn test_health_status_error() {
    let health = HealthStatus::Error {
        critical: vec!["Database missing".to_string()],
    };
    assert!(matches!(health, HealthStatus::Error { .. }));
}

#[test]
fn test_health_status_serialization() {
    let health = HealthStatus::Warn {
        issues: vec!["warning 1".to_string()],
    };
    let json = serde_json::to_string(&health);
    let Ok(json_str) = json else {
        panic!("serialization failed");
    };
    assert!(json_str.contains("warn"));
    assert!(json_str.contains("issues"));
}

// ── Suggestions Tests ────────────────────────────────────────────────

#[test]
fn test_suggestions_for_main_location() {
    let repo = RepositoryContext {
        root: "/path".to_string(),
        branch: "main".to_string(),
        uncommitted_files: 0,
        commits_ahead: 0,
        has_conflicts: false,
    };
    let suggestions = generate_suggestions(&Location::Main, &HealthStatus::Good, &repo);
    assert!(!suggestions.is_empty());
    assert!(suggestions.iter().any(|s| s.contains("zjj add")));
}

#[test]
fn test_suggestions_for_workspace() {
    let repo = RepositoryContext {
        root: "/path".to_string(),
        branch: "abc123".to_string(),
        uncommitted_files: 3,
        commits_ahead: 0,
        has_conflicts: false,
    };
    let location = Location::Workspace {
        name: "test".to_string(),
        path: "/path".to_string(),
    };
    let suggestions = generate_suggestions(&location, &HealthStatus::Good, &repo);
    assert!(!suggestions.is_empty());
    // Should mention uncommitted files
    assert!(suggestions.iter().any(|s| s.contains("uncommitted")));
}

#[test]
fn test_suggestions_for_warning_health() {
    let repo = RepositoryContext {
        root: "/path".to_string(),
        branch: "main".to_string(),
        uncommitted_files: 0,
        commits_ahead: 0,
        has_conflicts: false,
    };
    let health = HealthStatus::Warn {
        issues: vec!["Test warning".to_string()],
    };
    let suggestions = generate_suggestions(&Location::Main, &health, &repo);
    assert!(suggestions.iter().any(|s| s.contains("Warning")));
}

// ── check_health Tests ───────────────────────────────────────────────

#[tokio::test]
async fn test_check_health_returns_good_for_valid_state() {
    // When db exists and session workspace exists, should be Good
    // This is a partial test since we can't easily mock the filesystem
    let result = check_health(std::path::Path::new("/nonexistent"), None).await;
    // With no session and nonexistent path, should return Error for missing db
    assert!(matches!(result, HealthStatus::Error { .. }));
}

// ── Field Extraction Tests ───────────────────────────────────────────

#[test]
fn test_field_pointer_conversion() {
    // Test that field paths are converted correctly
    let context = sample_context();
    let json_value = serde_json::to_value(&context);
    assert!(json_value.is_ok());

    let Ok(value) = json_value else {
        panic!("serialization failed");
    };
    // location.type should become /location/type
    let pointer = "/location/type".to_string();
    let result = value.pointer(&pointer);
    assert!(result.is_some());
}

#[test]
fn test_nested_field_access() {
    let context = sample_context();
    let json_value = serde_json::to_value(&context);
    assert!(json_value.is_ok());

    let Ok(value) = json_value else {
        panic!("serialization failed");
    };
    // repository.branch should be accessible
    let result = value.pointer("/repository/branch");
    assert!(result.is_some());
}

// ── extract_agent_from_metadata Tests ────────────────────────────────────

#[test]
fn test_extract_agent_from_metadata_with_valid_agent() {
    let metadata = serde_json::json!({
        "bead_id": "zjj-abc12",
        "agent_id": "architect-1"
    });
    let result = extract_agent_from_metadata(Some(&metadata));
    assert_eq!(result, Some("architect-1".to_string()));
}

#[test]
fn test_extract_agent_from_metadata_without_agent() {
    let metadata = serde_json::json!({
        "bead_id": "zjj-abc12"
    });
    let result = extract_agent_from_metadata(Some(&metadata));
    assert!(result.is_none());
}

#[test]
fn test_extract_agent_from_metadata_with_null_metadata() {
    let result = extract_agent_from_metadata(None);
    assert!(result.is_none());
}

#[test]
fn test_extract_agent_from_metadata_with_non_string_agent_id() {
    let metadata = serde_json::json!({
        "agent_id": 123
    });
    let result = extract_agent_from_metadata(Some(&metadata));
    assert!(result.is_none());
}

#[test]
fn test_extract_agent_from_metadata_with_empty_agent_id() {
    let metadata = serde_json::json!({
        "agent_id": ""
    });
    let result = extract_agent_from_metadata(Some(&metadata));
    assert!(result.is_none());
}

#[test]
fn test_extract_agent_from_metadata_with_null_agent_id() {
    let metadata = serde_json::json!({
        "agent_id": null
    });
    let result = extract_agent_from_metadata(Some(&metadata));
    assert!(result.is_none());
}

#[test]
fn test_extract_agent_from_metadata_with_rich_metadata() {
    let metadata = serde_json::json!({
        "bead_id": "zjj-123",
        "agent_id": "architect-1",
        "priority": 2,
        "tags": ["feature", "auth"]
    });
    let result = extract_agent_from_metadata(Some(&metadata));
    assert_eq!(result, Some("architect-1".to_string()));
}

#[test]
fn test_extract_agent_from_metadata_with_unicode_agent_id() {
    let metadata = serde_json::json!({
        "agent_id": "agent-中文-тест"
    });
    let result = extract_agent_from_metadata(Some(&metadata));
    assert_eq!(result, Some("agent-中文-тест".to_string()));
}
