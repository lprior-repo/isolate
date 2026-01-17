//! System-level health checks
//!
//! This module contains checks for system dependencies and tools.
//! Checks whether required external tools are installed.

use zjj_core::introspection::{CheckStatus, DoctorCheck};

use crate::cli::is_command_available;

/// Check if JJ (Jujutsu) is installed
pub fn check_jj_installed() -> DoctorCheck {
    let installed = is_command_available("jj");

    DoctorCheck {
        name: "JJ Installation".to_string(),
        status: if installed {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        message: if installed {
            "JJ is installed".to_string()
        } else {
            "JJ is not installed".to_string()
        },
        suggestion: if installed {
            None
        } else {
            Some("Install JJ: https://github.com/martinvonz/jj#installation".to_string())
        },
        auto_fixable: false,
        details: None,
    }
}

/// Check if Zellij is installed
pub fn check_zellij_installed() -> DoctorCheck {
    let installed = is_command_available("zellij");

    DoctorCheck {
        name: "Zellij Installation".to_string(),
        status: if installed {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        message: if installed {
            "Zellij is installed".to_string()
        } else {
            "Zellij is not installed".to_string()
        },
        suggestion: if installed {
            None
        } else {
            Some("Install Zellij: https://zellij.dev/documentation/installation".to_string())
        },
        auto_fixable: false,
        details: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_jj_installed_returns_valid_check() {
        let check = check_jj_installed();
        assert_eq!(check.name, "JJ Installation");
        assert!(check.status == CheckStatus::Pass || check.status == CheckStatus::Fail);
        assert!(!check.message.is_empty());
    }

    #[test]
    fn test_check_zellij_installed_returns_valid_check() {
        let check = check_zellij_installed();
        assert_eq!(check.name, "Zellij Installation");
        assert!(check.status == CheckStatus::Pass || check.status == CheckStatus::Fail);
        assert!(!check.message.is_empty());
    }
}
