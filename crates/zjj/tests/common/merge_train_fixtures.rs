//! Test fixtures for merge train E2E tests
//!
//! This module provides test utilities for setting up merge train tests:
//! - TestEntryBuilder: Fluent API for creating test queue entries
//! - MockJJExecutor: Simulates jj commands
//! - MockTestRunner: Simulates test execution
//! - InMemoryQueueFactory: Creates isolated test queues
//! - JsonlCapturer: Captures and parses JSONL output

// Allow test code ergonomics
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::future_not_send,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::redundant_closure_for_method_calls
)]

use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use tokio::sync::RwLock;
use zjj_core::{
    coordination::{MergeQueue, QueueEntry, QueueStatus},
    Error, Result,
};

/// Test configuration constants
pub const TEST_LOCK_TIMEOUT_SECS: i64 = 300;
pub const TEST_DEFAULT_TIMEOUT_SECS: i64 = 600;
pub const TEST_MAX_ATTEMPTS: i32 = 3;

/// Global entry ID counter for deterministic test IDs
static ENTRY_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Reset the entry ID counter (call between tests)
pub fn reset_entry_id_counter() {
    ENTRY_ID_COUNTER.store(1, Ordering::SeqCst);
}

/// Generate next test entry ID
fn next_entry_id() -> i64 {
    ENTRY_ID_COUNTER
        .fetch_add(1, Ordering::SeqCst)
        .cast_signed()
}

/// Builder for creating test queue entries with fluent API
#[derive(Debug, Clone)]
pub struct TestEntryBuilder {
    workspace: String,
    bead_id: Option<String>,
    priority: i64,
    status: QueueStatus,
    head_sha: Option<String>,
    tested_against_sha: Option<String>,
    dedupe_key: Option<String>,
    agent_id: Option<String>,
    error_message: Option<String>,
    attempt_count: i32,
    max_attempts: i32,
    test_timeout_secs: i64,
}

impl TestEntryBuilder {
    /// Create a new test entry builder
    pub fn new(workspace: impl Into<String>) -> Self {
        Self {
            workspace: workspace.into(),
            bead_id: None,
            priority: 5,
            status: QueueStatus::Pending,
            head_sha: Some("abc123def456".to_string()),
            tested_against_sha: None,
            dedupe_key: None,
            agent_id: Some("test-agent".to_string()),
            error_message: None,
            attempt_count: 0,
            max_attempts: TEST_MAX_ATTEMPTS,
            test_timeout_secs: TEST_DEFAULT_TIMEOUT_SECS,
        }
    }

    /// Set the bead ID
    pub fn with_bead_id(mut self, bead_id: impl Into<String>) -> Self {
        self.bead_id = Some(bead_id.into());
        self
    }

    /// Set the priority (lower = higher priority)
    pub const fn with_priority(mut self, priority: i64) -> Self {
        self.priority = priority;
        self
    }

