//! Configuration validation (Immutable functional pattern)
//!
//! This module contains validation logic for configuration values.
//! All operations return new instances rather than mutating in place.

use super::types::Config;
use crate::{Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// VALIDATION LOGIC (Immutable pattern)
// ═══════════════════════════════════════════════════════════════════════════

impl Config {
    /// Validate configuration values
    ///
    /// # Errors
    ///
    /// Returns error if any values are out of range or invalid
    pub fn validate(&self) -> Result<()> {
        // Validate main_branch is not empty if set
        if let Some(branch) = &self.main_branch {
            if branch.trim().is_empty() {
                return Err(Error::validation_error(
                    "main_branch cannot be empty - either unset it or provide a branch name"
                        .to_string(),
                ));
            }
        }

        // Validate debounce_ms range [10-5000]
        if self.watch.debounce_ms < 10 || self.watch.debounce_ms > 5000 {
            return Err(Error::validation_error(
                "debounce_ms must be 10-5000".to_string(),
            ));
        }

        // Validate refresh_ms range [100-10000]
        if self.dashboard.refresh_ms < 100 || self.dashboard.refresh_ms > 10000 {
            return Err(Error::validation_error(
                "refresh_ms must be 100-10000".to_string(),
            ));
        }

        Ok(())
    }

    /// Substitute placeholders like {repo} in config values - immutable pattern
    ///
    /// # Errors
    ///
    /// Returns error if unable to determine values for placeholders
    pub fn substitute_placeholders(mut self) -> Result<Self> {
        let repo_name = get_repo_name()?;
        self.workspace_dir = self.workspace_dir.replace("{repo}", &repo_name);
        Ok(self)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Get repository name from current directory
///
/// # Errors
///
/// Returns error if:
/// - Current directory cannot be determined
/// - Directory name cannot be extracted
fn get_repo_name() -> Result<String> {
    std::env::current_dir()
        .map_err(|e| Error::io_error(format!("Failed to get current directory: {e}")))
        .and_then(|dir| {
            dir.file_name()
                .and_then(|name| name.to_str())
                .map(String::from)
                .ok_or_else(|| Error::Unknown("Failed to determine repository name".to_string()))
        })
}
