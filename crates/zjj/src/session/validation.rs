//! Session validation functions

use zjj_core::{Error, Result};

use super::status::SessionStatus;

/// Reserved workspace names that cannot be used for sessions
///
/// These names are reserved by JJ or have special meaning:
/// - "default": The initial workspace name created by `jj init`
/// - "root": Often used to refer to the main/root workspace
const RESERVED_NAMES: &[&str] = &["default", "root"];

/// Validate a session name
///
/// Session names must:
/// - Not be empty or whitespace-only
/// - Not exceed 255 characters
/// - Only contain ASCII alphanumeric characters, dashes, underscores, and periods
/// - Start with a letter (a-z, A-Z)
/// - Not be a reserved name (default, root)
/// - Not contain path traversal sequences, shell metacharacters, or SQL special characters
///
/// This validation prevents:
/// - Path traversal attacks (../, ..\)
/// - Shell command injection ($, backtick, |, &, ;, etc.)
/// - SQL injection (', ", --, etc.)
/// - Control characters and newlines
/// - Conflicts with JJ reserved workspace names
pub fn validate_session_name(name: &str) -> Result<()> {
    // Trim whitespace for checking empty/whitespace-only names
    let trimmed = name.trim();

    if trimmed.is_empty() {
        return Err(Error::validation_error(
            "Session name cannot be empty or whitespace-only\n\nSession names must:\n  - Contain only letters, numbers, dash, underscore, period\n  - Be between 1 and 255 characters\n  - Not consist entirely of whitespace\n\nExamples:\n  - feature-123\n  - bug_fix_auth\n  - refactor.2024",
        ));
    }

    // Reject if original name has leading/trailing whitespace
    if trimmed != name {
        return Err(Error::validation_error(
            "Session name cannot have leading or trailing whitespace",
        ));
    }

    // Check maximum length (255 characters as per security spec)
    if name.len() > 255 {
        return Err(Error::validation_error(format!(
            "Session name too long (max 255 characters, got {})",
            name.len()
        )));
    }

    // Check for non-ASCII characters first (prevents unicode bypasses)
    if !name.is_ascii() {
        return Err(Error::validation_error(
            "Session name must contain only ASCII characters (a-z, A-Z, 0-9, -, _, .)",
        ));
    }

    // Collect invalid characters for detailed error message
    let invalid_chars: Vec<char> = name
        .chars()
        .filter(|c| !c.is_ascii_alphanumeric() && *c != '-' && *c != '_' && *c != '.')
        .collect();

    if !invalid_chars.is_empty() {
        let invalid_list: String = invalid_chars
            .iter()
            .map(|c| match c {
                ' ' => "' '".to_string(),
                '\t' => "'\\t'".to_string(),
                '\n' => "'\\n'".to_string(),
                '\r' => "'\\r'".to_string(),
                c if c.is_control() => format!("'\\x{:02x}'", *c as u8),
                _ => format!("'{c}'"),
            })
            .collect::<Vec<_>>()
            .join(", ");

        return Err(Error::validation_error(
            format!(
                "Invalid session name: contains invalid characters: {invalid_list}\n\
                 Session names can only contain letters, numbers, dash, underscore, and period (a-z, A-Z, 0-9, -, _, .)"
            ),
        ));
    }

    // Must start with a letter (not dash, underscore, period, or digit)
    if let Some(first) = name.chars().next() {
        if !first.is_ascii_alphabetic() {
            return Err(Error::validation_error(
                "Invalid session name: must start with a letter (a-z, A-Z)",
            ));
        }
    }

    // Check for reserved names (case-insensitive)
    validate_not_reserved(name)?;

    // Additional security checks: reject dangerous patterns
    validate_no_path_traversal(name)?;
    validate_no_dangerous_patterns(name)?;

    Ok(())
}

/// Validate that the session name is not a reserved name
///
/// Reserved names are checked case-insensitively to prevent bypasses
fn validate_not_reserved(name: &str) -> Result<()> {
    let name_lower = name.to_lowercase();
    for reserved in RESERVED_NAMES {
        if name_lower == *reserved {
            return Err(Error::validation_error(format!(
                "Session name '{reserved}' is reserved by JJ and cannot be used"
            )));
        }
    }
    Ok(())
}

/// Validate that the session name doesn't contain path traversal sequences
fn validate_no_path_traversal(name: &str) -> Result<()> {
    if name.contains("..") {
        return Err(Error::validation_error(
            "Session name cannot contain path traversal sequences (..)",
        ));
    }
    Ok(())
}

/// Validate that the session name doesn't contain dangerous shell or SQL patterns
fn validate_no_dangerous_patterns(name: &str) -> Result<()> {
    // Check for shell metacharacters (should already be caught by character validation)
    // This is a defense-in-depth check
    let dangerous_chars = [
        '$', '`', '|', '&', ';', '<', '>', '(', ')', '[', ']', '{', '}', '\\', '/', '\'', '"', '*',
        '?', '!', '#', '~', '@', '%', '^', '=', '+', ':', ',',
    ];

    for ch in dangerous_chars {
        if name.contains(ch) {
            return Err(Error::validation_error(format!(
                "Session name cannot contain dangerous character: '{ch}'"
            )));
        }
    }

    // Check for control characters and null bytes
    if name.chars().any(|c| c.is_control() || c == '\0') {
        return Err(Error::validation_error(
            "Session name cannot contain control characters or null bytes",
        ));
    }

    Ok(())
}

/// Validate a status transition
///
/// Enforces valid state transitions in the session lifecycle:
/// - Creating -> Active, Failed
/// - Active -> Paused, Completed, Failed
/// - Paused -> Active, Failed
/// - Failed -> Creating (retry)
/// - Completed -> Active (reopen)
#[allow(dead_code)]
pub fn validate_status_transition(from: SessionStatus, to: SessionStatus) -> Result<()> {
    use SessionStatus::{Active, Completed, Creating, Failed, Paused};

    let valid = matches!(
        (from, to),
        (Creating | Paused | Completed, Active)
            | (Creating | Active | Paused, Failed)
            | (Active, Paused | Completed)
            | (Failed, Creating) // Can retry failed session
    );

    if valid {
        Ok(())
    } else {
        Err(Error::validation_error(format!(
            "Invalid status transition from {from} to {to}"
        )))
    }
}