    /// Set the initial status
    pub const fn with_status(mut self, status: QueueStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the head SHA
    pub fn with_head_sha(mut self, sha: impl Into<String>) -> Self {
        self.head_sha = Some(sha.into());
        self
    }

    /// Set the tested_against_sha
    pub fn with_tested_against_sha(mut self, sha: impl Into<String>) -> Self {
        self.tested_against_sha = Some(sha.into());
        self
    }

    /// Set the dedupe key
    pub fn with_dedupe_key(mut self, key: impl Into<String>) -> Self {
        self.dedupe_key = Some(key.into());
        self
    }

    /// Set the agent ID
    pub fn with_agent_id(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Set the error message
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error_message = Some(error.into());
        self
    }

    /// Set the attempt count
    pub const fn with_attempt_count(mut self, count: i32) -> Self {
        self.attempt_count = count;
        self
    }

    /// Set the max attempts
    pub const fn with_max_attempts(mut self, max: i32) -> Self {
        self.max_attempts = max;
        self
    }

    /// Set the test timeout in seconds
    pub const fn with_test_timeout(mut self, timeout_secs: i64) -> Self {
        self.test_timeout_secs = timeout_secs;
        self
    }

    /// Build and insert the entry into the queue
    pub async fn build_and_insert(self, queue: &MergeQueue) -> Result<QueueEntry> {
        let entry_id = next_entry_id();

        // Insert directly into database
        let now = Utc::now().timestamp();
        let status_str = self.status.as_str();

        sqlx::query(
            "INSERT INTO merge_queue (
                id, workspace, bead_id, priority, status,
                added_at, started_at, completed_at,
                error_message, agent_id, dedupe_key,
                head_sha, tested_against_sha,
                attempt_count, max_attempts
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(entry_id)
        .bind(&self.workspace)
        .bind(&self.bead_id)
        .bind(self.priority)
        .bind(status_str)
        .bind(now)
        .bind(self.started_at().map(|dt| dt.timestamp()))
        .bind(self.completed_at().map(|dt| dt.timestamp()))
        .bind(&self.error_message)
        .bind(&self.agent_id)
        .bind(&self.dedupe_key)
        .bind(&self.head_sha)
        .bind(&self.tested_against_sha)
        .bind(self.attempt_count)
        .bind(self.max_attempts)
        .execute(queue.pool())
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to insert test entry: {e}")))?;

        // Fetch and return the entry
        let entry = sqlx::query_as::<_, QueueEntry>("SELECT * FROM merge_queue WHERE id = ?")
            .bind(entry_id)
            .fetch_one(queue.pool())
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to fetch test entry: {e}")))?;

        Ok(entry)
    }

    fn started_at(&self) -> Option<DateTime<Utc>> {
        if matches!(
            self.status,
            QueueStatus::Claimed
                | QueueStatus::Rebasing
                | QueueStatus::Testing
                | QueueStatus::ReadyToMerge
                | QueueStatus::Merging
        ) {
            Some(Utc::now())
        } else {
            None
        }
    }

    fn completed_at(&self) -> Option<DateTime<Utc>> {
        if matches!(
            self.status,
            QueueStatus::Merged
                | QueueStatus::FailedRetryable
                | QueueStatus::FailedTerminal
                | QueueStatus::Cancelled
        ) {
            Some(Utc::now())
        } else {
            None
        }
    }
}

/// Mock JJ command executor for testing
#[derive(Debug, Clone)]
pub struct MockJJExecutor {
    commands: Rc<RwLock<VecDeque<MockCommand>>>,
}

/// Mock JJ command results
#[derive(Debug, Clone)]
pub enum MockCommand {
    /// Rebase command result
    Rebase {
        result: std::result::Result<String, String>,
    },
    /// Merge command result
    Merge {
        result: std::result::Result<String, String>,
    },
    /// Log command output
    Log { output: String },
    /// Conflict check result
    ConflictCheck { has_conflicts: bool },
    /// Bookmark update result
    BookmarkUpdate {
        result: std::result::Result<(), String>,
    },
}

impl MockJJExecutor {
    /// Create a new mock executor
    pub fn new() -> Self {
        Self {
            commands: Rc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Queue a mock command
    pub async fn queue_command(&self, command: MockCommand) {
        self.commands.write().await.push_back(command);
    }

    /// Pop the next command (for testing expectations)
    pub async fn pop_command(&self) -> Option<MockCommand> {
        self.commands.write().await.pop_front()
    }

    /// Get remaining command count
    pub async fn command_count(&self) -> usize {
        self.commands.read().await.len()
    }

    /// Clear all queued commands
    pub async fn clear(&self) {
        self.commands.write().await.clear();
    }
}

impl Default for MockJJExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock test runner for simulating test execution
#[derive(Debug, Clone)]
pub struct MockTestRunner {
    results: Rc<RwLock<HashMap<String, TestResult>>>,
}

/// Test execution result
#[derive(Debug, Clone)]
pub struct TestResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_secs: i64,
}

impl TestResult {
    /// Create a successful test result
    pub fn success() -> Self {
        Self {
            exit_code: 0,
            stdout: "All tests passed".to_string(),
            stderr: String::new(),
            duration_secs: 10,
        }
    }

    /// Create a failed test result
    pub fn failure(reason: impl Into<String>) -> Self {
        Self {
            exit_code: 1,
            stdout: String::new(),
            stderr: reason.into(),
            duration_secs: 5,
        }
    }

    /// Create a timeout test result
    pub fn timeout() -> Self {
        Self {
            exit_code: 124, // timeout exit code
            stdout: String::new(),
            stderr: "Test timeout exceeded".to_string(),
            duration_secs: 600,
        }
    }
}

impl MockTestRunner {
    /// Create a new mock test runner
    pub fn new() -> Self {
        Self {
            results: Rc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set a test result for a workspace
    pub async fn set_result(&self, workspace: impl Into<String>, result: TestResult) {
        self.results.write().await.insert(workspace.into(), result);
    }

    /// Get the test result for a workspace
    pub async fn get_result(&self, workspace: &str) -> Option<TestResult> {
        self.results.read().await.get(workspace).cloned()
    }

    /// Clear all test results
    pub async fn clear(&self) {
        self.results.write().await.clear();
    }

    /// Set all future tests to pass
    pub async fn all_pass(&self) {
        self.clear().await;
    }

    /// Set all future tests to fail
    pub async fn all_fail(&self, reason: impl Into<String>) {
        let result = TestResult::failure(reason);
        // Store a marker that all tests should fail
        self.results.write().await.insert("*".to_string(), result);
    }
}

impl Default for MockTestRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Factory for creating in-memory queues for testing
#[derive(Debug)]
pub struct InMemoryQueueFactory;

impl InMemoryQueueFactory {
    /// Create a new in-memory merge queue for testing
    pub async fn create() -> Result<MergeQueue> {
        reset_entry_id_counter();
        MergeQueue::open_in_memory().await
    }

    /// Create a queue with initial test data
    pub async fn create_with_entries(entries: Vec<TestEntryBuilder>) -> Result<MergeQueue> {
        let queue = Self::create().await?;

        for entry_builder in entries {
            entry_builder.build_and_insert(&queue).await?;
        }

        Ok(queue)
    }

    /// Create a queue with a single test entry
    pub async fn create_single_entry(workspace: &str) -> Result<MergeQueue> {
        Self::create_with_entries(vec![TestEntryBuilder::new(workspace)]).await
    }

    /// Create a queue with multiple test entries
    pub async fn create_multiple_entries(workspaces: &[&str]) -> Result<MergeQueue> {
        let entries: Vec<TestEntryBuilder> = workspaces
            .iter()
            .enumerate()
            .map(|(i, ws)| {
                TestEntryBuilder::new(*ws).with_priority(i64::try_from(i).unwrap_or(i64::MAX) + 1)
            })
            .collect();

        Self::create_with_entries(entries).await
    }
}

/// Capturer for JSONL output events
#[derive(Debug, Clone)]
pub struct JsonlCapturer {
    events: Rc<RwLock<Vec<JsonValue>>>,
}

impl JsonlCapturer {
    /// Create a new JSONL capturer
    pub fn new() -> Self {
        Self {
            events: Rc::new(RwLock::new(Vec::new())),
        }
    }

    /// Parse JSONL output and capture events
    pub async fn parse(&self, output: &str) -> Result<usize> {
        let mut count = 0;
        for line in output.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let json: JsonValue = serde_json::from_str(line)
                .map_err(|e| Error::InvalidConfig(format!("Invalid JSONL: {e}")))?;
            self.events.write().await.push(json);
            count += 1;
        }
        Ok(count)
    }

    /// Get all captured events
    pub async fn events(&self) -> Vec<JsonValue> {
        self.events.read().await.clone()
    }

    /// Clear all captured events
    pub async fn clear(&self) {
        self.events.write().await.clear();
    }

    /// Get event count
    pub async fn count(&self) -> usize {
        self.events.read().await.len()
    }

    /// Find events by type
    pub async fn find_by_type(&self, event_type: &str) -> Vec<JsonValue> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|e| e.get("type").and_then(|t| t.as_str()) == Some(event_type))
            .cloned()
            .collect()
    }

    /// Find TrainStep events for a specific entry
    pub async fn find_train_steps(&self, entry_id: i64) -> Vec<JsonValue> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|e| {
                e.get("type").and_then(|t| t.as_str()) == Some("TrainStep")
                    && e.get("entry_id").and_then(|i| i.as_i64()) == Some(entry_id)
            })
            .cloned()
            .collect()
    }

