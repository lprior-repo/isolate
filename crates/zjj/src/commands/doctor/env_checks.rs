//! Environment-level health checks
//!
//! This module contains checks for the current runtime environment.
//! Checks whether the environment is properly configured for jjz.

use zjj_core::introspection::{CheckStatus, DoctorCheck};

use crate::cli::is_inside_zellij;

/// Check if currently running inside Zellij
pub fn check_zellij_running() -> DoctorCheck {
    let running = is_inside_zellij();

    DoctorCheck {
        name: "Zellij Running".to_string(),
        status: if running {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
        },
        message: if running {
            "Inside Zellij session".to_string()
        } else {
            "Not running inside Zellij".to_string()
        },
        suggestion: if running {
            None
        } else {
            Some("Start Zellij: zellij".to_string())
        },
        auto_fixable: false,
        details: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_zellij_running_returns_valid_check() {
        let check = check_zellij_running();
        assert_eq!(check.name, "Zellij Running");
        assert!(check.status == CheckStatus::Pass || check.status == CheckStatus::Warn);
        assert!(!check.message.is_empty());
    }
}
