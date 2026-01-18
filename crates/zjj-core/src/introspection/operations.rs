//! Operations and implementations for introspection types

use super::{
    command_specs::Prerequisites,
    doctor_types::{CheckStatus, DoctorCheck, DoctorOutput},
    output_types::{Capabilities, CapabilityCategory, IntrospectOutput},
    query_types::SuggestNameQuery,
};
use crate::{Error, Result};
use im::HashMap;

impl IntrospectOutput {
    /// Create default introspection output
    pub fn new(version: &str) -> Self {
        let version_string = version.to_string();
        Self {
            version: version_string.clone(),
            zjj_version: version_string,
            capabilities: Capabilities::default(),
            dependencies: HashMap::new(),
            system_state: Default::default(),
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
        let mut count: usize = 0;
        if self.initialized {
            count = count.saturating_add(1);
        }
        if self.jj_installed {
            count = count.saturating_add(1);
        }
        if self.zellij_running {
            count = count.saturating_add(1);
        }
        count
    }

    /// Total number of prerequisites
    pub const fn total(&self) -> usize {
        3_usize.saturating_add(self.custom.len())
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

        // AI-specific guidance based on system state
        let ai_guidance = vec![
            "Run 'zjj context --json' for complete environment state".to_string(),
            "Run 'zjj introspect --json' to discover all available commands".to_string(),
            "Run 'zjj query can-run <command>' to check prerequisites before executing".to_string(),
            "Use '--json' flag with any command for machine-readable output".to_string(),
        ];

        Self {
            healthy,
            checks,
            warnings,
            errors,
            auto_fixable_issues,
            ai_guidance,
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
        return Err(Error::validation_error(
            "Pattern must contain {n} placeholder",
        ));
    }

    // Extract prefix and suffix
    let parts: Vec<&str> = pattern.split("{n}").collect();
    if parts.len() != 2 {
        return Err(Error::validation_error(
            "Pattern must contain exactly one {n} placeholder",
        ));
    }

    let prefix = parts[0];
    let suffix = parts[1];

    // Find all numbers used in matching names using functional pipeline
    let (used_numbers, matching): (Vec<usize>, Vec<String>) = existing_names
        .iter()
        .filter(|name| name.starts_with(prefix) && name.ends_with(suffix))
        .filter_map(|name| {
            let end_idx = name.len().saturating_sub(suffix.len());
            let num_part = &name[prefix.len()..end_idx];
            num_part.parse::<usize>().ok().map(|n| (n, name.clone()))
        })
        .unzip();

    // Find next available number
    let next_n = (1..=used_numbers.len().saturating_add(2))
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
