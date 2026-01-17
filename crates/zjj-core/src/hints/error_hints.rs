//! Error-based hint generation
//!
//! Pure functions for generating helpful hints based on error codes,
//! providing context-aware solutions and next steps.

use crate::hints::{Hint, HintType};

// ═══════════════════════════════════════════════════════════════════════════
// ERROR CODE HINTS
// ═══════════════════════════════════════════════════════════════════════════

/// Extract session name from error message safely
///
/// Attempts to extract text between single quotes.
/// Returns the default session name if extraction fails.
#[must_use]
fn extract_session_name(error_msg: &str, default: &str) -> String {
    error_msg
        .split('\'')
        .nth(1)
        .map(|s| s.to_string())
        .unwrap_or_else(|| default.to_string())
}

/// Generate hints for SESSION_ALREADY_EXISTS error
#[must_use]
pub(crate) fn hints_for_session_exists(error_msg: &str) -> Vec<Hint> {
    let session_name = extract_session_name(error_msg, "session");
    vec![
        Hint::suggestion("Use a different name for the new session")
            .with_command(format!("jjz add {session_name}-v2"))
            .with_rationale("Append version or date to differentiate"),
        Hint::suggestion("Switch to the existing session")
            .with_command(format!("jjz focus {session_name}"))
            .with_rationale("Continue work in existing session"),
        Hint::suggestion("Remove the existing session first")
            .with_command(format!("jjz remove {session_name}"))
            .with_rationale("Clean up old session before creating new one"),
    ]
}

/// Generate hints for ZELLIJ_NOT_RUNNING error
#[must_use]
pub(crate) fn hints_for_zellij_not_running() -> Vec<Hint> {
    vec![
        Hint::suggestion("Start Zellij first")
            .with_command("zellij")
            .with_rationale("jjz requires Zellij to be running"),
        Hint::tip("You can attach to existing Zellij session")
            .with_command("zellij attach")
            .with_rationale("Reuse existing session instead of creating new one"),
    ]
}

/// Generate hints for NOT_INITIALIZED error
#[must_use]
pub(crate) fn hints_for_not_initialized() -> Vec<Hint> {
    vec![
        Hint::suggestion("Initialize jjz in this repository")
            .with_command("jjz init")
            .with_rationale("Creates .jjz directory with configuration"),
        Hint::tip("After init, you can configure jjz in .jjz/config.toml")
            .with_rationale("Customize workspace paths, hooks, and layouts"),
    ]
}

/// Generate hints for JJ_NOT_FOUND error
#[must_use]
pub(crate) fn hints_for_jj_not_found() -> Vec<Hint> {
    vec![
        Hint::warning("JJ (Jujutsu) is not installed or not in PATH")
            .with_rationale("jjz requires JJ for workspace management"),
        Hint::suggestion("Install JJ from https://github.com/martinvonz/jj")
            .with_rationale("Follow installation instructions for your platform"),
    ]
}

/// Generate hints for SESSION_NOT_FOUND error
#[must_use]
pub(crate) fn hints_for_session_not_found() -> Vec<Hint> {
    vec![
        Hint::suggestion("List all sessions to see available ones")
            .with_command("jjz list")
            .with_rationale("Check session names and status"),
        Hint::tip("Session names are case-sensitive")
            .with_rationale("Ensure exact match when referencing sessions"),
    ]
}

/// Generate generic error recovery hints
#[must_use]
pub(crate) fn hints_for_generic_error() -> Vec<Hint> {
    vec![Hint::info("Check error details for more information")
        .with_command("jjz status")
        .with_rationale("Verify system state and configuration")]
}

/// Generate hints for a specific error code
///
/// # Arguments
/// * `error_code` - The error classification code
/// * `error_msg` - The error message with context
///
/// Returns appropriate hints based on the error code,
/// providing next steps and recovery strategies.
#[must_use]
pub fn hints_for_error_code(error_code: &str, error_msg: &str) -> Vec<Hint> {
    match error_code {
        "SESSION_ALREADY_EXISTS" => hints_for_session_exists(error_msg),
        "ZELLIJ_NOT_RUNNING" => hints_for_zellij_not_running(),
        "NOT_INITIALIZED" => hints_for_not_initialized(),
        "JJ_NOT_FOUND" => hints_for_jj_not_found(),
        "SESSION_NOT_FOUND" => hints_for_session_not_found(),
        _ => hints_for_generic_error(),
    }
}

/// Generate hints for a specific error (public API)
///
/// Delegates to error_code matching and returns appropriate suggestions.
#[must_use]
pub fn hints_for_error(error_code: &str, error_msg: &str) -> Vec<Hint> {
    hints_for_error_code(error_code, error_msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_session_name_with_quotes() {
        let name = extract_session_name("Session 'test-name' already exists", "default");
        assert_eq!(name, "test-name");
    }

    #[test]
    fn test_extract_session_name_fallback() {
        let name = extract_session_name("No quotes here", "default");
        assert_eq!(name, "default");
    }

    #[test]
    fn test_hints_for_session_exists() {
        let hints = hints_for_session_exists("Session 'my-session' already exists");
        assert_eq!(hints.len(), 3);
        assert!(hints[0].message.contains("different name"));
        assert!(hints[1].message.contains("Switch"));
        assert!(hints[2].message.contains("Remove"));
    }

    #[test]
    fn test_hints_for_zellij_not_running() {
        let hints = hints_for_zellij_not_running();
        assert!(!hints.is_empty());
        assert!(hints[0].message.contains("Start Zellij"));
    }

    #[test]
    fn test_hints_for_not_initialized() {
        let hints = hints_for_not_initialized();
        assert!(!hints.is_empty());
        assert!(hints[0].message.contains("Initialize"));
    }

    #[test]
    fn test_hints_for_jj_not_found() {
        let hints = hints_for_jj_not_found();
        assert!(!hints.is_empty());
        assert!(hints[0].hint_type == HintType::Warning);
    }

    #[test]
    fn test_hints_for_session_not_found() {
        let hints = hints_for_session_not_found();
        assert!(!hints.is_empty());
        assert!(hints[0].message.contains("List all"));
    }

    #[test]
    fn test_hints_for_error_code_mapping() {
        let hints = hints_for_error_code("SESSION_ALREADY_EXISTS", "Session 'test' exists");
        assert!(!hints.is_empty());
    }

    #[test]
    fn test_hints_for_error_unknown_code() {
        let hints = hints_for_error_code("UNKNOWN_ERROR", "Unknown error");
        assert!(!hints.is_empty());
    }

    #[test]
    fn test_hints_for_error_public_api() {
        let hints = hints_for_error("NOT_INITIALIZED", "jjz not initialized");
        assert!(!hints.is_empty());
    }
}
