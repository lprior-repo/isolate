//! Health check implementations
//!
//! This module orchestrates all individual health checks that validate
//! the jjz system state. Checks are organized by category:
//! - System checks: External tool availability
//! - Environment checks: Runtime environment state
//! - Repository checks: JJ repository and database health
//! - JJZ setup checks: JJZ initialization and integration status

use super::env_checks;
use super::repo_checks;
use super::system_checks;
use super::zjj_setup_checks;

use im;
use zjj_core::introspection::DoctorCheck;

/// Run all health checks
///
/// Returns a vector of all health checks organized by category:
/// 1. System checks (JJ and Zellij installation)
/// 2. Environment checks (Zellij running status)
/// 3. Repository checks (JJ repo, state database, orphaned workspaces)
/// 4. JJZ setup checks (initialization, Beads integration)
pub async fn run_all() -> im::Vector<DoctorCheck> {
    im::vector![
        // System checks
        system_checks::check_jj_installed(),
        system_checks::check_zellij_installed(),
        // Environment checks
        env_checks::check_zellij_running(),
        // Repository checks
        repo_checks::check_jj_repo(),
        repo_checks::check_state_db().await,
        repo_checks::check_orphaned_workspaces().await,
        // JJZ setup checks
        zjj_setup_checks::check_initialized(),
        zjj_setup_checks::check_beads(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_all_returns_checks() {
        tokio_test::block_on(async {
            let checks = run_all().await;
            // Should have all 8 checks
            assert_eq!(checks.len(), 8);

            // Verify each check has a name
            for check in checks.iter() {
                assert!(!check.name.is_empty());
                assert!(!check.message.is_empty());
            }
        });
    }

    #[test]
    fn test_check_names_are_present() {
        tokio_test::block_on(async {
            let checks = run_all().await;
            let names: Vec<String> = checks.iter().map(|c| c.name.clone()).collect();

            // System checks
            assert!(names.contains(&"JJ Installation".to_string()));
            assert!(names.contains(&"Zellij Installation".to_string()));
            // Environment checks
            assert!(names.contains(&"Zellij Running".to_string()));
            // Repository checks
            assert!(names.contains(&"JJ Repository".to_string()));
            assert!(names.contains(&"State Database".to_string()));
            assert!(names.contains(&"Orphaned Workspaces".to_string()));
            // JJZ setup checks
            assert!(names.contains(&"jjz Initialized".to_string()));
            assert!(names.contains(&"Beads Integration".to_string()));
        });
    }
}
