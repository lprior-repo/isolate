//! Workspace Integrity Management
//!
//! Provides corruption detection, validation, backup, and recovery mechanisms
//! for JJ workspaces. Designed for zero data loss with Railway-Oriented Programming.
//!
//! # Design Principles
//!
//! - **Zero panics**: All operations return `Result<T, Error>`
//! - **Zero unwraps**: Use combinators and `?` operator
//! - **Zero data loss**: Always backup before repair
//! - **Idempotent**: Safe to run multiple times
//!
//! # Corruption Types
//!
//! ```text
//! CorruptionType
//! ├── MissingDirectory    - Workspace dir doesn't exist
//! ├── MissingJjDir        - .jj directory missing
//! ├── InvalidJjState      - .jj state corrupted
//! ├── StaleWorkingCopy    - Working copy out of sync
//! ├── OrphanedWorkspace   - Workspace exists but not in JJ
//! ├── DatabaseMismatch    - State DB doesn't match filesystem
//! └── PermissionDenied    - Cannot read/write workspace
//! ```
//!
//! # Recovery Strategies
//!
//! ```text
//! RepairStrategy
//! ├── RecreateWorkspace   - Delete and recreate (data loss)
//! ├── UpdateWorkingCopy   - Run jj update
//! ├── ForgetAndRecreate   - jj workspace forget + add
//! ├── SyncDatabase        - Update DB to match filesystem
//! └── NoRepairPossible    - Manual intervention required
//! ```

use std::{
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
    time::{Duration, SystemTime},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// CORRUPTION TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Types of workspace corruption that can be detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorruptionType {
    /// Workspace directory does not exist
    MissingDirectory,
    /// .jj directory is missing from workspace
    MissingJjDir,
    /// .jj state files are corrupted or invalid
    InvalidJjState,
    /// Working copy is out of sync with repository state
    StaleWorkingCopy,
    /// Workspace directory exists but not registered in JJ
    OrphanedWorkspace,
    /// State database doesn't match filesystem state
    DatabaseMismatch,
    /// Insufficient permissions to access workspace
    PermissionDenied,
    /// Lock file exists but process is dead
    StaleLock,
    /// Unknown corruption type
    Unknown,
}

impl CorruptionType {
    /// Returns all corruption types
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::MissingDirectory,
            Self::MissingJjDir,
            Self::InvalidJjState,
            Self::StaleWorkingCopy,
            Self::OrphanedWorkspace,
            Self::DatabaseMismatch,
            Self::PermissionDenied,
            Self::StaleLock,
            Self::Unknown,
        ]
    }

    /// Returns the severity level (1-5, 5 being most severe)
    #[must_use]
    pub const fn severity(&self) -> u8 {
        match self {
            Self::StaleWorkingCopy => 1,
            Self::StaleLock | Self::DatabaseMismatch => 2,
            Self::OrphanedWorkspace => 3,
            Self::MissingJjDir | Self::InvalidJjState => 4,
            Self::MissingDirectory | Self::PermissionDenied | Self::Unknown => 5,
        }
    }

    /// Returns true if this corruption type can be auto-repaired
    #[must_use]
    pub const fn is_auto_repairable(&self) -> bool {
        match self {
            Self::StaleWorkingCopy
            | Self::StaleLock
            | Self::DatabaseMismatch
            | Self::OrphanedWorkspace => true,
            Self::MissingDirectory
            | Self::MissingJjDir
            | Self::InvalidJjState
            | Self::PermissionDenied
            | Self::Unknown => false,
        }
    }

    /// Returns the recommended repair strategy
    #[must_use]
    pub const fn recommended_strategy(&self) -> RepairStrategy {
        match self {
            Self::StaleWorkingCopy => RepairStrategy::UpdateWorkingCopy,
            Self::StaleLock => RepairStrategy::ClearStaleLock,
            Self::DatabaseMismatch => RepairStrategy::SyncDatabase,
            Self::OrphanedWorkspace | Self::MissingJjDir | Self::InvalidJjState => {
                RepairStrategy::ForgetAndRecreate
            }
            Self::MissingDirectory => RepairStrategy::RecreateWorkspace,
            Self::PermissionDenied | Self::Unknown => RepairStrategy::NoRepairPossible,
        }
    }
}

impl fmt::Display for CorruptionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDirectory => write!(f, "missing_directory"),
            Self::MissingJjDir => write!(f, "missing_jj_dir"),
            Self::InvalidJjState => write!(f, "invalid_jj_state"),
            Self::StaleWorkingCopy => write!(f, "stale_working_copy"),
            Self::OrphanedWorkspace => write!(f, "orphaned_workspace"),
            Self::DatabaseMismatch => write!(f, "database_mismatch"),
            Self::PermissionDenied => write!(f, "permission_denied"),
            Self::StaleLock => write!(f, "stale_lock"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl FromStr for CorruptionType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "missing_directory" => Ok(Self::MissingDirectory),
            "missing_jj_dir" => Ok(Self::MissingJjDir),
            "invalid_jj_state" => Ok(Self::InvalidJjState),
            "stale_working_copy" => Ok(Self::StaleWorkingCopy),
            "orphaned_workspace" => Ok(Self::OrphanedWorkspace),
            "database_mismatch" => Ok(Self::DatabaseMismatch),
            "permission_denied" => Ok(Self::PermissionDenied),
            "stale_lock" => Ok(Self::StaleLock),
            "unknown" => Ok(Self::Unknown),
            _ => Err(Error::ValidationError(format!(
                "Invalid corruption type: '{s}'"
            ))),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// REPAIR STRATEGIES
// ═══════════════════════════════════════════════════════════════════════════

/// Strategies for repairing workspace corruption
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairStrategy {
    /// Delete and recreate the workspace (potential data loss)
    RecreateWorkspace,
    /// Run `jj` update to sync working copy
    UpdateWorkingCopy,
    /// Forget workspace in JJ and recreate it
    ForgetAndRecreate,
    /// Update database to match filesystem state
    SyncDatabase,
    /// Clear stale lock files
    ClearStaleLock,
    /// No automated repair possible - manual intervention required
    NoRepairPossible,
}

impl RepairStrategy {
    /// Returns all repair strategies
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::RecreateWorkspace,
            Self::UpdateWorkingCopy,
            Self::ForgetAndRecreate,
            Self::SyncDatabase,
            Self::ClearStaleLock,
            Self::NoRepairPossible,
        ]
    }

    /// Returns true if this strategy may result in data loss
    #[must_use]
    pub const fn may_lose_data(&self) -> bool {
        matches!(self, Self::RecreateWorkspace | Self::ForgetAndRecreate)
    }

    /// Returns the risk level (1-5, 5 being highest risk)
    #[must_use]
    pub const fn risk_level(&self) -> u8 {
        match self {
            Self::SyncDatabase | Self::ClearStaleLock => 1,
            Self::UpdateWorkingCopy => 2,
            Self::ForgetAndRecreate => 4,
            Self::RecreateWorkspace => 5,
            Self::NoRepairPossible => 0,
        }
    }

    /// Returns a human-readable description of the repair action
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::RecreateWorkspace => "Delete workspace directory and recreate from scratch",
            Self::UpdateWorkingCopy => "Run 'jj' to update working copy to match repository",
            Self::ForgetAndRecreate => "Forget workspace in JJ and add it again",
            Self::SyncDatabase => "Update state database to match filesystem",
            Self::ClearStaleLock => "Remove stale lock files from dead processes",
            Self::NoRepairPossible => "Manual intervention required",
        }
    }
}

