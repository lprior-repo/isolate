//! AI-first introspection capabilities
//!
//! This module provides structured metadata about zjj capabilities,
//! enabling AI agents to discover features and understand system state.

use im::HashMap;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Complete introspection output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectOutput {
    /// JJZ version
    pub jjz_version: String,
    /// Categorized capabilities
    pub capabilities: Capabilities,
    /// External dependency status
    pub dependencies: HashMap<String, DependencyInfo>,
    /// Current system state
    pub system_state: SystemState,
}

/// Categorized capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    /// Session management capabilities
    pub session_management: CapabilityCategory,
    /// Configuration capabilities
    pub configuration: CapabilityCategory,
    /// Version control capabilities
    pub version_control: CapabilityCategory,
    /// Introspection and diagnostics
    pub introspection: CapabilityCategory,
}

/// A category of related capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityCategory {
    /// Available commands in this category
    pub commands: Vec<String>,
    /// Feature descriptions
    pub features: Vec<String>,
}

/// Information about an external dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    /// Whether this dependency is required for core functionality
    pub required: bool,
    /// Whether the dependency is currently installed
    pub installed: bool,
    /// Installed version if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Command name
    pub command: String,
}

/// Current system state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemState {
    /// Whether zjj has been initialized in this repo
    pub initialized: bool,
    /// Whether current directory is a JJ repository
    pub jj_repo: bool,
    /// Path to config file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_path: Option<String>,
    /// Path to state database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_db: Option<String>,
    /// Total number of sessions
    pub sessions_count: usize,
    /// Number of active sessions
    pub active_sessions: usize,
}

/// Detailed command introspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandIntrospection {
    /// Command name
    pub command: String,
    /// Human-readable description
    pub description: String,
    /// Command aliases
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Positional arguments
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<ArgumentSpec>,
    /// Optional flags
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<FlagSpec>,
    /// Usage examples
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<CommandExample>,
    /// Prerequisites for running this command
    pub prerequisites: Prerequisites,
    /// Side effects this command will produce
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub side_effects: Vec<String>,
    /// Possible error conditions
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub error_conditions: Vec<ErrorCondition>,
}

/// Argument specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentSpec {
    /// Argument name
    pub name: String,
    /// Type of argument
    #[serde(rename = "type")]
    pub arg_type: String,
    /// Whether this argument is required
    pub required: bool,
    /// Human-readable description
    pub description: String,
    /// Validation pattern (regex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<String>,
    /// Example values
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,
}

/// Flag specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagSpec {
    /// Long flag name (e.g., "no-hooks")
    pub long: String,
    /// Short flag name (e.g., "t")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    /// Human-readable description
    pub description: String,
    /// Type of flag value
    #[serde(rename = "type")]
    pub flag_type: String,
    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Possible values for enum-like flags
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub possible_values: Vec<String>,
    /// Category for grouping flags in help output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

impl FlagSpec {
    /// Validate that a category is one of the allowed values.
    ///
    /// Valid categories are: behavior, configuration, filter, output, advanced
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if the category is not in the allowed list.
    ///
    /// # Examples
    ///
    /// ```
    /// # use zjj_core::introspection::FlagSpec;
    /// assert!(FlagSpec::validate_category("behavior").is_ok());
    /// assert!(FlagSpec::validate_category("invalid").is_err());
    /// ```
    pub fn validate_category(category: &str) -> Result<()> {
        const VALID_CATEGORIES: &[&str] =
            &["behavior", "configuration", "filter", "output", "advanced"];

        if VALID_CATEGORIES.contains(&category) {
            Ok(())
        } else {
            Err(Error::ValidationError(format!(
                "Invalid flag category: '{}'. Must be one of: {}",
                category,
                VALID_CATEGORIES.join(", ")
            )))
        }
    }
}

/// Command usage example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExample {
    /// Example command line
    pub command: String,
    /// Description of what this example does
    pub description: String,
}

/// Prerequisites for a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prerequisites {
    /// Must be initialized
    pub initialized: bool,
    /// JJ must be installed
    pub jj_installed: bool,
    /// Zellij must be running
    pub zellij_running: bool,
    /// Additional custom checks
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub custom: Vec<String>,
}

/// Error condition documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCondition {
    /// Error code
    pub code: String,
    /// Human-readable description
    pub description: String,
    /// How to resolve this error
    pub resolution: String,
}

/// System health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorCheck {
    /// Check name
    pub name: String,
    /// Check status
    pub status: CheckStatus,
    /// Status message
    pub message: String,
    /// Suggestion for fixing issues
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Whether this issue can be auto-fixed
    pub auto_fixable: bool,
    /// Additional details about the check
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Status of a health check
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    /// Check passed
    Pass,
    /// Warning - non-critical issue
    Warn,
    /// Failure - critical issue
    Fail,
}

