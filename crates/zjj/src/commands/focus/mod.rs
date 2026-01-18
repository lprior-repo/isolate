//! Focus command implementation
//!
//! Refactored focus command with extracted:
//! - `tab_switch`: Tab switching operations
//! - `validation`: Pre-focus validation
//! - `error_handler`: Error handling and `JSON` output

pub mod error_handler;
pub mod tab_switch;
pub mod validation;

use anyhow::Result;

use crate::json_output::FocusOutput;

use self::{error_handler::FocusError, tab_switch::TabSwitchResult};

/// Options for the focus command
#[derive(Debug, Clone, Default)]
pub struct FocusOptions {
    /// Output as `JSON`
    pub json: bool,
}

/// Run the focus command with options
///
/// Workflow:
/// 1. Validate session name
/// 2. Validate database and session existence
/// 3. Validate `TTY` environment
/// 4. Switch to tab (inside or outside `Zellij`)
/// 5. Return success or error
///
/// # Arguments
/// * `name` - Session name to focus
/// * `options` - Command options (json output flag)
///
/// # Returns
/// * `Ok(())` - Successfully switched or attached
/// * `Err(e)` - If any validation or operation fails
pub async fn run_with_options(name: &str, options: &FocusOptions) -> Result<()> {
    // Step 1: Validate session name
    if let Err(e) = validation::validate_session_name(name) {
        if options.json {
            error_handler::output_error_json_and_exit(FocusError::Validation, &e.to_string(), None);
        }
        return Err(e);
    }

    // Step 2: Validate database and session
    let (_db, zellij_tab) = match validation::validate_database_and_session(name).await {
        Ok((db, tab)) => (db, tab),
        Err(e) => {
            if options.json {
                let suggestion = Some("Use 'zjj list' to see available sessions".to_string());
                error_handler::output_error_json_and_exit(
                    FocusError::NotFound,
                    &e.to_string(),
                    suggestion,
                );
            }
            return Err(e);
        }
    };

    // Step 3: Validate TTY environment
    if let Err(e) = validation::validate_tty_environment() {
        if options.json {
            error_handler::output_error_json_and_exit(FocusError::System, &e.to_string(), None);
        }
        return Err(e);
    }

    // Step 4: Switch to tab
    let switch_result = tab_switch::switch_to_tab(&zellij_tab, name).await?;

    // Step 5: Output result
    if options.json {
        let output = FocusOutput {
            success: true,
            session_name: name.to_string(),
            tab: zellij_tab,
            switched: switch_result.did_switch(),
            error: None,
        };
        let json = serde_json::to_string(&output)?;
        println!("{json}");
    } else {
        match switch_result {
            TabSwitchResult::Switched => {
                println!("Switched to session '{name}'");
            }
            TabSwitchResult::Attached => {
                // Message already printed by switch_to_tab
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_options_default() {
        let options = FocusOptions::default();
        assert!(!options.json);
    }

    #[test]
    fn test_focus_options_json() {
        let options = FocusOptions { json: true };
        assert!(options.json);
    }
}