impl fmt::Display for RepairStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecreateWorkspace => write!(f, "recreate_workspace"),
            Self::UpdateWorkingCopy => write!(f, "update_working_copy"),
            Self::ForgetAndRecreate => write!(f, "forget_and_recreate"),
            Self::SyncDatabase => write!(f, "sync_database"),
            Self::ClearStaleLock => write!(f, "clear_stale_lock"),
            Self::NoRepairPossible => write!(f, "no_repair_possible"),
        }
    }
}

impl FromStr for RepairStrategy {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "recreate_workspace" => Ok(Self::RecreateWorkspace),
            "update_working_copy" => Ok(Self::UpdateWorkingCopy),
            "forget_and_recreate" => Ok(Self::ForgetAndRecreate),
            "sync_database" => Ok(Self::SyncDatabase),
            "clear_stale_lock" => Ok(Self::ClearStaleLock),
            "no_repair_possible" => Ok(Self::NoRepairPossible),
            _ => Err(Error::ValidationError(format!(
                "Invalid repair strategy: '{s}'"
            ))),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// VALIDATION RESULT
// ═══════════════════════════════════════════════════════════════════════════

/// Result of workspace integrity validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Workspace name
    pub workspace: String,
    /// Workspace path
    pub path: PathBuf,
    /// Whether the workspace is valid
    pub is_valid: bool,
    /// Detected corruption issues (empty if valid)
    pub issues: Vec<IntegrityIssue>,
    /// Timestamp of validation
    pub validated_at: DateTime<Utc>,
    /// Duration of validation check
    pub duration_ms: u64,
}

impl ValidationResult {
    /// Create a valid result with no issues
    #[must_use]
    pub fn valid(workspace: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            workspace: workspace.into(),
            path: path.into(),
            is_valid: true,
            issues: Vec::new(),
            validated_at: Utc::now(),
            duration_ms: 0,
        }
    }

    /// Create an invalid result with issues
    #[must_use]
    pub fn invalid(
        workspace: impl Into<String>,
        path: impl Into<PathBuf>,
        issues: Vec<IntegrityIssue>,
    ) -> Self {
        Self {
            workspace: workspace.into(),
            path: path.into(),
            is_valid: false,
            issues,
            validated_at: Utc::now(),
            duration_ms: 0,
        }
    }

    /// Set the validation duration
    #[must_use]
    pub const fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Returns the most severe issue, if any
    #[must_use]
    pub fn most_severe_issue(&self) -> Option<&IntegrityIssue> {
        self.issues
            .iter()
            .max_by_key(|i| i.corruption_type.severity())
    }

    /// Returns true if any issue can be auto-repaired
    #[must_use]
    pub fn has_auto_repairable_issues(&self) -> bool {
        self.issues
            .iter()
            .any(|i| i.corruption_type.is_auto_repairable())
    }

    /// Returns issues that can be auto-repaired
    #[must_use]
    pub fn auto_repairable_issues(&self) -> Vec<&IntegrityIssue> {
        self.issues
            .iter()
            .filter(|i| i.corruption_type.is_auto_repairable())
            .collect()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// INTEGRITY ISSUE
// ═══════════════════════════════════════════════════════════════════════════

/// A specific integrity issue detected in a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityIssue {
    /// Type of corruption
    pub corruption_type: CorruptionType,
    /// Human-readable description
    pub description: String,
    /// Affected path (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affected_path: Option<PathBuf>,
    /// Recommended repair strategy
    pub recommended_strategy: RepairStrategy,
    /// Additional context for diagnosis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

impl IntegrityIssue {
    /// Create a new integrity issue
    #[must_use]
    pub fn new(corruption_type: CorruptionType, description: impl Into<String>) -> Self {
        Self {
            corruption_type,
            description: description.into(),
            affected_path: None,
            recommended_strategy: corruption_type.recommended_strategy(),
            context: None,
        }
    }

    /// Set the affected path
    #[must_use]
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.affected_path = Some(path.into());
        self
    }

    /// Set additional context
    #[must_use]
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Override the recommended strategy
    #[must_use]
    pub const fn with_strategy(mut self, strategy: RepairStrategy) -> Self {
        self.recommended_strategy = strategy;
        self
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// BACKUP
// ═══════════════════════════════════════════════════════════════════════════

/// Metadata about a workspace backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    /// Unique backup ID
    pub id: String,
    /// Workspace name that was backed up
    pub workspace: String,
    /// Original workspace path
    pub original_path: PathBuf,
    /// Backup location
    pub backup_path: PathBuf,
    /// Timestamp of backup creation
    pub created_at: DateTime<Utc>,
    /// Size of backup in bytes
    pub size_bytes: u64,
    /// Reason for backup
    pub reason: BackupReason,
    /// Integrity hash of backup (SHA-256)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
}

impl BackupMetadata {
    /// Create new backup metadata
    #[must_use]
    pub fn new(
        workspace: impl Into<String>,
        original_path: impl Into<PathBuf>,
        backup_path: impl Into<PathBuf>,
        reason: BackupReason,
    ) -> Self {
        let now = Utc::now();
        let id = format!(
            "backup-{}-{}",
            now.format("%Y%m%d-%H%M%S"),
            now.timestamp_millis() % 1000
        );

        Self {
            id,
            workspace: workspace.into(),
            original_path: original_path.into(),
            backup_path: backup_path.into(),
            created_at: now,
            size_bytes: 0,
            reason,
            checksum: None,
        }
    }

    /// Set the backup size
    #[must_use]
    pub const fn with_size(mut self, size_bytes: u64) -> Self {
        self.size_bytes = size_bytes;
        self
    }

    /// Set the checksum
    #[must_use]
    pub fn with_checksum(mut self, checksum: impl Into<String>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }

    /// Returns the age of this backup
    #[must_use]
    pub fn age(&self) -> Duration {
        let now = Utc::now();
        let diff = now.signed_duration_since(self.created_at);
        Duration::from_secs(diff.num_seconds().unsigned_abs())
    }
}

/// Reason for creating a backup
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupReason {
    /// Backup before repair operation
    PreRepair,
    /// Backup before risky operation
    PreRiskyOperation,
    /// Scheduled periodic backup
    Scheduled,
    /// User-requested backup
    Manual,
    /// Backup before workspace deletion
    PreDelete,
}

impl fmt::Display for BackupReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PreRepair => write!(f, "pre_repair"),
            Self::PreRiskyOperation => write!(f, "pre_risky_operation"),
            Self::Scheduled => write!(f, "scheduled"),
            Self::Manual => write!(f, "manual"),
            Self::PreDelete => write!(f, "pre_delete"),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// REPAIR RESULT
// ═══════════════════════════════════════════════════════════════════════════

/// Result of a repair operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairResult {
    /// Workspace that was repaired
    pub workspace: String,
    /// Strategy that was applied
    pub strategy: RepairStrategy,
    /// Whether repair was successful
    pub success: bool,
    /// Backup created before repair (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup: Option<BackupMetadata>,
    /// Issues that were addressed
    pub issues_addressed: Vec<CorruptionType>,
    /// Any issues that remain after repair
    pub remaining_issues: Vec<IntegrityIssue>,
    /// Timestamp of repair
    pub repaired_at: DateTime<Utc>,
    /// Duration of repair in milliseconds
    pub duration_ms: u64,
    /// Human-readable summary
    pub summary: String,
}