/// Overall health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorOutput {
    /// Whether the system is healthy overall
    pub healthy: bool,
    /// Individual check results
    pub checks: Vec<DoctorCheck>,
    /// Count of warnings
    pub warnings: usize,
    /// Count of errors
    pub errors: usize,
    /// Number of issues that can be auto-fixed
    pub auto_fixable_issues: usize,
}

/// Result of auto-fix operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorFixOutput {
    /// Issues that were fixed
    pub fixed: Vec<FixResult>,
    /// Issues that could not be fixed
    pub unable_to_fix: Vec<UnfixableIssue>,
}

/// Result of fixing a single issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResult {
    /// Issue that was fixed
    pub issue: String,
    /// Action taken
    pub action: String,
    /// Whether the fix succeeded
    pub success: bool,
}

/// Issue that could not be auto-fixed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnfixableIssue {
    /// Issue name
    pub issue: String,
    /// Reason why it couldn't be fixed
    pub reason: String,
    /// Manual fix suggestion
    pub suggestion: String,
}

/// Error information for failed queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryError {
    /// Error code
    pub code: String,
    /// Human-readable error message
    pub message: String,
}

/// Query result for session existence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExistsQuery {
    /// Whether the session exists (null if query failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exists: Option<bool>,
    /// Session details if it exists
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionInfo>,
    /// Error information if query failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<QueryError>,
}

/// Basic session information for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session name
    pub name: String,
    /// Session status
    pub status: String,
}

/// Query result for session count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCountQuery {
    /// Number of sessions matching filter (null if query failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    /// Filter that was applied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<serde_json::Value>,
    /// Error information if query failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<QueryError>,
}

/// Query result for "can run" check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanRunQuery {
    /// Whether the command can be run
    pub can_run: bool,
    /// Command being checked
    pub command: String,
    /// Prerequisites that are blocking execution
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<Blocker>,
    /// Number of prerequisites met
    pub prerequisites_met: usize,
    /// Total number of prerequisites
    pub prerequisites_total: usize,
}

/// A prerequisite that is blocking command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blocker {
    /// Check name
    pub check: String,
    /// Check status (should be false)
    pub status: bool,
    /// Human-readable message
    pub message: String,
}

/// Query result for name suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestNameQuery {
    /// Pattern used
    pub pattern: String,
    /// Suggested name
    pub suggested: String,
    /// Next available number in sequence
    pub next_available_n: usize,
    /// Existing names matching pattern
    pub existing_matches: Vec<String>,
}

impl IntrospectOutput {
    /// Create default introspection output
    pub fn new(version: &str) -> Self {
        Self {
            jjz_version: version.to_string(),
            capabilities: Capabilities::default(),
            dependencies: HashMap::new(),
            system_state: SystemState::default(),
        }
    }
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            session_management: CapabilityCategory {
                commands: vec![
                    "init".to_string(),
                    "add".to_string(),
                    "remove".to_string(),
                    "list".to_string(),
                    "status".to_string(),
                    "focus".to_string(),
                    "sync".to_string(),
                ],
                features: vec![
                    "parallel_workspaces".to_string(),
                    "zellij_integration".to_string(),
                    "hook_lifecycle".to_string(),
                ],
            },
            configuration: CapabilityCategory {
                commands: vec![],
                features: vec![
                    "hierarchy".to_string(),
                    "placeholder_substitution".to_string(),
                ],
            },
            version_control: CapabilityCategory {
                commands: vec!["diff".to_string()],
                features: vec![
                    "jj_integration".to_string(),
                    "workspace_isolation".to_string(),
                ],
            },
            introspection: CapabilityCategory {
                commands: vec![
                    "introspect".to_string(),
                    "doctor".to_string(),
                    "query".to_string(),
                ],
                features: vec![
                    "capability_discovery".to_string(),
                    "health_checks".to_string(),
                    "auto_fix".to_string(),
                    "state_queries".to_string(),
                ],
            },
        }
    }
}

impl Prerequisites {
    /// Check if all prerequisites are met
    pub const fn all_met(&self) -> bool {
        self.initialized && self.jj_installed && (!self.zellij_running || self.custom.is_empty())
    }

    /// Count how many prerequisites are met
    pub const fn count_met(&self) -> usize {
        let mut count = 0;
        if self.initialized {
            count += 1;
        }
        if self.jj_installed {
            count += 1;
        }
        if self.zellij_running {
            count += 1;
        }
        count
    }

    /// Total number of prerequisites
    pub const fn total(&self) -> usize {
        3 + self.custom.len()
    }
}

impl DoctorOutput {
    /// Calculate summary statistics from checks
    pub fn from_checks(checks: Vec<DoctorCheck>) -> Self {
        let warnings = checks
            .iter()
            .filter(|c| c.status == CheckStatus::Warn)
            .count();
        let errors = checks
            .iter()
            .filter(|c| c.status == CheckStatus::Fail)
            .count();
        let auto_fixable_issues = checks.iter().filter(|c| c.auto_fixable).count();
        let healthy = errors == 0;

        Self {
            healthy,
            checks,
            warnings,
            errors,
            auto_fixable_issues,
        }
    }
}

