//! Path and data validation for database operations

use std::path::Path;

use zjj_core::{Error, Result};

/// Validate database path preconditions
pub(crate) fn validate_database_path(path: &Path, allow_create: bool) -> Result<()> {
    let exists = path.exists();

    if !exists && !allow_create {
        return Err(Error::database_error(format!(
            "Database file does not exist: {}\n\nRun 'zjj init' to initialize ZJZ.",
            path.display()
        )));
    }

    if exists {
        validate_existing_file(path)?;
    }

    Ok(())
}

/// Validate existing database file is not empty
fn validate_existing_file(path: &Path) -> Result<()> {
    std::fs::metadata(path)
        .map_err(|e| Error::database_error(format!("Failed to read database metadata: {e}")))
        .and_then(|metadata| {
            if metadata.len() == 0 {
                Err(Error::database_error(format!(
                    "Database file is empty or corrupted: {}\n\nRun 'zjj init' to reinitialize.",
                    path.display()
                )))
            } else {
                Ok(())
            }
        })
}
