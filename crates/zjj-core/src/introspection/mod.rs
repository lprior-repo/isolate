//! AI-first introspection capabilities
//!
//! This module provides structured metadata about zjj capabilities,
//! enabling AI agents to discover features and understand system state.

mod command_specs;
mod doctor_types;
mod operations;
mod output_types;
mod query_types;

// Re-export all public types
pub use output_types::{
    Capabilities, CapabilityCategory, DependencyInfo, IntrospectOutput, SystemState,
};

pub use command_specs::{
    ArgumentSpec, CommandExample, CommandIntrospection, ErrorCondition, FlagSpec, Prerequisites,
};

pub use doctor_types::{
    CheckStatus, DoctorCheck, DoctorFixOutput, DoctorOutput, FixResult, UnfixableIssue,
};

pub use query_types::{
    Blocker, CanRunQuery, QueryError, SessionCountQuery, SessionExistsQuery, SessionInfo,
    SuggestNameQuery,
};

pub use operations::suggest_name;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Result;

    #[test]
    fn test_introspect_output_new() {
        let output = IntrospectOutput::new("0.1.0");
        assert_eq!(output.zjj_version, "0.1.0");
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
        assert!(!output.ai_guidance.is_empty());
        assert!(output.ai_guidance.len() >= 3);
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
}
