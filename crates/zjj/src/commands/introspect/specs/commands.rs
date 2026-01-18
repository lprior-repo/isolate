//! Command dispatcher and routing
//!
//! This module provides functions to retrieve command specifications by name.
//! It acts as the public interface for command spec lookups.

use super::builders;
use zjj_core::introspection::CommandIntrospection;

/// Get introspection data for a command by name
///
/// # Arguments
/// * `command` - Name of the command to introspect
///
/// # Returns
/// Ok(CommandIntrospection) if the command is recognized
/// Err(String) if the command is unknown
///
/// # Example
/// ```ignore
/// let spec = get_command_spec("add")?;
/// println!("{}", spec.description);
/// ```
pub fn get_command_spec(command: &str) -> Result<CommandIntrospection, String> {
    match command {
        "add" => Ok(builders::add()),
        "remove" => Ok(builders::remove()),
        "list" => Ok(builders::list()),
        "init" => Ok(builders::init()),
        "focus" => Ok(builders::focus()),
        "status" => Ok(builders::status()),
        "sync" => Ok(builders::sync()),
        "diff" => Ok(builders::diff()),
        "introspect" => Ok(builders::introspect()),
        "doctor" => Ok(builders::doctor()),
        "query" => Ok(builders::query()),
        _ => Err(format!("Unknown command: {command}")),
    }
}

/// Get all command names that have introspection specs
///
/// Returns a vector of command names as static string references.
/// This is useful for discovery and validation purposes.
///
/// # Returns
/// Vector of all known command names
///
/// # Example
/// ```ignore
/// let commands = all_command_names();
/// println!("Available commands: {:?}", commands);
/// ```
#[allow(dead_code)]
pub fn all_command_names() -> Vec<&'static str> {
    vec![
        "add",
        "remove",
        "list",
        "init",
        "focus",
        "status",
        "sync",
        "diff",
        "introspect",
        "doctor",
        "query",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_add_spec() {
        let spec = get_command_spec("add");
        assert!(spec.is_ok());
        if let Ok(spec) = spec {
            assert_eq!(spec.command, "add");
            assert_eq!(spec.description, "Create new parallel development session");
        }
    }

    #[test]
    fn test_get_remove_spec() {
        let spec = get_command_spec("remove");
        assert!(spec.is_ok());
        if let Ok(spec) = spec {
            assert_eq!(spec.command, "remove");
            assert_eq!(spec.description, "Remove a session and its workspace");
        }
    }

    #[test]
    fn test_all_command_names() {
        let names = all_command_names();
        assert_eq!(names.len(), 11);
        assert!(names.contains(&"add"));
        assert!(names.contains(&"remove"));
        assert!(names.contains(&"query"));
    }

    #[test]
    fn test_unknown_command() {
        let spec = get_command_spec("nonexistent");
        assert!(spec.is_err());
        if let Err(e) = spec {
            assert!(e.contains("Unknown command"));
        }
    }

    #[test]
    fn test_spec_has_prerequisites() {
        get_command_spec("add")
            .map(|spec| {
                assert!(spec.prerequisites.initialized);
                assert!(spec.prerequisites.jj_installed);
            })
            .ok();
    }

    #[test]
    fn test_spec_has_examples() {
        get_command_spec("add")
            .map(|spec| {
                assert!(!spec.examples.is_empty());
                assert!(spec.examples[0].command.contains("zjj add feature-auth"));
            })
            .ok();
    }

    #[test]
    fn test_spec_has_error_conditions() {
        get_command_spec("add")
            .map(|spec| {
                assert!(!spec.error_conditions.is_empty());
                assert_eq!(spec.error_conditions[0].code, "SESSION_ALREADY_EXISTS");
            })
            .ok();
    }
}
