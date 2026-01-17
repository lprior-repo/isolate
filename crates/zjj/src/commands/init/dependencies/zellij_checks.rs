//! Zellij version checking
//!
//! This module validates that Zellij is installed and meets the minimum version requirement.
//! All operations are pure and panic-free.

use anyhow::{bail, Context, Result};

use super::system_checks;

const MIN_ZELLIJ_VERSION: (u32, u32, u32) = (0, 35, 1);

/// Check if Zellij version meets minimum requirement (0.35.1)
///
/// # Errors
///
/// Returns error if:
/// - Cannot execute `zellij --version` command
/// - Command execution fails
/// - Version string cannot be parsed
/// - Version is below minimum requirement (0.35.1)
pub fn check_zellij_version() -> Result<()> {
    let output = std::process::Command::new("zellij")
        .arg("--version")
        .output()
        .context("Failed to get zellij version")?;

    if !output.status.success() {
        bail!("Failed to get zellij version");
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    // Parse version from output like "zellij 0.43.1"
    let version = system_checks::parse_version(&version_str, "zellij")
        .context("Could not parse zellij version")?;

    if version < MIN_ZELLIJ_VERSION {
        bail!(
            "zellij version {}.{}.{} is too old. Minimum required: {}.{}.{}\n\
             Required features: KDL layouts (v0.32.0+), go-to-tab-name action (v0.35.1+)\n\
             Update with: cargo install --locked zellij",
            version.0,
            version.1,
            version.2,
            MIN_ZELLIJ_VERSION.0,
            MIN_ZELLIJ_VERSION.1,
            MIN_ZELLIJ_VERSION.2
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_zellij_version_constant() {
        // Verify the minimum version constant is correct
        assert_eq!(MIN_ZELLIJ_VERSION, (0, 35, 1));
    }

    #[test]
    fn test_zellij_version_comparison_logic() {
        // Verify version comparison works as expected
        let current_version = (0, 43, 1);
        assert!(current_version >= MIN_ZELLIJ_VERSION);

        let old_version = (0, 35, 0);
        assert!(old_version < MIN_ZELLIJ_VERSION);

        let new_version = (0, 36, 0);
        assert!(new_version > MIN_ZELLIJ_VERSION);
    }
}
