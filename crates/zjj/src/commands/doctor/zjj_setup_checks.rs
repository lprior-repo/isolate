//! JJZ setup and integration checks
//!
//! This module contains checks for jjz initialization status and integration
//! with external tools like Beads.

use std::process::Command;

use zjj_core::introspection::{CheckStatus, DoctorCheck};

use crate::cli::is_command_available;

/// Check if jjz is initialized
pub fn check_initialized() -> DoctorCheck {
    // Check for .jjz directory existence directly, without depending on JJ installation
    let jjz_dir = std::path::Path::new(".jjz");
    let config_file = jjz_dir.join("config.toml");
    let initialized = jjz_dir.exists() && config_file.exists();

    DoctorCheck {
        name: "jjz Initialized".to_string(),
        status: if initialized {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        message: if initialized {
            ".jjz directory exists with valid config".to_string()
        } else {
            "jjz not initialized".to_string()
        },
        suggestion: if initialized {
            None
        } else {
            Some("Initialize jjz: jjz init".to_string())
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
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_check_initialized_detects_jjz_directory() {
        // Create a temporary directory
        let temp_dir = TempDir::new().ok();
        let Some(temp_dir) = temp_dir else {
            return;
        };

        // Change to temp directory
        let original_dir = std::env::current_dir().ok();
        let Some(original_dir) = original_dir else {
            return;
        };
        if std::env::set_current_dir(temp_dir.path()).is_err() {
            return;
        }

        // Test 1: No .jjz directory - should fail
        let result = check_initialized();
        assert_eq!(result.status, CheckStatus::Fail);
        assert_eq!(result.name, "jjz Initialized");
        assert!(result.message.contains("not initialized"));

        // Test 2: .jjz directory exists but no config.toml - should fail
        if fs::create_dir(".jjz").is_err() {
            let _ = std::env::set_current_dir(original_dir);
            return;
        }
        let result = check_initialized();
        assert_eq!(result.status, CheckStatus::Fail);

        // Test 3: .jjz directory with config.toml - should pass
        if fs::write(".jjz/config.toml", "workspace_dir = \"test\"").is_err() {
            let _ = std::env::set_current_dir(original_dir);
            return;
        }
        let result = check_initialized();
        assert_eq!(result.status, CheckStatus::Pass);
        assert!(result.message.contains(".jjz directory exists"));

        // Cleanup: restore original directory
        let _ = std::env::set_current_dir(original_dir);
    }

    #[test]
    fn test_check_initialized_independent_of_jj() {
        // This test verifies that check_initialized doesn't call jj commands
        // We test this by checking it works even without a JJ repo

        let temp_dir = TempDir::new().ok();
        let Some(temp_dir) = temp_dir else {
            return;
        };

        let original_dir = std::env::current_dir().ok();
        let Some(original_dir) = original_dir else {
            return;
        };
        if std::env::set_current_dir(temp_dir.path()).is_err() {
            return;
        }

        // Create .jjz structure WITHOUT initializing a JJ repo
        if fs::create_dir(".jjz").is_err() {
            let _ = std::env::set_current_dir(original_dir);
            return;
        }
        if fs::write(".jjz/config.toml", "workspace_dir = \"test\"").is_err() {
            let _ = std::env::set_current_dir(original_dir);
            return;
        }

        // Even without JJ installed/initialized, should detect .jjz
        // Verify files exist before checking
        let jjz_exists = std::path::Path::new(".jjz").exists();
        let config_exists = std::path::Path::new(".jjz/config.toml").exists();
        if !jjz_exists || !config_exists {
            // If we couldn't create the files, skip the test
            let _ = std::env::set_current_dir(original_dir);
            return;
        }
        let result = check_initialized();
        assert_eq!(result.status, CheckStatus::Pass);

        // Cleanup
        let _ = std::env::set_current_dir(original_dir);
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
        assert_eq!(check.name, "jjz Initialized");
        assert!(check.status == CheckStatus::Pass || check.status == CheckStatus::Fail);
        assert!(!check.message.is_empty());
    }
}
