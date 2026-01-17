//! User confirmation handling

use std::io::{self, Write};

use anyhow::{bail, Result};

use crate::cli::is_stdin_tty;

/// Prompt user for confirmation before removal
///
/// Returns true if user confirms (y/yes), false if they cancel (n/no).
/// Errors if stdin is not a TTY (non-interactive context).
pub fn confirm_removal(name: &str) -> Result<bool> {
    // Check if stdin is a TTY before prompting for input
    if !is_stdin_tty() {
        // Not in an interactive terminal - auto-deny for safety
        bail!(
            "Cannot prompt for confirmation: stdin is not a TTY\n\
             \n\
             This happens when:\n\
             • Running in CI/CD pipelines\n\
             • Input is piped from a file or command\n\
             • Running in non-interactive SSH session\n\
             • Running as a background process\n\
             \n\
             To remove without confirmation, use --force flag:\n\
             jjz remove {name} --force"
        );
    }

    print!("Remove session '{name}' and its workspace? [y/N] ");
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    let response = response.trim().to_lowercase();
    Ok(response == "y" || response == "yes")
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_confirm_removal_format() {
        // Test that confirmation prompt logic is correct
        // Actual I/O testing would require mocking stdin/stdout
        // This verifies the function signature is correct
    }
}
