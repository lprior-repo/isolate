//! Doctor command - system health checks and auto-fix
//!
//! This command checks the health of the jjz system and can
//! automatically fix common issues.
//!
//! # Architecture
//!
//! The doctor module is organized into focused submodules:
//! - `checks`: Individual health check implementations
//! - `fixes`: Auto-fix implementations for detected issues
//! - `output`: Report formatting and display
//!
//! # Exit Codes
//!
//! The doctor command follows standard Unix conventions for exit codes:
//!
//! - **Exit 0**: System is healthy (all checks passed), or all critical issues were successfully
//!   fixed
//! - **Exit 1**: System has errors (one or more checks failed), or critical issues remain after
//!   auto-fix
//!
//! Warnings (`CheckStatus::Warn`) do not cause non-zero exit codes - only failures
//! (`CheckStatus::Fail`) do.

mod checks;
mod env_checks;
mod fixes;
mod output;
mod repo_checks;
mod system_checks;
mod zjj_setup_checks;

use anyhow::Result;

/// Run health checks
///
/// # Errors
/// Returns error if:
/// - Health checks fail (when not in fix mode)
/// - Critical issues remain after auto-fix (when in fix mode)
pub async fn run(json: bool, fix: bool) -> Result<()> {
    let checks = checks::run_all().await;
    let check_vec = checks.iter().cloned().collect::<Vec<_>>();

    if fix {
        let (fixed, unable_to_fix) = fixes::run_all(&check_vec).await;
        let fix_output = fixes::create_output(fixed, unable_to_fix);
        output::show_fix_results(&check_vec, &fix_output, json)
    } else {
        output::show_health_report(&check_vec, json)
    }
}