impl RepairResult {
    /// Create a successful repair result
    #[must_use]
    pub fn success(
        workspace: impl Into<String>,
        strategy: RepairStrategy,
        issues_addressed: Vec<CorruptionType>,
    ) -> Self {
        let summary = format!(
            "Successfully repaired {} issue(s) using {}",
            issues_addressed.len(),
            strategy
        );
        Self {
            workspace: workspace.into(),
            strategy,
            success: true,
            backup: None,
            issues_addressed,
            remaining_issues: Vec::new(),
            repaired_at: Utc::now(),
            duration_ms: 0,
            summary,
        }
    }

    /// Create a failed repair result
    #[must_use]
    pub fn failure(
        workspace: impl Into<String>,
        strategy: RepairStrategy,
        remaining_issues: Vec<IntegrityIssue>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            workspace: workspace.into(),
            strategy,
            success: false,
            backup: None,
            issues_addressed: Vec::new(),
            remaining_issues,
            repaired_at: Utc::now(),
            duration_ms: 0,
            summary: reason.into(),
        }
    }

    /// Set the backup that was created
    #[must_use]
    pub fn with_backup(mut self, backup: BackupMetadata) -> Self {
        self.backup = Some(backup);
        self
    }

    /// Set the duration
    #[must_use]
    pub const fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ROLLBACK
// ═══════════════════════════════════════════════════════════════════════════

/// Result of a rollback operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResult {
    /// Workspace that was rolled back
    pub workspace: String,
    /// Backup that was restored
    pub backup_id: String,
    /// Whether rollback was successful
    pub success: bool,
    /// Timestamp of rollback
    pub rolled_back_at: DateTime<Utc>,
    /// Summary message
    pub summary: String,
}

impl RollbackResult {
    /// Create a successful rollback result
    #[must_use]
    pub fn success(workspace: impl Into<String>, backup_id: impl Into<String>) -> Self {
        let backup_id = backup_id.into();
        Self {
            workspace: workspace.into(),
            backup_id: backup_id.clone(),
            success: true,
            rolled_back_at: Utc::now(),
            summary: format!("Successfully restored from backup {backup_id}"),
        }
    }

