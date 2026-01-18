//! ZJJ setup and integration checks
//!
//! This module contains checks for zjj initialization status and integration
//! with external tools like Beads.

use std::process::Command;

use zjj_core::introspection::{CheckStatus, DoctorCheck};

use crate::cli::is_command_available;

/// Check if zjj is initialized in the current directory
pub fn check_initialized() -> DoctorCheck {
    check_initialized_at(".")
}

/// Check if zjj is initialized at a specific path
///
/// This is the implementation that allows testing without changing current directory.
pub fn check_initialized_at(base_path: impl AsRef<std::path::Path>) -> DoctorCheck {
    // Check for .zjj directory existence directly, without depending on JJ installation
    let base = base_path.as_ref();
    let zjj_dir = base.join(".zjj");
    let config_file = zjj_dir.join("config.toml");
    let initialized = zjj_dir.exists() && config_file.exists();

    DoctorCheck {
        name: "zjj Initialized".to_string(),
        status: if initialized {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        message: if initialized {
            ".zjj directory exists with valid config".to_string()
        } else {
            "zjj not initialized".to_string()
        },
        suggestion: if initialized {
            None
        } else {
            Some("Initialize zjj: zjj init".to_string())
        },
        auto_fixable: false,
        details: None,
    }
}

/// Check Beads integration
pub fn check_beads() -> DoctorCheck {
    let installed = is_command_available("bd");

    if !installed {
        return DoctorCheck {
            name: "Beads Integration".to_string(),
            status: CheckStatus::Pass,
            message: "Beads not installed (optional)".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        };
    }

    // Count open issues
    let output = Command::new("bd").args(["list", "--status=open"]).output();

    match output {
        Ok(out) if out.status.success() => {
            let count = String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|line| !line.is_empty())
                .count();

            DoctorCheck {
                name: "Beads Integration".to_string(),
                status: CheckStatus::Pass,
                message: format!("Beads installed, {count} open issues"),
                suggestion: None,
                auto_fixable: false,
                details: None,
            }
        }
        _ => DoctorCheck {
            name: "Beads Integration".to_string(),
            status: CheckStatus::Pass,
            message: "Beads installed".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_check_initialized_detects_zjj_directory() {
        // Create a temporary directory - no need to change current directory
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Test 1: No .zjj directory - should fail
        let result = check_initialized_at(temp_dir.path());
        assert_eq!(result.status, CheckStatus::Fail);
        assert_eq!(result.name, "zjj Initialized");
        assert!(result.message.contains("not initialized"));

        // Test 2: .zjj directory exists but no config.toml - should fail
        let zjj_dir = temp_dir.path().join(".zjj");
        fs::create_dir(&zjj_dir).expect("Failed to create .zjj dir");
        let result = check_initialized_at(temp_dir.path());
        assert_eq!(result.status, CheckStatus::Fail);

        // Test 3: .zjj directory with config.toml - should pass
        fs::write(zjj_dir.join("config.toml"), "workspace_dir = \"test\"")
            .expect("Failed to write config.toml");
        let result = check_initialized_at(temp_dir.path());
        assert_eq!(result.status, CheckStatus::Pass);
        assert!(result.message.contains(".zjj directory exists"));
    }

    #[test]
    fn test_check_initialized_independent_of_jj() {
        // This test verifies that check_initialized doesn't call jj commands
        // We test this by checking it works even without a JJ repo

        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create .zjj structure WITHOUT initializing a JJ repo
        let zjj_dir = temp_dir.path().join(".zjj");
        fs::create_dir(&zjj_dir).expect("Failed to create .zjj dir");
        fs::write(zjj_dir.join("config.toml"), "workspace_dir = \"test\"")
            .expect("Failed to write config.toml");

        // Even without JJ installed/initialized, should detect .zjj
        let result = check_initialized_at(temp_dir.path());
        assert_eq!(result.status, CheckStatus::Pass);
    }

    #[test]
    fn test_check_beads_returns_valid_check() {
        let check = check_beads();
        assert_eq!(check.name, "Beads Integration");
        assert!(check.status == CheckStatus::Pass);
        assert!(!check.message.is_empty());
    }

    #[test]
    fn test_check_initialized_returns_valid_check() {
        let check = check_initialized();
        assert_eq!(check.name, "zjj Initialized");
        assert!(check.status == CheckStatus::Pass || check.status == CheckStatus::Fail);
        assert!(!check.message.is_empty());
    }
}