    /// Get the final TrainResult event
    pub async fn train_result(&self) -> Option<JsonValue> {
        let events = self.events.read().await;
        events
            .iter()
            .find(|e| e.get("type").and_then(|t| t.as_str()) == Some("TrainResult"))
            .cloned()
    }
}

impl Default for JsonlCapturer {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper struct for workspace paths in tests
#[derive(Debug, Clone)]
pub struct TestWorkspace {
    pub name: String,
    pub path: PathBuf,
    pub head_sha: String,
}

impl TestWorkspace {
    /// Create a new test workspace
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            name: name.clone(),
            path: PathBuf::from(format!("/tmp/test-zjj-workspaces/{name}")),
            head_sha: format!("{:012x}", rand::random::<u64>()),
        }
    }

    /// Create with specific head SHA
    pub fn with_sha(name: impl Into<String>, sha: impl Into<String>) -> Self {
        let mut ws = Self::new(name);
        ws.head_sha = sha.into();
        ws
    }
}

/// Helper for creating test timestamps
pub struct TestTime;

impl TestTime {
    /// Get current time as DateTime
    pub fn now() -> DateTime<Utc> {
        Utc::now()
    }

    /// Get time N seconds ago
    pub fn seconds_ago(n: i64) -> DateTime<Utc> {
        Utc::now() - chrono::Duration::seconds(n)
    }

