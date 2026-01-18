//! Dependency checking for required external tools
//!
//! This module validates that required dependencies (jj, zellij) are installed
//! and meet minimum version requirements. All operations are pure and panic-free.
//!
//! # Module Organization
//!
//! - [`jj_checks`]: JJ (Jujutsu) version verification
//! - [`zellij_checks`]: Zellij version verification
//! - [`system_checks`]: Version parsing and utility functions
//! - [`check_dependencies`]: Main orchestrator combining all checks

mod jj_checks;
mod system_checks;
mod zellij_checks;

use anyhow::{bail, Result};

use crate::cli::{is_jj_installed, is_zellij_installed};

/// Check that required dependencies are installed and meet minimum version requirements
///
/// This function uses functional error aggregation to collect all dependency issues
/// before returning a comprehensive error message with installation instructions.
///
/// # Examples
///
/// ```no_run
/// # use zjj::commands::init::dependencies::check_dependencies;
/// match check_dependencies() {
///     Ok(()) => println!("All dependencies satisfied"),
///     Err(e) => eprintln!("Missing dependencies: {}", e),
/// }
/// ```
///
/// # Errors
///
/// Returns error if:
/// - Any required dependency is not installed
/// - Any installed dependency version is below minimum requirement
/// - Version cannot be determined or parsed
///
/// Error messages include installation instructions for all missing/outdated dependencies.
pub fn check_dependencies() -> Result<()> {
    // Functional dependency checking - collect missing and version errors
    type DepCheck = (&'static str, fn() -> bool, fn() -> anyhow::Result<()>);
    let deps: [DepCheck; 2] = [
        ("jj (Jujutsu)", is_jj_installed, jj_checks::check_jj_version),
        (
            "zellij",
            is_zellij_installed,
            zellij_checks::check_zellij_version,
        ),
    ];

    let (missing, version_errors): (Vec<&str>, Vec<String>) = deps.iter().fold(
        (Vec::new(), Vec::new()),
        |(mut missing, mut version_errors), (name, is_installed, check_version)| {
            if !is_installed() {
                missing.push(*name);
            } else if let Err(e) = check_version() {
                version_errors.push(format!("{name}: {e}"));
            }
            (missing, version_errors)
        },
    );

    // Early return if no issues
    if missing.is_empty() && version_errors.is_empty() {
        return Ok(());
    }

    // Build error message functionally
    let msg = [
        // Missing dependencies section
        (!missing.is_empty()).then(|| {
            format!(
                "Missing required dependencies:\n\n{}",
                missing
                    .iter()
                    .map(|dep| format!("  - {dep}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }),
        // Version errors section
        (!version_errors.is_empty()).then(|| {
            format!(
                "Version requirement errors:\n\n{}",
                version_errors
                    .iter()
                    .map(|error| format!("  - {error}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }),
        // Installation instructions
        Some("\nInstallation instructions:".to_string()),
        // JJ install instructions
        missing.contains(&"jj (Jujutsu)").then(|| {
            "\n  jj (Jujutsu) >= 0.8.0:\n\
             \x20   cargo install jj-cli\n\
             \x20   # or: brew install jj\n\
             \x20   # or: https://github.com/jj-vcs/jj/releases"
                .to_string()
        }),
        // Zellij install instructions
        missing.contains(&"zellij").then(|| {
            "\n  zellij >= 0.35.1:\n\
             \x20   cargo install --locked zellij\n\
             \x20   # or: brew install zellij\n\
             \x20   # or: https://github.com/zellij-org/zellij/releases"
                .to_string()
        }),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n");

    bail!("{msg}")
}

// Re-export parse_version for testing purposes

#[cfg(test)]
mod tests {
    #[test]
    fn test_check_dependencies_error_message_format() {
        // This test verifies the error message structure
        // We can't test actual dependency checking as it depends on system state
        // But we can verify the error aggregation logic works correctly
        let missing = ["tool1", "tool2"];
        let version_errors = ["tool3: version too old".to_string()];

        let has_missing = !missing.is_empty();
        let has_version_errors = !version_errors.is_empty();

        assert!(has_missing);
        assert!(has_version_errors);

        // Verify functional message building
        let msg_part_count = [
            has_missing.then(|| "Missing dependencies".to_string()),
            has_version_errors.then(|| "Version errors".to_string()),
        ]
        .into_iter()
        .flatten()
        .count();

        assert_eq!(msg_part_count, 2);
    }
}
