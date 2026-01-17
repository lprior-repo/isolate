//! JJ (Jujutsu) version checking
//!
//! This module validates that JJ is installed and meets the minimum version requirement.
//! All operations are pure and panic-free.

use anyhow::{bail, Context, Result};

use super::system_checks;

const MIN_JJ_VERSION: (u32, u32, u32) = (0, 8, 0);

/// Check if JJ version meets minimum requirement (0.8.0)
///
/// # Errors
///
/// Returns error if:
/// - Cannot execute `jj --version` command
/// - Command execution fails
/// - Version string cannot be parsed
/// - Version is below minimum requirement (0.8.0)
pub fn check_jj_version() -> Result<()> {
    let output = std::process::Command::new("jj")
        .arg("--version")
        .output()
        .context("Failed to get jj version")?;

    if !output.status.success() {
        bail!("Failed to get jj version");
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    // Parse version from output like "jj 0.36.0-70fd8f7697fbc20a9329a6e2f790ef86a8e284d1"
    let version =
        system_checks::parse_version(&version_str, "jj").context("Could not parse jj version")?;

    if version < MIN_JJ_VERSION {
        bail!(
            "jj version {}.{}.{} is too old. Minimum required: {}.{}.{}\n\
             Required features: workspace add, workspace forget, workspace list, root\n\
             Update with: cargo install jj-cli",
            version.0,
            version.1,
            version.2,
            MIN_JJ_VERSION.0,
            MIN_JJ_VERSION.1,
            MIN_JJ_VERSION.2
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_jj_version_constant() {
        // Verify the minimum version constant is correct
        assert_eq!(MIN_JJ_VERSION, (0, 8, 0));
    }

    #[test]
    fn test_jj_version_comparison_logic() {
        // Verify version comparison works as expected
        let current_version = (0, 36, 0);
        assert!(current_version >= MIN_JJ_VERSION);

        let old_version = (0, 7, 9);
        assert!(old_version < MIN_JJ_VERSION);

        let new_version = (1, 0, 0);
        assert!(new_version > MIN_JJ_VERSION);
    }
}