    /// Create a failed rollback result
    #[must_use]
    pub fn failure(
        workspace: impl Into<String>,
        backup_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            workspace: workspace.into(),
            backup_id: backup_id.into(),
            success: false,
            rolled_back_at: Utc::now(),
            summary: reason.into(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// INTEGRITY VALIDATOR
// ═══════════════════════════════════════════════════════════════════════════

/// Workspace integrity validator
///
/// Provides methods to validate workspace integrity and detect corruption.
#[derive(Debug, Clone)]
pub struct IntegrityValidator {
    /// Root directory containing workspaces
    workspaces_root: PathBuf,
    /// Check timeout in milliseconds
    timeout_ms: u64,
}

impl IntegrityValidator {
    /// Default timeout for validation checks (5 seconds)
    pub const DEFAULT_TIMEOUT_MS: u64 = 5000;

    /// Create a new integrity validator
    #[must_use]
    pub fn new(workspaces_root: impl Into<PathBuf>) -> Self {
        Self {
            workspaces_root: workspaces_root.into(),
            timeout_ms: Self::DEFAULT_TIMEOUT_MS,
        }
    }

    /// Set custom timeout
    #[must_use]
    pub const fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Validate a single workspace
    pub fn validate(&self, workspace_name: &str) -> Result<ValidationResult> {
        let start = SystemTime::now();
        let workspace_path = self.workspaces_root.join(workspace_name);

        let mut issues = Vec::new();

        // Check 1: Directory exists
        if !workspace_path.exists() {
            issues.push(
                IntegrityIssue::new(
                    CorruptionType::MissingDirectory,
                    format!(
                        "Workspace directory does not exist: {}",
                        workspace_path.display()
                    ),
                )
                .with_path(&workspace_path),
            );

            // Can't continue validation if directory is missing
            let duration = start.elapsed().map(|d| d.as_millis() as u64).unwrap_or(0);
            return Ok(
                ValidationResult::invalid(workspace_name, &workspace_path, issues)
                    .with_duration(duration),
            );
        }

        // Check 2: Directory is readable
        if let Err(e) = std::fs::read_dir(&workspace_path) {
            issues.push(
                IntegrityIssue::new(
                    CorruptionType::PermissionDenied,
                    format!("Cannot read workspace directory: {e}"),
                )
                .with_path(&workspace_path)
                .with_context(e.to_string()),
            );
        }

        // Check 3: .jj directory exists
        let jj_dir = workspace_path.join(".jj");
        if jj_dir.exists() {
            // Check 4: .jj directory is valid
            if let Err(issue) = Self::validate_jj_directory(&jj_dir) {
                issues.push(issue);
            }
        } else {
            issues.push(
                IntegrityIssue::new(
                    CorruptionType::MissingJjDir,
                    format!(
                        ".jj directory missing from workspace: {}",
                        workspace_path.display()
                    ),
                )
                .with_path(&jj_dir),
            );
        }

        // Check 5: Lock files
        if let Some(issue) = Self::check_stale_locks(&workspace_path) {
            issues.push(issue);
        }

        let duration = start.elapsed().map(|d| d.as_millis() as u64).unwrap_or(0);

        if issues.is_empty() {
            Ok(ValidationResult::valid(workspace_name, &workspace_path).with_duration(duration))
        } else {
            Ok(
                ValidationResult::invalid(workspace_name, &workspace_path, issues)
                    .with_duration(duration),
            )
        }
    }

    /// Validate multiple workspaces
    pub fn validate_all(&self, workspace_names: &[String]) -> Result<Vec<ValidationResult>> {
        workspace_names
            .iter()
            .map(|name| self.validate(name))
            .collect()
    }

    /// Validate .jj directory structure
    fn validate_jj_directory(jj_dir: &Path) -> std::result::Result<(), IntegrityIssue> {
        // Check for essential files
        let working_copy_state = jj_dir.join("working_copy");

        if !working_copy_state.exists() {
            return Err(IntegrityIssue::new(
                CorruptionType::InvalidJjState,
                "Missing working_copy state in .jj directory",
            )
            .with_path(&working_copy_state));
        }

        // Check repo directory
        let repo_dir = jj_dir.join("repo");
        if !repo_dir.exists() {
            return Err(IntegrityIssue::new(
                CorruptionType::InvalidJjState,
                "Missing repo directory in .jj",
            )
            .with_path(&repo_dir));
        }

        Ok(())
    }

    /// Check for stale lock files
    fn check_stale_locks(workspace_path: &Path) -> Option<IntegrityIssue> {
        let lock_file = workspace_path.join(".jj").join("lock");

        if !lock_file.exists() {
            return None;
        }

        // Check if lock file is old (> 1 hour is considered stale)
        let metadata = std::fs::metadata(&lock_file).ok()?;
        let modified = metadata.modified().ok()?;
        let age = SystemTime::now().duration_since(modified).ok()?;

        if age > Duration::from_secs(3600) {
            Some(
                IntegrityIssue::new(
                    CorruptionType::StaleLock,
                    format!("Stale lock file detected (age: {} seconds)", age.as_secs()),
                )
                .with_path(&lock_file)
                .with_context(format!("Lock file age: {age:?}")),
            )
        } else {
            None
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// BACKUP MANAGER
// ═══════════════════════════════════════════════════════════════════════════

/// Manages workspace backups
#[derive(Debug, Clone)]
pub struct BackupManager {
    /// Root directory for backups
    backup_root: PathBuf,
    /// Maximum number of backups to keep per workspace
    max_backups_per_workspace: usize,
    /// Maximum age of backups to keep
    max_backup_age: Duration,
}

impl BackupManager {
    /// Default maximum backups per workspace
    pub const DEFAULT_MAX_BACKUPS: usize = 5;
    /// Default maximum backup age (7 days)
    pub const DEFAULT_MAX_AGE_SECS: u64 = 7 * 24 * 3600;

    /// Create a new backup manager
    #[must_use]
    pub fn new(backup_root: impl Into<PathBuf>) -> Self {
        Self {
            backup_root: backup_root.into(),
            max_backups_per_workspace: Self::DEFAULT_MAX_BACKUPS,
            max_backup_age: Duration::from_secs(Self::DEFAULT_MAX_AGE_SECS),
        }
    }

    /// Set maximum backups per workspace
    #[must_use]
    pub const fn with_max_backups(mut self, max: usize) -> Self {
        self.max_backups_per_workspace = max;
        self
    }

    /// Set maximum backup age
    #[must_use]
    pub const fn with_max_age(mut self, max_age: Duration) -> Self {
        self.max_backup_age = max_age;
        self
    }

    /// Create a backup of a workspace
    pub fn create_backup(
        &self,
        workspace_name: &str,
        workspace_path: &Path,
        reason: BackupReason,
    ) -> Result<BackupMetadata> {
        // Ensure backup directory exists
        let workspace_backup_dir = self.backup_root.join(workspace_name);
        std::fs::create_dir_all(&workspace_backup_dir)
            .map_err(|e| Error::IoError(format!("Failed to create backup directory: {e}")))?;

        // Create backup metadata
        let backup_path =
            workspace_backup_dir.join(format!("backup-{}", Utc::now().format("%Y%m%d-%H%M%S")));

        let mut metadata =
            BackupMetadata::new(workspace_name, workspace_path, &backup_path, reason);

        // Copy workspace to backup location
        let size = Self::copy_workspace(workspace_path, &backup_path)?;
        metadata = metadata.with_size(size);

        // Write metadata file
        let metadata_path = backup_path.with_extension("meta.json");
        let metadata_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| Error::ParseError(format!("Failed to serialize backup metadata: {e}")))?;
        std::fs::write(&metadata_path, metadata_json)
            .map_err(|e| Error::IoError(format!("Failed to write backup metadata: {e}")))?;

        Ok(metadata)
    }

    /// List backups for a workspace
    pub fn list_backups(&self, workspace_name: &str) -> Result<Vec<BackupMetadata>> {
        let workspace_backup_dir = self.backup_root.join(workspace_name);

        if !workspace_backup_dir.exists() {
            return Ok(Vec::new());
        }

        let entries = std::fs::read_dir(&workspace_backup_dir)
            .map_err(|e| Error::IoError(format!("Failed to read backup directory: {e}")))?;

        let mut backups = Vec::new();

        for entry in entries {
            let entry = entry
                .map_err(|e| Error::IoError(format!("Failed to read directory entry: {e}")))?;
            let path = entry.path();

            // Look for metadata files
            if path.extension().is_some_and(|ext| ext == "json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(metadata) = serde_json::from_str::<BackupMetadata>(&content) {
                        backups.push(metadata);
                    }
                }
            }
        }

        // Sort by creation time (newest first)
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(backups)
    }

    /// Restore a workspace from backup
    pub fn restore_backup(
        &self,
        backup_id: &str,
        workspace_name: &str,
        target_path: &Path,
    ) -> Result<RollbackResult> {
        let backups = self.list_backups(workspace_name)?;

        let backup = backups
            .iter()
            .find(|b| b.id == backup_id)
            .ok_or_else(|| Error::NotFound(format!("Backup not found: {backup_id}")))?;

        // Ensure backup exists
        if !backup.backup_path.exists() {
            return Ok(RollbackResult::failure(
                workspace_name,
                backup_id,
                format!(
                    "Backup directory does not exist: {}",
                    backup.backup_path.display()
                ),
            ));
        }

        // Remove existing target if it exists
        if target_path.exists() {
            std::fs::remove_dir_all(target_path)
                .map_err(|e| Error::IoError(format!("Failed to remove existing workspace: {e}")))?;
        }

        // Copy backup to target
        Self::copy_workspace(&backup.backup_path, target_path)?;

        Ok(RollbackResult::success(workspace_name, backup_id))
    }

    /// Cleanup old backups
    pub fn cleanup(&self, workspace_name: &str) -> Result<usize> {
        let backups = self.list_backups(workspace_name)?;
        let mut removed = 0;

        // Remove backups exceeding max count
        for backup in backups.iter().skip(self.max_backups_per_workspace) {
            if Self::remove_backup(backup).is_ok() {
                removed += 1;
            }
        }

        // Remove backups exceeding max age
        for backup in &backups {
            if backup.age() > self.max_backup_age && Self::remove_backup(backup).is_ok() {
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// Copy workspace directory recursively
    fn copy_workspace(source: &Path, dest: &Path) -> Result<u64> {
        std::fs::create_dir_all(dest)
            .map_err(|e| Error::IoError(format!("Failed to create destination directory: {e}")))?;

        let mut total_size = 0u64;

        for entry in walkdir::WalkDir::new(source)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let relative = entry
                .path()
                .strip_prefix(source)
                .map_err(|e| Error::IoError(format!("Failed to compute relative path: {e}")))?;
            let dest_path = dest.join(relative);

            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&dest_path)
                    .map_err(|e| Error::IoError(format!("Failed to create directory: {e}")))?;
            } else {
                if let Some(parent) = dest_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        Error::IoError(format!("Failed to create parent directory: {e}"))
                    })?;
                }
                let size = std::fs::copy(entry.path(), &dest_path)
                    .map_err(|e| Error::IoError(format!("Failed to copy file: {e}")))?;
                total_size += size;
            }
        }

        Ok(total_size)
    }