    /// Get time N minutes ago
    pub fn minutes_ago(n: i64) -> DateTime<Utc> {
        Utc::now() - chrono::Duration::minutes(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_entry_builder_creates_pending_entry() {
        let queue = InMemoryQueueFactory::create().await.unwrap();
        let entry = TestEntryBuilder::new("test-workspace")
            .build_and_insert(&queue)
            .await
            .unwrap();

        assert_eq!(entry.workspace, "test-workspace");
        assert_eq!(entry.status, QueueStatus::Pending);
        assert_eq!(entry.priority, 5);
        assert_eq!(entry.attempt_count, 0);
    }

    #[tokio::test]
    async fn test_entry_builder_with_custom_fields() {
        let queue = InMemoryQueueFactory::create().await.unwrap();
        let entry = TestEntryBuilder::new("custom-workspace")
            .with_bead_id("bd-test")
            .with_priority(1)
            .with_attempt_count(2)
            .build_and_insert(&queue)
            .await
            .unwrap();

        assert_eq!(entry.workspace, "custom-workspace");
        assert_eq!(entry.bead_id.as_deref(), Some("bd-test"));
        assert_eq!(entry.priority, 1);
        assert_eq!(entry.attempt_count, 2);
    }

    #[tokio::test]
    async fn test_in_memory_queue_factory() {
        let queue = InMemoryQueueFactory::create().await.unwrap();
        let entry = TestEntryBuilder::new("ws1")
            .build_and_insert(&queue)
            .await
            .unwrap();

        assert_eq!(entry.id, 1); // First entry gets ID 1
    }

    #[tokio::test]
    async fn test_multiple_entries_increment_ids() {
        let queue = InMemoryQueueFactory::create_multiple_entries(&["ws1", "ws2", "ws3"])
            .await
            .unwrap();

        let entries = queue.list(Some(QueueStatus::Pending)).await.unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[tokio::test]
    async fn test_jsonl_capturer_parse() {
        let capturer = JsonlCapturer::new();
        let jsonl = r#"{"type":"TrainStep","action":"Claimed"}
{"type":"TrainStep","action":"Testing"}
{"type":"TrainResult"}"#;

        let count = capturer.parse(jsonl).await.unwrap();
        assert_eq!(count, 3);
        assert_eq!(capturer.count().await, 3);
    }

    #[tokio::test]
    async fn test_jsonl_capturer_filter_by_type() {
        let capturer = JsonlCapturer::new();
        let jsonl = r#"{"type":"TrainStep","entry_id":1}
{"type":"TrainResult"}
{"type":"TrainStep","entry_id":2}"#;

        capturer.parse(jsonl).await.unwrap();
        let steps = capturer.find_by_type("TrainStep").await;
        assert_eq!(steps.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_test_runner_results() {
        let runner = MockTestRunner::new();
        runner.set_result("ws1", TestResult::success()).await;

        let result = runner.get_result("ws1").await.unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("passed"));
    }

    #[tokio::test]
    async fn test_test_workspace() {
        let ws = TestWorkspace::new("test-ws");
        assert_eq!(ws.name, "test-ws");
        assert!(ws.path.to_string_lossy().contains("test-ws"));
        assert!(!ws.head_sha.is_empty());
    }
}
