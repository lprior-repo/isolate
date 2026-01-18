//! Output formatting and display
//!
//! This module handles formatting and displaying health check reports
//! in both human-readable and JSON formats.

use std::process;

use anyhow::Result;
use zjj_core::introspection::{CheckStatus, DoctorCheck, DoctorFixOutput, DoctorOutput};

/// Display health report
///
/// # Exit Codes
/// - 0: All checks passed (healthy system)
/// - 1: One or more checks failed (unhealthy system)
pub fn show_health_report(checks: &[DoctorCheck], json: bool) -> Result<()> {
    let output = DoctorOutput::from_checks(checks.to_vec());

    if json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        display_human_readable(&output);
    }

    // Return error if system is unhealthy (has failures)
    if !output.healthy {
        if json {
            // JSON already output, just exit with error code
            // Exit code 4: Invalid state (unhealthy system - database issues, missing dependencies,
            // etc.)
            process::exit(4);
        }
        anyhow::bail!("Health check failed: {} error(s) detected", output.errors);
    }

    Ok(())
}

/// Display fix results
///
/// # Exit Codes
/// - 0: All critical issues were fixed or none existed
/// - 1: Critical issues remain unfixed
pub fn show_fix_results(
    checks: &[DoctorCheck],
    output: &DoctorFixOutput,
    json: bool,
) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(output)?);
    } else {
        display_fix_human_readable(output);
    }

    // Count critical (Fail status) issues that couldn't be fixed
    let critical_unfixed = checks
        .iter()
        .filter(|c| {
            c.status == CheckStatus::Fail && !output.fixed.iter().any(|f| f.issue == c.name)
        })
        .count();

    if critical_unfixed > 0 {
        anyhow::bail!("Auto-fix completed but {critical_unfixed} critical issue(s) remain unfixed");
    }

    Ok(())
}

/// Display human-readable health report
fn display_human_readable(output: &DoctorOutput) {
    println!("zjj System Health Check");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    output.checks.iter().for_each(|check| {
        let symbol = match check.status {
            CheckStatus::Pass => "✓",
            CheckStatus::Warn => "⚠",
            CheckStatus::Fail => "✗",
        };

        println!("{symbol} {:<25} {}", check.name, check.message);

        if let Some(ref suggestion) = check.suggestion {
            println!("  → {suggestion}");
        }
    });

    println!();
    let passed = output
        .checks
        .len()
        .saturating_sub(output.warnings)
        .saturating_sub(output.errors);
    println!(
        "Health: {passed} passed, {} warning(s), {} error(s)",
        output.warnings, output.errors
    );

    if output.auto_fixable_issues > 0 {
        println!("Some issues can be auto-fixed: zjj doctor --fix");
    }

    // AI Agent guidance section
    if !output.ai_guidance.is_empty() {
        println!();
        println!("For AI Agents:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        output.ai_guidance.iter().for_each(|guidance| {
            println!("  • {guidance}");
        });
    }
}

/// Display human-readable fix results
fn display_fix_human_readable(output: &DoctorFixOutput) {
    if !output.fixed.is_empty() {
        println!("Fixed Issues:");
        output.fixed.iter().for_each(|fix| {
            let symbol = if fix.success { "✓" } else { "✗" };
            println!("{symbol} {}: {}", fix.issue, fix.action);
        });
        println!();
    }

    if !output.unable_to_fix.is_empty() {
        println!("Unable to Fix:");
        output.unable_to_fix.iter().for_each(|issue| {
            println!("✗ {}: {}", issue.issue, issue.reason);
            println!("  → {}", issue.suggestion);
        });
    }
}