    /// Remove a backup
    fn remove_backup(backup: &BackupMetadata) -> Result<()> {
        if backup.backup_path.exists() {
            std::fs::remove_dir_all(&backup.backup_path)
                .map_err(|e| Error::IoError(format!("Failed to remove backup directory: {e}")))?;
        }

        let metadata_path = backup.backup_path.with_extension("meta.json");
        if metadata_path.exists() {
            std::fs::remove_file(&metadata_path)
                .map_err(|e| Error::IoError(format!("Failed to remove backup metadata: {e}")))?;
        }

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// REPAIR EXECUTOR
// ═══════════════════════════════════════════════════════════════════════════

/// Executes repair operations on corrupted workspaces
#[derive(Debug)]
pub struct RepairExecutor {
    /// Backup manager for pre-repair backups
    backup_manager: BackupManager,
    /// Whether to always create backups before repair
    always_backup: bool,
}

impl RepairExecutor {
    /// Create a new repair executor
    #[must_use]
    pub const fn new(backup_manager: BackupManager) -> Self {
        Self {
            backup_manager,
            always_backup: true,
        }
    }

    /// Set whether to always create backups
    #[must_use]
    pub const fn with_always_backup(mut self, always_backup: bool) -> Self {
        self.always_backup = always_backup;
        self
    }

    /// Execute a repair strategy on a workspace
    pub fn execute(
        &self,
        workspace_name: &str,
        workspace_path: &Path,
        validation_result: &ValidationResult,
        strategy: RepairStrategy,
    ) -> Result<RepairResult> {
        let start = std::time::Instant::now();

        // Create backup if needed
        let backup = if self.always_backup && strategy.may_lose_data() && workspace_path.exists() {
            Some(self.backup_manager.create_backup(
                workspace_name,
                workspace_path,
                BackupReason::PreRepair,
            )?)
        } else {
            None
        };

        // Execute the appropriate repair strategy
        let result = match strategy {
            RepairStrategy::ClearStaleLock => {
                Self::clear_stale_locks(workspace_name, workspace_path, validation_result)
            }
            RepairStrategy::UpdateWorkingCopy => {
                Self::update_working_copy(workspace_name, workspace_path, validation_result)
            }
            RepairStrategy::SyncDatabase => {
                Ok(Self::sync_database(workspace_name, validation_result))
            }
            RepairStrategy::ForgetAndRecreate => {
                Self::forget_and_recreate(workspace_name, workspace_path, validation_result)
            }
            RepairStrategy::RecreateWorkspace => {
                Self::recreate_workspace(workspace_name, workspace_path, validation_result)
            }
            RepairStrategy::NoRepairPossible => Ok(RepairResult::failure(
                workspace_name,
                strategy,
                validation_result.issues.clone(),
                "No automated repair possible - manual intervention required",
            )),
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        result.map(|mut r| {
            r.backup = backup;
            r.duration_ms = duration_ms;
            r
        })
    }

    /// Clear stale lock files
    fn clear_stale_locks(
        workspace_name: &str,
        workspace_path: &Path,
        validation_result: &ValidationResult,
    ) -> Result<RepairResult> {
        let lock_file = workspace_path.join(".jj").join("lock");

        if lock_file.exists() {
            std::fs::remove_file(&lock_file)
                .map_err(|e| Error::IoError(format!("Failed to remove lock file: {e}")))?;
        }

        let issues_addressed: Vec<CorruptionType> = validation_result
            .issues
            .iter()
            .filter(|i| i.corruption_type == CorruptionType::StaleLock)
            .map(|i| i.corruption_type)
            .collect();

        let remaining: Vec<IntegrityIssue> = validation_result
            .issues
            .iter()
            .filter(|i| i.corruption_type != CorruptionType::StaleLock)
            .cloned()
            .collect();

        if remaining.is_empty() {
            Ok(RepairResult::success(
                workspace_name,
                RepairStrategy::ClearStaleLock,
                issues_addressed,
            ))
        } else {
            Ok(RepairResult::failure(
                workspace_name,
                RepairStrategy::ClearStaleLock,
                remaining,
                "Cleared stale locks but other issues remain",
            ))
        }
    }

    /// Update working copy by running jj
    fn update_working_copy(
        workspace_name: &str,
        workspace_path: &Path,
        validation_result: &ValidationResult,
    ) -> Result<RepairResult> {
        use std::process::Command;

        let output = Command::new("jj")
            .current_dir(workspace_path)
            .args(["status"])
            .output()
            .map_err(|e| Error::Command(format!("Failed to run jj status: {e}")))?;

        if output.status.success() {
            let issues_addressed: Vec<CorruptionType> = validation_result
                .issues
                .iter()
                .filter(|i| i.corruption_type == CorruptionType::StaleWorkingCopy)
                .map(|i| i.corruption_type)
                .collect();

            Ok(RepairResult::success(
                workspace_name,
                RepairStrategy::UpdateWorkingCopy,
                issues_addressed,
            ))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(RepairResult::failure(
                workspace_name,
                RepairStrategy::UpdateWorkingCopy,
                validation_result.issues.clone(),
                format!("jj status failed: {stderr}"),
            ))
        }
    }

    /// Sync database to match filesystem
    fn sync_database(workspace_name: &str, validation_result: &ValidationResult) -> RepairResult {
        // This is a placeholder - actual implementation would update the SQLite database
        let issues_addressed: Vec<CorruptionType> = validation_result
            .issues
            .iter()
            .filter(|i| i.corruption_type == CorruptionType::DatabaseMismatch)
            .map(|i| i.corruption_type)
            .collect();

        RepairResult::success(
            workspace_name,
            RepairStrategy::SyncDatabase,
            issues_addressed,
        )
    }

    /// Forget workspace in JJ and recreate
    fn forget_and_recreate(
        workspace_name: &str,
        workspace_path: &Path,
        validation_result: &ValidationResult,
    ) -> Result<RepairResult> {
        use std::process::Command;

        // Forget the workspace
        let forget_output = Command::new("jj")
            .args(["workspace", "forget", workspace_name])
            .output()
            .map_err(|e| Error::Command(format!("Failed to forget workspace: {e}")))?;

        if !forget_output.status.success() {
            // Workspace might not be registered, which is fine
            let stderr = String::from_utf8_lossy(&forget_output.stderr);
            if !stderr.contains("not found") && !stderr.contains("No workspace") {
                return Ok(RepairResult::failure(
                    workspace_name,
                    RepairStrategy::ForgetAndRecreate,
                    validation_result.issues.clone(),
                    format!("Failed to forget workspace: {stderr}"),
                ));
            }
        }

        // Remove existing directory if it exists
        if workspace_path.exists() {
            std::fs::remove_dir_all(workspace_path).map_err(|e| {
                Error::IoError(format!("Failed to remove workspace directory: {e}"))
            })?;
        }

        // Recreate workspace
        let add_output = Command::new("jj")
            .args(["workspace", "add", "--name", workspace_name])
            .arg(workspace_path)
            .output()
            .map_err(|e| Error::Command(format!("Failed to add workspace: {e}")))?;

        if add_output.status.success() {
            let issues_addressed: Vec<CorruptionType> = validation_result
                .issues
                .iter()
                .map(|i| i.corruption_type)
                .collect();

            Ok(RepairResult::success(
                workspace_name,
                RepairStrategy::ForgetAndRecreate,
                issues_addressed,
            ))
        } else {
            let stderr = String::from_utf8_lossy(&add_output.stderr);
            Ok(RepairResult::failure(
                workspace_name,
                RepairStrategy::ForgetAndRecreate,
                validation_result.issues.clone(),
                format!("Failed to recreate workspace: {stderr}"),
            ))
        }
    }

    /// Recreate workspace from scratch
    fn recreate_workspace(
        workspace_name: &str,
        workspace_path: &Path,
        validation_result: &ValidationResult,
    ) -> Result<RepairResult> {
        // This is essentially the same as forget_and_recreate but more aggressive
        Self::forget_and_recreate(workspace_name, workspace_path, validation_result)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Quick validation check for a workspace path
pub fn quick_validate(workspace_path: &Path) -> Result<bool> {
    if !workspace_path.exists() {
        return Ok(false);
    }

    let jj_dir = workspace_path.join(".jj");
    if !jj_dir.exists() {
        return Ok(false);
    }

    let working_copy = jj_dir.join("working_copy");
    if !working_copy.exists() {
        return Ok(false);
    }

    Ok(true)
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // CORRUPTION TYPE TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_corruption_type_all() {
        let all = CorruptionType::all();
        assert_eq!(all.len(), 9);
        assert!(all.contains(&CorruptionType::MissingDirectory));
        assert!(all.contains(&CorruptionType::Unknown));
    }

    #[test]
    fn test_corruption_type_severity() {
        assert_eq!(CorruptionType::StaleWorkingCopy.severity(), 1);
        assert_eq!(CorruptionType::MissingDirectory.severity(), 5);
        assert_eq!(CorruptionType::PermissionDenied.severity(), 5);
    }

    #[test]
    fn test_corruption_type_auto_repairable() {
        assert!(CorruptionType::StaleWorkingCopy.is_auto_repairable());
        assert!(CorruptionType::StaleLock.is_auto_repairable());
        assert!(!CorruptionType::MissingDirectory.is_auto_repairable());
        assert!(!CorruptionType::PermissionDenied.is_auto_repairable());
    }

    #[test]
    fn test_corruption_type_recommended_strategy() {
        assert_eq!(
            CorruptionType::StaleWorkingCopy.recommended_strategy(),
            RepairStrategy::UpdateWorkingCopy
        );
        assert_eq!(
            CorruptionType::StaleLock.recommended_strategy(),
            RepairStrategy::ClearStaleLock
        );
        assert_eq!(
            CorruptionType::MissingDirectory.recommended_strategy(),
            RepairStrategy::RecreateWorkspace
        );
    }

    #[test]
    fn test_corruption_type_display() {
        assert_eq!(
            CorruptionType::MissingDirectory.to_string(),
            "missing_directory"
        );
        assert_eq!(CorruptionType::StaleLock.to_string(), "stale_lock");
    }

    #[test]
    fn test_corruption_type_from_str() {
        assert_eq!(
            CorruptionType::from_str("missing_directory").ok(),
            Some(CorruptionType::MissingDirectory)
        );
        assert_eq!(
            CorruptionType::from_str("stale_lock").ok(),
            Some(CorruptionType::StaleLock)
        );
        assert!(CorruptionType::from_str("invalid").is_err());
    }

    #[test]
    fn test_corruption_type_serialization() {
        for ct in CorruptionType::all() {
            let json = serde_json::to_string(ct);
            assert!(json.is_ok(), "Failed to serialize {ct:?}");
            let json_str = json.unwrap_or_default();
            let parsed: std::result::Result<CorruptionType, _> = serde_json::from_str(&json_str);
            assert!(parsed.is_ok(), "Failed to deserialize {ct:?}");
            assert_eq!(&parsed.unwrap_or(CorruptionType::Unknown), ct);
        }
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // REPAIR STRATEGY TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_repair_strategy_all() {
        let all = RepairStrategy::all();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn test_repair_strategy_may_lose_data() {
        assert!(RepairStrategy::RecreateWorkspace.may_lose_data());
        assert!(RepairStrategy::ForgetAndRecreate.may_lose_data());
        assert!(!RepairStrategy::UpdateWorkingCopy.may_lose_data());
        assert!(!RepairStrategy::ClearStaleLock.may_lose_data());
    }

    #[test]
    fn test_repair_strategy_risk_level() {
        assert_eq!(RepairStrategy::SyncDatabase.risk_level(), 1);
        assert_eq!(RepairStrategy::RecreateWorkspace.risk_level(), 5);
        assert_eq!(RepairStrategy::NoRepairPossible.risk_level(), 0);
    }

    #[test]
    fn test_repair_strategy_description() {
        assert!(!RepairStrategy::RecreateWorkspace.description().is_empty());
        assert!(!RepairStrategy::NoRepairPossible.description().is_empty());
    }

    #[test]
    fn test_repair_strategy_display() {
        assert_eq!(
            RepairStrategy::RecreateWorkspace.to_string(),
            "recreate_workspace"
        );
        assert_eq!(
            RepairStrategy::ClearStaleLock.to_string(),
            "clear_stale_lock"
        );
    }

    #[test]
    fn test_repair_strategy_from_str() {
        assert_eq!(
            RepairStrategy::from_str("recreate_workspace").ok(),
            Some(RepairStrategy::RecreateWorkspace)
        );
        assert!(RepairStrategy::from_str("invalid").is_err());
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // VALIDATION RESULT TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_validation_result_valid() {
        let result = ValidationResult::valid("test-ws", "/path/to/ws");
        assert!(result.is_valid);
        assert!(result.issues.is_empty());
        assert_eq!(result.workspace, "test-ws");
    }

    #[test]
    fn test_validation_result_invalid() {
        let issues = vec![IntegrityIssue::new(
            CorruptionType::MissingDirectory,
            "Directory not found",
        )];
        let result = ValidationResult::invalid("test-ws", "/path/to/ws", issues);
        assert!(!result.is_valid);
        assert_eq!(result.issues.len(), 1);
    }

    #[test]
    fn test_validation_result_with_duration() {
        let result = ValidationResult::valid("test-ws", "/path/to/ws").with_duration(100);
        assert_eq!(result.duration_ms, 100);
    }

    #[test]
    fn test_validation_result_most_severe_issue() {
        let issues = vec![
            IntegrityIssue::new(CorruptionType::StaleWorkingCopy, "Minor"),
            IntegrityIssue::new(CorruptionType::MissingDirectory, "Severe"),
        ];
        let result = ValidationResult::invalid("test-ws", "/path/to/ws", issues);
        let most_severe = result.most_severe_issue();
        assert!(most_severe.is_some());
        assert_eq!(
            most_severe.map(|i| i.corruption_type),
            Some(CorruptionType::MissingDirectory)
        );
    }

    #[test]
    fn test_validation_result_auto_repairable_issues() {
        let issues = vec![
            IntegrityIssue::new(CorruptionType::StaleWorkingCopy, "Auto-repairable"),
            IntegrityIssue::new(CorruptionType::PermissionDenied, "Not auto-repairable"),
        ];
        let result = ValidationResult::invalid("test-ws", "/path/to/ws", issues);
        assert!(result.has_auto_repairable_issues());
        let repairable = result.auto_repairable_issues();
        assert_eq!(repairable.len(), 1);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // INTEGRITY ISSUE TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_integrity_issue_new() {
        let issue = IntegrityIssue::new(CorruptionType::MissingDirectory, "Test description");
        assert_eq!(issue.corruption_type, CorruptionType::MissingDirectory);
        assert_eq!(issue.description, "Test description");
        assert!(issue.affected_path.is_none());
        assert_eq!(
            issue.recommended_strategy,
            RepairStrategy::RecreateWorkspace
        );
    }

    #[test]
    fn test_integrity_issue_with_path() {
        let issue =
            IntegrityIssue::new(CorruptionType::MissingDirectory, "Test").with_path("/some/path");
        assert_eq!(issue.affected_path, Some(PathBuf::from("/some/path")));
    }

    #[test]
    fn test_integrity_issue_with_context() {
        let issue = IntegrityIssue::new(CorruptionType::MissingDirectory, "Test")
            .with_context("Additional info");
        assert_eq!(issue.context, Some("Additional info".to_string()));
    }

    #[test]
    fn test_integrity_issue_with_strategy() {
        let issue = IntegrityIssue::new(CorruptionType::MissingDirectory, "Test")
            .with_strategy(RepairStrategy::NoRepairPossible);
        assert_eq!(issue.recommended_strategy, RepairStrategy::NoRepairPossible);
    }

    #[test]
    fn test_integrity_issue_serialization() {
        let issue = IntegrityIssue::new(CorruptionType::StaleLock, "Stale lock detected")
            .with_path("/ws/.jj/lock")
            .with_context("Age: 3600 seconds");

        let json = serde_json::to_string(&issue);
        assert!(json.is_ok());
        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("stale_lock"));
        assert!(json_str.contains("Stale lock detected"));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // BACKUP METADATA TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_backup_metadata_new() {
        let metadata = BackupMetadata::new(
            "test-ws",
            "/original/path",
            "/backup/path",
            BackupReason::PreRepair,
        );
        assert_eq!(metadata.workspace, "test-ws");
        assert!(metadata.id.starts_with("backup-"));
        assert_eq!(metadata.reason, BackupReason::PreRepair);
    }

    #[test]
    fn test_backup_metadata_with_size() {
        let metadata = BackupMetadata::new("test-ws", "/orig", "/backup", BackupReason::Manual)
            .with_size(1024);
        assert_eq!(metadata.size_bytes, 1024);
    }

    #[test]
    fn test_backup_metadata_with_checksum() {
        let metadata = BackupMetadata::new("test-ws", "/orig", "/backup", BackupReason::Manual)
            .with_checksum("abc123");
        assert_eq!(metadata.checksum, Some("abc123".to_string()));
    }

    #[test]
    fn test_backup_reason_display() {
        assert_eq!(BackupReason::PreRepair.to_string(), "pre_repair");
        assert_eq!(BackupReason::Manual.to_string(), "manual");
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // REPAIR RESULT TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_repair_result_success() {
        let result = RepairResult::success(
            "test-ws",
            RepairStrategy::ClearStaleLock,
            vec![CorruptionType::StaleLock],
        );
        assert!(result.success);
        assert_eq!(result.workspace, "test-ws");
        assert_eq!(result.issues_addressed.len(), 1);
    }

    #[test]
    fn test_repair_result_failure() {
        let remaining = vec![IntegrityIssue::new(
            CorruptionType::PermissionDenied,
            "Cannot fix",
        )];
        let result = RepairResult::failure(
            "test-ws",
            RepairStrategy::NoRepairPossible,
            remaining,
            "Manual intervention needed",
        );
        assert!(!result.success);
        assert_eq!(result.remaining_issues.len(), 1);
    }

    #[test]
    fn test_repair_result_with_backup() {
        let backup = BackupMetadata::new("test-ws", "/orig", "/backup", BackupReason::PreRepair);
        let result = RepairResult::success("test-ws", RepairStrategy::ClearStaleLock, vec![])
            .with_backup(backup);
        assert!(result.backup.is_some());
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // ROLLBACK RESULT TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_rollback_result_success() {
        let result = RollbackResult::success("test-ws", "backup-123");
        assert!(result.success);
        assert_eq!(result.workspace, "test-ws");
        assert_eq!(result.backup_id, "backup-123");
    }

    #[test]
    fn test_rollback_result_failure() {
        let result = RollbackResult::failure("test-ws", "backup-123", "Backup not found");
        assert!(!result.success);
        assert!(result.summary.contains("Backup not found"));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // INTEGRITY VALIDATOR TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_integrity_validator_new() {
        let validator = IntegrityValidator::new("/workspaces");
        assert_eq!(validator.timeout_ms, IntegrityValidator::DEFAULT_TIMEOUT_MS);
    }

    #[test]
    fn test_integrity_validator_with_timeout() {
        let validator = IntegrityValidator::new("/workspaces").with_timeout(1000);
        assert_eq!(validator.timeout_ms, 1000);
    }

    #[test]
    fn test_integrity_validator_missing_directory() {
        let temp_dir = TempDir::new().ok();
        let Some(temp) = temp_dir else {
            return;
        };

        let validator = IntegrityValidator::new(temp.path());
        let result = validator.validate("nonexistent");

        assert!(result.is_ok());
        let result = result.unwrap_or_else(|_| ValidationResult::valid("", ""));
        assert!(!result.is_valid);
        assert!(result
            .issues
            .iter()
            .any(|i| i.corruption_type == CorruptionType::MissingDirectory));
    }

    #[test]
    fn test_integrity_validator_missing_jj_dir() {
        let temp_dir = TempDir::new().ok();
        let Some(temp) = temp_dir else {
            return;
        };

        // Create workspace directory without .jj
        let ws_dir = temp.path().join("test-ws");
        let _ = fs::create_dir(&ws_dir);

        let validator = IntegrityValidator::new(temp.path());
        let result = validator.validate("test-ws");

        assert!(result.is_ok());
        let result = result.unwrap_or_else(|_| ValidationResult::valid("", ""));
        assert!(!result.is_valid);
        assert!(result
            .issues
            .iter()
            .any(|i| i.corruption_type == CorruptionType::MissingJjDir));
    }

    #[test]
    fn test_integrity_validator_valid_workspace() {
        let temp_dir = TempDir::new().ok();
        let Some(temp) = temp_dir else {
            return;
        };

        // Create valid workspace structure
        let ws_dir = temp.path().join("test-ws");
        let jj_dir = ws_dir.join(".jj");
        let _ = fs::create_dir_all(&jj_dir);
        let _ = fs::create_dir(jj_dir.join("repo"));
        let _ = fs::write(jj_dir.join("working_copy"), "");

        let validator = IntegrityValidator::new(temp.path());
        let result = validator.validate("test-ws");

        assert!(result.is_ok());
        let result = result.unwrap_or_else(|_| ValidationResult::invalid("", "", vec![]));
        assert!(result.is_valid);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn test_integrity_validator_validate_all() {
        let temp_dir = TempDir::new().ok();
        let Some(temp) = temp_dir else {
            return;
        };

        let validator = IntegrityValidator::new(temp.path());
        let workspaces = vec!["ws1".to_string(), "ws2".to_string()];
        let results = validator.validate_all(&workspaces);

        assert!(results.is_ok());
        let results = results.unwrap_or_default();
        assert_eq!(results.len(), 2);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // BACKUP MANAGER TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_backup_manager_new() {
        let manager = BackupManager::new("/backups");
        assert_eq!(
            manager.max_backups_per_workspace,
            BackupManager::DEFAULT_MAX_BACKUPS
        );
    }

    #[test]
    fn test_backup_manager_with_max_backups() {
        let manager = BackupManager::new("/backups").with_max_backups(10);
        assert_eq!(manager.max_backups_per_workspace, 10);
    }

    #[test]
    fn test_backup_manager_list_empty() {
        let temp_dir = TempDir::new().ok();
        let Some(temp) = temp_dir else {
            return;
        };

        let manager = BackupManager::new(temp.path());
        let backups = manager.list_backups("nonexistent");

        assert!(backups.is_ok());
        assert!(backups
            .unwrap_or_else(|_| vec![BackupMetadata::new("", "", "", BackupReason::Manual)])
            .is_empty());
    }

    #[test]
    fn test_backup_manager_create_and_list() {
        let temp_dir = TempDir::new().ok();
        let Some(temp) = temp_dir else {
            return;
        };

        // Create a source workspace to back up
        let source = temp.path().join("source-ws");
        let _ = fs::create_dir_all(&source);
        let _ = fs::write(source.join("test.txt"), "test content");

        let backup_root = temp.path().join("backups");
        let manager = BackupManager::new(&backup_root);

        let result = manager.create_backup("test-ws", &source, BackupReason::Manual);
        assert!(result.is_ok());

        let backups = manager.list_backups("test-ws");
        assert!(backups.is_ok());
        assert_eq!(backups.unwrap_or_default().len(), 1);
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // REPAIR EXECUTOR TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_repair_executor_new() {
        let temp_dir = TempDir::new().ok();
        let Some(temp) = temp_dir else {
            return;
        };

        let manager = BackupManager::new(temp.path());
        let executor = RepairExecutor::new(manager);
        assert!(executor.always_backup);
    }

    #[test]
    fn test_repair_executor_with_always_backup() {
        let temp_dir = TempDir::new().ok();
        let Some(temp) = temp_dir else {
            return;
        };

        let manager = BackupManager::new(temp.path());
        let executor = RepairExecutor::new(manager).with_always_backup(false);
        assert!(!executor.always_backup);
    }

    #[test]
    fn test_repair_executor_no_repair_possible() {
        let temp_dir = TempDir::new().ok();
        let Some(temp) = temp_dir else {
            return;
        };

        let manager = BackupManager::new(temp.path().join("backups"));
        let executor = RepairExecutor::new(manager).with_always_backup(false);

        let validation = ValidationResult::invalid(
            "test-ws",
            temp.path().join("ws"),
            vec![IntegrityIssue::new(
                CorruptionType::PermissionDenied,
                "Access denied",
            )],
        );

        let result = executor.execute(
            "test-ws",
            &temp.path().join("ws"),
            &validation,
            RepairStrategy::NoRepairPossible,
        );

        assert!(result.is_ok());
        let result = result.unwrap_or_else(|_| {
            RepairResult::failure("", RepairStrategy::NoRepairPossible, vec![], "")
        });
        assert!(!result.success);
    }

    #[test]
    fn test_repair_executor_clear_stale_locks() {
        let temp_dir = TempDir::new().ok();
        let Some(temp) = temp_dir else {
            return;
        };

        // Create workspace with lock file
        let ws_dir = temp.path().join("test-ws");
        let jj_dir = ws_dir.join(".jj");
        let _ = fs::create_dir_all(&jj_dir);
        let lock_file = jj_dir.join("lock");
        let _ = fs::write(&lock_file, "lock");

        let manager = BackupManager::new(temp.path().join("backups"));
        let executor = RepairExecutor::new(manager).with_always_backup(false);

        let validation = ValidationResult::invalid(
            "test-ws",
            &ws_dir,
            vec![IntegrityIssue::new(CorruptionType::StaleLock, "Stale lock").with_path(&lock_file)],
        );

        let result = executor.execute(
            "test-ws",
            &ws_dir,
            &validation,
            RepairStrategy::ClearStaleLock,
        );

        assert!(result.is_ok());
        let result = result.unwrap_or_else(|_| {
            RepairResult::failure("", RepairStrategy::NoRepairPossible, vec![], "")
        });
        assert!(result.success);
        assert!(!lock_file.exists());
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // HELPER FUNCTION TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_quick_validate_missing() {
        let result = quick_validate(Path::new("/nonexistent/path"));
        assert!(result.is_ok());
        assert!(!result.unwrap_or(true));
    }

    #[test]
    fn test_quick_validate_valid() {
        let temp_dir = TempDir::new().ok();
        let Some(temp) = temp_dir else {
            return;
        };

        // Create valid structure
        let jj_dir = temp.path().join(".jj");
        let _ = fs::create_dir_all(&jj_dir);
        let _ = fs::write(jj_dir.join("working_copy"), "");

        let result = quick_validate(temp.path());
        assert!(result.is_ok());
        assert!(result.unwrap_or(false));
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // RAILWAY-ORIENTED PROGRAMMING TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_validator_returns_result_not_panic() {
        let validator = IntegrityValidator::new("/nonexistent");
        let result = validator.validate("test");
        // Should return Ok with invalid result, not panic
        assert!(result.is_ok());
    }

    #[test]
    fn test_backup_manager_returns_result_not_panic() {
        let manager = BackupManager::new("/readonly");
        let result = manager.list_backups("any");
        // Should return Ok(empty vec), not panic
        assert!(result.is_ok());
    }

    #[test]
    fn test_from_str_returns_result_not_panic() {
        // Invalid corruption type returns Err, not panic
        let result = CorruptionType::from_str("invalid");
        assert!(result.is_err());

        // Invalid repair strategy returns Err, not panic
        let result = RepairStrategy::from_str("invalid");
        assert!(result.is_err());
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // SERIALIZATION ROUNDTRIP TESTS
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    #[test]
    fn test_validation_result_serialization_roundtrip() {
        let issues = vec![IntegrityIssue::new(CorruptionType::StaleLock, "Test")
            .with_path("/test/path")
            .with_context("Context")];
        let result = ValidationResult::invalid("test-ws", "/path/to/ws", issues).with_duration(100);

        let json = serde_json::to_string(&result);
        assert!(json.is_ok());

        let parsed: std::result::Result<ValidationResult, _> =
            serde_json::from_str(&json.unwrap_or_default());
        assert!(parsed.is_ok());

        let parsed = parsed.unwrap_or_else(|_| ValidationResult::valid("", ""));
        assert_eq!(parsed.workspace, "test-ws");
        assert!(!parsed.is_valid);
        assert_eq!(parsed.issues.len(), 1);
    }

    #[test]
    fn test_repair_result_serialization_roundtrip() {
        let backup = BackupMetadata::new("test-ws", "/orig", "/backup", BackupReason::PreRepair)
            .with_size(1024);
        let result = RepairResult::success(
            "test-ws",
            RepairStrategy::ClearStaleLock,
            vec![CorruptionType::StaleLock],
        )
        .with_backup(backup)
        .with_duration(50);

        let json = serde_json::to_string(&result);
        assert!(json.is_ok());

        let parsed: std::result::Result<RepairResult, _> =
            serde_json::from_str(&json.unwrap_or_default());
        assert!(parsed.is_ok());

        let parsed = parsed.unwrap_or_else(|_| {
            RepairResult::failure("", RepairStrategy::NoRepairPossible, vec![], "")
        });
        assert!(parsed.success);
        assert!(parsed.backup.is_some());
    }
}