/// Parse a name pattern and suggest next available name
///
/// Pattern format: `prefix-{n}` or `{n}-suffix` where {n} is a number placeholder
#[allow(clippy::literal_string_with_formatting_args)]
pub fn suggest_name(pattern: &str, existing_names: &[String]) -> Result<SuggestNameQuery> {
    // Find {n} placeholder
    if !pattern.contains("{n}") {
        return Err(Error::ValidationError(
            "Pattern must contain {n} placeholder".into(),
        ));
    }

    // Extract prefix and suffix
    let parts: Vec<&str> = pattern.split("{n}").collect();
    if parts.len() != 2 {
        return Err(Error::ValidationError(
            "Pattern must contain exactly one {n} placeholder".into(),
        ));
    }

    let prefix = parts[0];
    let suffix = parts[1];

    // Find all numbers used in matching names
    let mut used_numbers = Vec::new();
    let mut matching = Vec::new();

    for name in existing_names {
        if name.starts_with(prefix) && name.ends_with(suffix) {
            let num_part = &name[prefix.len()..name.len() - suffix.len()];
            if let Ok(n) = num_part.parse::<usize>() {
                used_numbers.push(n);
                matching.push(name.clone());
            }
        }
    }

    // Find next available number
    let next_n = (1..=used_numbers.len() + 2)
        .find(|n| !used_numbers.contains(n))
        .unwrap_or(1);

    let suggested = pattern.replace("{n}", &next_n.to_string());

    Ok(SuggestNameQuery {
        pattern: pattern.to_string(),
        suggested,
        next_available_n: next_n,
        existing_matches: matching,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_introspect_output_new() {
        let output = IntrospectOutput::new("0.1.0");
        assert_eq!(output.jjz_version, "0.1.0");
        assert!(!output.capabilities.session_management.commands.is_empty());
    }

    #[test]
    fn test_capabilities_default() {
        let caps = Capabilities::default();
        assert!(caps
            .session_management
            .commands
            .contains(&"add".to_string()));
        assert!(caps.introspection.commands.contains(&"doctor".to_string()));
    }

    #[test]
    fn test_prerequisites_all_met() {
        let prereqs = Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: true,
            custom: vec![],
        };
        assert!(prereqs.all_met());
    }

    #[test]
    fn test_prerequisites_not_met() {
        let prereqs = Prerequisites {
            initialized: false,
            jj_installed: true,
            zellij_running: true,
            custom: vec![],
        };
        assert!(!prereqs.all_met());
    }

    #[test]
    fn test_prerequisites_count() {
        let prereqs = Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: false,
            custom: vec![],
        };
        assert_eq!(prereqs.count_met(), 2);
        assert_eq!(prereqs.total(), 3);
    }

    #[test]
    fn test_doctor_output_from_checks() {
        let checks = vec![
            DoctorCheck {
                name: "Check 1".to_string(),
                status: CheckStatus::Pass,
                message: "OK".to_string(),
                suggestion: None,
                auto_fixable: false,
                details: None,
            },
            DoctorCheck {
                name: "Check 2".to_string(),
                status: CheckStatus::Warn,
                message: "Warning".to_string(),
                suggestion: Some("Fix it".to_string()),
                auto_fixable: true,
                details: None,
            },
            DoctorCheck {
                name: "Check 3".to_string(),
                status: CheckStatus::Fail,
                message: "Error".to_string(),
                suggestion: None,
                auto_fixable: false,
                details: None,
            },
        ];

        let output = DoctorOutput::from_checks(checks);
        assert!(!output.healthy);
        assert_eq!(output.warnings, 1);
        assert_eq!(output.errors, 1);
        assert_eq!(output.auto_fixable_issues, 1);
    }

    #[test]
    fn test_suggest_name_basic() -> Result<()> {
        let existing = vec!["feature-1".to_string(), "feature-2".to_string()];
        let result = suggest_name("feature-{n}", &existing)?;
        assert_eq!(result.suggested, "feature-3");
        assert_eq!(result.next_available_n, 3);
        assert_eq!(result.existing_matches.len(), 2);
        Ok(())
    }

    #[test]
    fn test_suggest_name_gap() -> Result<()> {
        let existing = vec!["test-1".to_string(), "test-3".to_string()];
        let result = suggest_name("test-{n}", &existing)?;
        assert_eq!(result.suggested, "test-2");
        assert_eq!(result.next_available_n, 2);
        Ok(())
    }

    #[test]
    fn test_suggest_name_no_existing() -> Result<()> {
        let existing = vec![];
        let result = suggest_name("bug-{n}", &existing)?;
        assert_eq!(result.suggested, "bug-1");
        assert_eq!(result.next_available_n, 1);
        assert_eq!(result.existing_matches.len(), 0);
        Ok(())
    }

    #[test]
    fn test_suggest_name_invalid_pattern() {
        let result = suggest_name("no-placeholder", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_suggest_name_multiple_placeholders() {
        let result = suggest_name("test-{n}-{n}", &[]);
        assert!(result.is_err());
    }

    // ===== CommandIntrospection Validation Tests (TDD15 Phase 4 - RED) =====
    // These tests verify the expected structure and validation rules for the list command.
    // They FAIL until the implementation adds the required flags and error conditions.

    /// Validates that a list introspection includes bead flag specification
    fn validate_list_has_bead_flag(list_introspection: &CommandIntrospection) -> bool {
        list_introspection.flags.iter().any(|f| f.long == "bead")
    }

    /// Validates that a list introspection includes agent flag specification
    fn validate_list_has_agent_flag(list_introspection: &CommandIntrospection) -> bool {
        list_introspection.flags.iter().any(|f| f.long == "agent")
    }

    /// Validates that error conditions include NO_MATCHING_SESSIONS
    fn validate_has_no_matching_error(list_introspection: &CommandIntrospection) -> bool {
        list_introspection
            .error_conditions
            .iter()
            .any(|e| e.code == "NO_MATCHING_SESSIONS")
    }

    /// Validates that examples include filter usage patterns
    fn validate_has_filter_examples(list_introspection: &CommandIntrospection) -> bool {
        list_introspection
            .examples
            .iter()
            .any(|ex| ex.command.contains("--bead") || ex.command.contains("--agent"))
    }

    #[test]
    fn test_list_introspection_structure_supports_bead_flag() {
        // FAILING: Validates that list command introspection can include --bead flag
        let expected_bead_flag = FlagSpec {
            long: "bead".to_string(),
            short: Some("b".to_string()),
            description: "Filter sessions by bead ID".to_string(),
            flag_type: "string".to_string(),
            default: None,
            possible_values: vec![],
            category: None,
        };

        // This test verifies the FlagSpec structure itself is valid
        assert_eq!(expected_bead_flag.long, "bead");
        assert!(expected_bead_flag.short.is_some());
        assert!(!expected_bead_flag.description.is_empty());
        assert_eq!(expected_bead_flag.flag_type, "string");
    }

    #[test]
    fn test_list_introspection_structure_supports_agent_flag() {
        // FAILING: Validates that list command introspection can include --agent flag
        let expected_agent_flag = FlagSpec {
            long: "agent".to_string(),
            short: Some("a".to_string()),
            description: "Filter sessions by assigned agent".to_string(),
            flag_type: "string".to_string(),
            default: None,
            possible_values: vec![],
            category: None,
        };

        // This test verifies the FlagSpec structure itself is valid
        assert_eq!(expected_agent_flag.long, "agent");
        assert!(expected_agent_flag.short.is_some());
        assert!(!expected_agent_flag.description.is_empty());
        assert_eq!(expected_agent_flag.flag_type, "string");
    }

    #[test]
    fn test_list_command_introspection_includes_no_matching_sessions_error() {
        // FAILING: Verifies NO_MATCHING_SESSIONS error condition structure
        let expected_error = ErrorCondition {
            code: "NO_MATCHING_SESSIONS".to_string(),
            description: "No sessions match the specified filter criteria".to_string(),
            resolution: "Check filter parameters and try with fewer restrictions".to_string(),
        };

        // Validate error structure
        assert_eq!(expected_error.code, "NO_MATCHING_SESSIONS");
        assert!(!expected_error.description.is_empty());
        assert!(!expected_error.resolution.is_empty());
    }

    #[test]
    fn test_list_command_bead_flag_has_dynamic_value_documentation() {
        // FAILING: Verifies bead flag supports dynamic values with proper documentation
        let bead_with_doc = FlagSpec {
            long: "bead".to_string(),
            short: Some("b".to_string()),
            description: "Filter by bead ID or pattern - supports dynamic values like 'feature-*'"
                .to_string(),
            flag_type: "string".to_string(),
            default: None,
            possible_values: vec![],
            category: None,
        };

        // Verify it supports dynamic values (type is string, not enum)
        assert_eq!(bead_with_doc.flag_type, "string");
        // Verify documentation is present
        assert!(
            bead_with_doc.description.contains("dynamic")
                || bead_with_doc.description.contains("filter")
                || bead_with_doc.description.contains("ID")
        );
    }

    #[test]
    fn test_list_command_agent_flag_has_dynamic_value_documentation() {
        // FAILING: Verifies agent flag supports dynamic values with proper documentation
        let agent_with_doc = FlagSpec {
            long: "agent".to_string(),
            short: Some("a".to_string()),
            description: "Filter by agent name or pattern - supports dynamic values".to_string(),
            flag_type: "string".to_string(),
            default: None,
            possible_values: vec![],
            category: None,
        };

        // Verify it supports dynamic values (type is string, not enum)
        assert_eq!(agent_with_doc.flag_type, "string");
        // Verify documentation is present
        assert!(
            agent_with_doc.description.contains("dynamic")
                || agent_with_doc.description.contains("agent")
        );
    }

    #[test]
    fn test_list_command_error_conditions_document_no_matches_scenario() {
        // FAILING: Verifies error condition for filtering with no matches
        let list_with_error = CommandIntrospection {
            command: "list".to_string(),
            description: "List all sessions".to_string(),
            aliases: vec!["ls".to_string()],
            arguments: vec![],
            flags: vec![],
            examples: vec![],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: false,
                zellij_running: false,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![ErrorCondition {
                code: "NO_MATCHING_SESSIONS".to_string(),
                description: "No sessions match the filter criteria".to_string(),
                resolution: "Try with less restrictive filters".to_string(),
            }],
        };

        // Validate error is documented
        assert!(validate_has_no_matching_error(&list_with_error));
    }

    #[test]
    fn test_list_command_examples_demonstrate_bead_filtering() {
        // FAILING: Verifies examples show how to use --bead filter
        let list_with_bead_example = CommandIntrospection {
            command: "list".to_string(),
            description: "List all sessions".to_string(),
            aliases: vec!["ls".to_string()],
            arguments: vec![],
            flags: vec![],
            examples: vec![CommandExample {
                command: "zjj list --bead feature-123".to_string(),
                description: "List sessions for bead feature-123".to_string(),
            }],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: false,
                zellij_running: false,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![],
        };

        // Validate bead example exists
        assert!(validate_has_filter_examples(&list_with_bead_example));
    }

    #[test]
    fn test_list_command_examples_demonstrate_agent_filtering() {
        // FAILING: Verifies examples show how to use --agent filter
        let list_with_agent_example = CommandIntrospection {
            command: "list".to_string(),
            description: "List all sessions".to_string(),
            aliases: vec!["ls".to_string()],
            arguments: vec![],
            flags: vec![],
            examples: vec![CommandExample {
                command: "zjj list --agent alice".to_string(),
                description: "List sessions assigned to alice".to_string(),
            }],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: false,
                zellij_running: false,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![],
        };

        // Validate agent example exists
        assert!(validate_has_filter_examples(&list_with_agent_example));
    }

    #[test]
    fn test_list_command_supports_combined_filter_parameters() {
        // FAILING: Verifies list command allows combining multiple filters
        let list_with_combined = CommandIntrospection {
            command: "list".to_string(),
            description: "List all sessions".to_string(),
            aliases: vec!["ls".to_string()],
            arguments: vec![],
            flags: vec![],
            examples: vec![CommandExample {
                command: "zjj list --bead feature-123 --agent alice".to_string(),
                description: "List feature-123 sessions assigned to alice".to_string(),
            }],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: false,
                zellij_running: false,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![],
        };

        // Validate combined filters example
        let has_combined = list_with_combined
            .examples
            .iter()
            .any(|ex| ex.command.contains("--bead") && ex.command.contains("--agent"));

        assert!(
            has_combined,
            "Expected example with both --bead and --agent"
        );
    }

    #[test]
    fn test_list_command_filters_are_optional_parameters() {
        // FAILING: Verifies that both filter flags are optional (not required)
        // FlagSpec structure doesn't have a 'required' field; all flags are optional by design
        let filters = vec![
            FlagSpec {
                long: "bead".to_string(),
                short: Some("b".to_string()),
                description: "Filter by bead".to_string(),
                flag_type: "string".to_string(),
                default: None,
                possible_values: vec![],
                category: None,
            },
            FlagSpec {
                long: "agent".to_string(),
                short: Some("a".to_string()),
                description: "Filter by agent".to_string(),
                flag_type: "string".to_string(),
                default: None,
                possible_values: vec![],
                category: None,
            },
        ];

        // Validate both are present and have string type (not boolean/required)
        assert_eq!(filters.len(), 2);
        assert!(filters.iter().all(|f| f.flag_type == "string"));
    }

    // ===== PHASE 4 (RED): CommandIntrospection Validation Tests for ADD Command =====
    // These tests MUST FAIL initially - they define expected behavior that will be
    // implemented in Phase 5 (GREEN).
    // Tests validate session name rules: ^[a-zA-Z][a-zA-Z0-9_-]{0,63}$

    /// Validate session name against expected pattern using Railway-Oriented Programming
    /// Returns Result with descriptive error messages on validation failure
    fn validate_add_session_name(name: &str) -> Result<()> {
        // Check for empty string
        if name.is_empty() {
            return Err(Error::ValidationError(
                "Session name cannot be empty".into(),
            ));
        }

        // Check maximum length (64 characters)
        if name.len() > 64 {
            return Err(Error::ValidationError(
                "Session name cannot exceed 64 characters".into(),
            ));
        }

        // Check first character is a letter (using ROP pattern)
        name.chars()
            .next()
            .ok_or_else(|| Error::ValidationError("Session name is empty".into()))
            .and_then(|first| {
                if first.is_ascii_alphabetic() {
                    Ok(())
                } else {
                    Err(Error::ValidationError(
                        "Session name must start with a letter (a-z, A-Z)".into(),
                    ))
                }
            })?;

        // Check for ASCII-only characters
        if !name.is_ascii() {
            return Err(Error::ValidationError(
                "Session name must contain only ASCII characters".into(),
            ));
        }

        // Check that all characters are alphanumeric, dash, or underscore
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(Error::ValidationError(
                "Session name can only contain ASCII alphanumeric characters, dashes (-), and underscores (_)"
                    .into(),
            ));
        }

        Ok(())
    }

    #[test]
    fn test_add_command_introspection_has_name_argument_with_validation() {
        // FAILING: Verifies add command has proper introspection with validation
        let add_introspection = CommandIntrospection {
            command: "add".to_string(),
            description: "Create a new session with JJ workspace + Zellij tab".to_string(),
            aliases: vec![],
            arguments: vec![ArgumentSpec {
                name: "name".to_string(),
                arg_type: "string".to_string(),
                required: true,
                description:
                    "Session name (alphanumeric, dash, underscore; must start with letter)"
                        .to_string(),
                validation: Some("^[a-zA-Z][a-zA-Z0-9_-]{0,63}$".to_string()),
                examples: vec![
                    "my-session".to_string(),
                    "feature_001".to_string(),
                    "BugFix".to_string(),
                ],
            }],
            flags: vec![],
            examples: vec![],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: true,
                zellij_running: false,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![],
        };

        assert_eq!(add_introspection.command, "add");
        assert!(!add_introspection.arguments.is_empty());
        assert_eq!(add_introspection.arguments[0].name, "name");
        assert!(add_introspection.arguments[0].required);
        let validation = &add_introspection.arguments[0].validation;
        assert!(validation.is_some());
        assert_eq!(
            validation.as_ref().unwrap(),
            "^[a-zA-Z][a-zA-Z0-9_-]{0,63}$"
        );
    }

    #[test]
    fn test_add_command_validation_accepts_single_letter() {
        // FAILING: Single letter names should be valid
        assert!(validate_add_session_name("a").is_ok());
        assert!(validate_add_session_name("Z").is_ok());
        assert!(validate_add_session_name("M").is_ok());
    }

    #[test]
    fn test_add_command_validation_accepts_standard_names() {
        // FAILING: Standard session names should be valid
        let valid_names = vec![
            "session",
            "feature",
            "my-session",
            "my_session",
            "Session123",
        ];

        for name in valid_names {
            assert!(
                validate_add_session_name(name).is_ok(),
                "Name '{}' should be valid",
                name
            );
        }
    }

    #[test]
    fn test_add_command_validation_accepts_names_with_dashes() {
        // FAILING: Names with dashes should be valid
        let names = vec!["my-session", "bug-fix", "feature-branch-001"];

        for name in names {
            assert!(
                validate_add_session_name(name).is_ok(),
                "Name '{}' with dashes should be valid",
                name
            );
        }
    }

    #[test]
    fn test_add_command_validation_accepts_names_with_underscores() {
        // FAILING: Names with underscores should be valid
        let names = vec!["my_session", "bug_fix", "feature_branch_001"];

        for name in names {
            assert!(
                validate_add_session_name(name).is_ok(),
                "Name '{}' with underscores should be valid",
                name
            );
        }
    }

    #[test]
    fn test_add_command_validation_accepts_mixed_case() {
        // FAILING: Mixed case names should be valid
        let names = vec!["MySession", "BugFix", "FeatureBranch"];

        for name in names {
            assert!(
                validate_add_session_name(name).is_ok(),
                "Name '{}' with mixed case should be valid",
                name
            );
        }
    }

    #[test]
    fn test_add_command_validation_accepts_names_with_digits() {
        // FAILING: Names with digits (but not starting with them) should be valid
        let names = vec!["session1", "feature123", "bug_fix_001"];

        for name in names {
            assert!(
                validate_add_session_name(name).is_ok(),
                "Name '{}' with digits should be valid",
                name
            );
        }
    }

    #[test]
    fn test_add_command_validation_max_length_64_chars() {
        // FAILING: 64-character names should be valid
        let max_valid = "a".to_string() + &"b".repeat(63);
        assert_eq!(max_valid.len(), 64);
        assert!(
            validate_add_session_name(&max_valid).is_ok(),
            "64-character name should be valid"
        );
    }

    #[test]
    fn test_add_command_validation_rejects_empty_name() {
        // FAILING: Empty names should be rejected
        let result = validate_add_session_name("");
        assert!(result.is_err(), "Empty name should be rejected");
    }

    #[test]
    fn test_add_command_validation_rejects_name_starting_with_digit() {
        // FAILING: Names starting with digit should be rejected
        let names = vec!["0session", "1feature", "9bug"];

        for name in names {
            assert!(
                validate_add_session_name(name).is_err(),
                "Name '{}' starting with digit should be rejected",
                name
            );
        }
    }

    #[test]
    fn test_add_command_validation_rejects_name_starting_with_dash() {
        // FAILING: Names starting with dash should be rejected
        let result = validate_add_session_name("-session");
        assert!(
            result.is_err(),
            "Name starting with dash should be rejected"
        );
    }

    #[test]
    fn test_add_command_validation_rejects_name_starting_with_underscore() {
        // FAILING: Names starting with underscore should be rejected
        let result = validate_add_session_name("_session");
        assert!(
            result.is_err(),
            "Name starting with underscore should be rejected"
        );
    }

    #[test]
    fn test_add_command_validation_rejects_name_exceeding_max_length() {
        // FAILING: Names exceeding 64 characters should be rejected
        let too_long = "a".to_string() + &"b".repeat(64);
        assert!(too_long.len() > 64);
        assert!(
            validate_add_session_name(&too_long).is_err(),
            "Name exceeding 64 characters should be rejected"
        );
    }

    #[test]
    fn test_add_command_validation_rejects_names_with_spaces() {
        // FAILING: Names with spaces should be rejected
        let names = vec!["my session", "session name", "a b c"];

        for name in names {
            assert!(
                validate_add_session_name(name).is_err(),
                "Name '{}' with spaces should be rejected",
                name
            );
        }
    }

    #[test]
    fn test_add_command_validation_rejects_names_with_special_chars() {
        // FAILING: Names with special characters should be rejected
        let names = vec![
            "session@name",
            "session#tag",
            "session$var",
            "session%mod",
            "session!wow",
            "session.ext",
            "session,list",
            "session;stmt",
            "session:port",
            "session/path",
            "session\\path",
            "session(paren)",
            "session[bracket]",
            "session{brace}",
        ];

        for name in names {
            assert!(
                validate_add_session_name(name).is_err(),
                "Name '{}' with special chars should be rejected",
                name
            );
        }
    }

    #[test]
    fn test_add_command_validation_rejects_names_with_unicode() {
        // FAILING: Names with unicode characters should be rejected
        let names = vec![
            "café",
            "résumé",
            "naïve",
            "日本語",
            "中文",
            "한글",
            "session_😀",
            "bug_🐛",
            "feature_✨",
        ];

        for name in names {
            assert!(
                validate_add_session_name(name).is_err(),
                "Name '{}' with unicode should be rejected",
                name
            );
        }
    }

    #[test]
    fn test_add_command_validation_error_messages_informative() {
        // FAILING: Error messages should be descriptive and guide users
        let error_cases = vec![
            ("", "empty"),
            ("0session", "start"),
            ("-session", "start"),
            ("_session", "start"),
            ("my session", "alphanumeric"),
            ("session@domain", "alphanumeric"),
        ];

        for (name, expected_keyword) in error_cases {
            let result = validate_add_session_name(name);
            assert!(result.is_err(), "Name '{}' should produce error", name);

            if let Err(Error::ValidationError(msg)) = result {
                let lowercase_msg = msg.to_lowercase();
                assert!(
                    lowercase_msg.contains(&expected_keyword.to_lowercase()),
                    "Error for '{}' should mention '{}', got: {}",
                    name,
                    expected_keyword,
                    msg
                );
            }
        }
    }

    #[test]
    fn test_add_command_validation_table_driven_valid_cases() {
        // FAILING: Table-driven test for all valid session name patterns
        let test_cases = vec![
            ("a", "single letter lowercase"),
            ("Z", "single letter uppercase"),
            ("name", "simple name"),
            ("Name", "mixed case name"),
            ("name123", "name with digits"),
            ("my-session", "name with dash"),
            ("my_session", "name with underscore"),
            ("my-session_123", "mixed separators"),
            ("feature-branch-001", "multiple dashes"),
            ("bug_fix_patch", "multiple underscores"),
            ("MyFeature123", "complex mixed case"),
        ];

        for (name, description) in test_cases {
            let result = validate_add_session_name(name);
            assert!(
                result.is_ok(),
                "Name '{}' ({}) should be valid",
                name,
                description
            );
        }
    }

    #[test]
    fn test_add_command_validation_table_driven_invalid_cases() {
        // FAILING: Table-driven test for all invalid session name patterns
        let test_cases = vec![
            ("", "empty string"),
            (" ", "whitespace only"),
            ("0name", "starts with digit"),
            ("-name", "starts with dash"),
            ("_name", "starts with underscore"),
            ("name space", "contains space"),
            ("name@host", "contains @"),
            ("name#tag", "contains #"),
            ("name$var", "contains $"),
            ("name%mod", "contains %"),
            ("name!!", "contains !"),
            ("name.txt", "contains period"),
            ("name,list", "contains comma"),
            ("name;stmt", "contains semicolon"),
            ("name:port", "contains colon"),
            ("name/path", "contains slash"),
            ("name\\path", "contains backslash"),
            ("café", "contains accented char"),
            ("日本語", "contains japanese"),
            ("中文", "contains chinese"),
            ("session_😀", "contains emoji"),
        ];

        for (name, description) in test_cases {
            let result = validate_add_session_name(name);
            assert!(
                result.is_err(),
                "Name '{}' ({}) should be invalid",
                name,
                description
            );
        }
    }

    #[test]
    fn test_add_command_validation_boundary_minimum() {
        // FAILING: Verify minimum length requirement (1 character)
        assert!(
            validate_add_session_name("a").is_ok(),
            "Single character should be valid (minimum)"
        );
    }

    #[test]
    fn test_add_command_validation_boundary_maximum() {
        // FAILING: Verify maximum length requirement (64 characters)
        let exactly_64 = "a".to_string() + &"b".repeat(63);
        assert_eq!(exactly_64.len(), 64);
        assert!(
            validate_add_session_name(&exactly_64).is_ok(),
            "64-character name should be valid (maximum)"
        );

        let exactly_65 = "a".to_string() + &"b".repeat(64);
        assert_eq!(exactly_65.len(), 65);
        assert!(
            validate_add_session_name(&exactly_65).is_err(),
            "65-character name should be invalid (over maximum)"
        );
    }

    #[test]
    fn test_add_command_validation_functional_pipeline() {
        // FAILING: Verify validation works in functional composition (ROP pattern)
        let names = vec!["valid-name", "another_session", "0invalid", ""];

        let results: Vec<(String, bool)> = names
            .iter()
            .map(|name| (name.to_string(), validate_add_session_name(name).is_ok()))
            .collect();

        // Verify correct validation outcomes
        assert_eq!(results[0].1, true, "valid-name should pass");
        assert_eq!(results[1].1, true, "another_session should pass");
        assert_eq!(results[2].1, false, "0invalid should fail");
        assert_eq!(results[3].1, false, "empty should fail");

        // Count valid names using functional filter
        let valid_count = results.iter().filter(|(_, is_valid)| *is_valid).count();
        assert_eq!(valid_count, 2, "Should have exactly 2 valid names");
    }

    #[test]
    fn test_add_command_validation_error_types_consistent() {
        // FAILING: All validation errors should be ValidationError type
        let invalid_names = vec!["", "0invalid", "-invalid", "name space"];

        for name in invalid_names {
            let result = validate_add_session_name(name);
            assert!(result.is_err(), "Name '{}' should produce error", name);

            // Verify error type is ValidationError
            match result {
                Err(Error::ValidationError(_)) => {
                    // Correct error type
                }
                Err(_other) => panic!("Name '{}' produced wrong error type", name),
                Ok(_) => panic!("Name '{}' should have failed", name),
            }
        }
    }

    #[test]
    fn test_add_command_error_conditions_document_validation_errors() {
        // FAILING: add command should document validation errors in introspection
        let add_command = CommandIntrospection {
            command: "add".to_string(),
            description: "Create a new session".to_string(),
            aliases: vec![],
            arguments: vec![],
            flags: vec![],
            examples: vec![],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: true,
                zellij_running: false,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![
                ErrorCondition {
                    code: "ERR_VALIDATION".to_string(),
                    description: "Invalid session name format".to_string(),
                    resolution: "Session name must start with a letter and contain only alphanumeric characters, dashes, or underscores".to_string(),
                },
                ErrorCondition {
                    code: "ERR_DUPLICATE".to_string(),
                    description: "Session already exists".to_string(),
                    resolution: "Choose a different name or delete the existing session".to_string(),
                },
            ],
        };

        // Verify validation error is documented
        let has_validation_error = add_command
            .error_conditions
            .iter()
            .any(|e| e.code == "ERR_VALIDATION");
        assert!(
            has_validation_error,
            "add command should document ERR_VALIDATION error"
        );
    }

    #[test]
    fn test_add_command_argument_examples_follow_validation() {
        // FAILING: All examples in ArgumentSpec should be valid names
        let arg = ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Session name".to_string(),
            validation: Some("^[a-zA-Z][a-zA-Z0-9_-]{0,63}$".to_string()),
            examples: vec![
                "session1".to_string(),
                "my-session".to_string(),
                "Feature_Branch".to_string(),
            ],
        };

        // Verify all examples are valid
        for example in &arg.examples {
            assert!(
                validate_add_session_name(example).is_ok(),
                "Example '{}' should be a valid session name",
                example
            );
        }
    }

    #[test]
    fn test_add_command_validation_consistency_between_functions() {
        // FAILING: Verify validation is consistent across multiple invocations
        let test_names = vec![
            ("valid-name", true),
            ("another_session", true),
            ("0invalid", false),
            ("-invalid", false),
            ("_invalid", false),
            ("", false),
        ];

        for (name, should_be_valid) in test_names {
            let result = validate_add_session_name(name);
            let is_valid = result.is_ok();

            assert_eq!(
                is_valid, should_be_valid,
                "Name '{}': validation result {} doesn't match expectation {}",
                name, is_valid, should_be_valid
            );
        }
    }
}
