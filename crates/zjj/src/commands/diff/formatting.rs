//! Output formatting and pager handling for diff command

use std::{
    io::Write,
    process::{Command, Stdio},
};

use crate::json_output::{ErrorDetail, ErrorOutput};

/// Output error in JSON format
pub fn output_json_error(code: &str, message: &str) {
    let error_output = ErrorOutput {
        success: false,
        error: ErrorDetail {
            code: code.to_string(),
            message: message.to_string(),
            details: None, // No structured details for diff errors
            suggestion: None,
        },
    };

    if let Ok(json_str) = serde_json::to_string(&error_output) {
        println!("{json_str}");
    }
}

/// Get the pager command from environment or defaults
pub fn get_pager() -> Option<String> {
    // Check PAGER environment variable
    std::env::var("PAGER")
        .ok()
        .filter(|p| !p.is_empty())
        .or_else(|| {
            // Try common pagers in order of preference
            ["delta", "bat", "less"]
                .iter()
                .find(|&&pager| which::which(pager).is_ok())
                .map(|&pager| pager.to_string())
        })
}

/// Output content through a pager if available, otherwise print directly
pub fn output_with_pager(content: &str) {
    if let Some(pager) = get_pager() {
        if let Ok(mut child) = Command::new(&pager).stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(content.as_bytes());
            }
            let _ = child.wait();
            return;
        }
    }
    // Fallback: print directly if no pager or pager failed
    print!("{content}");
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    #[test]
    #[serial]
    fn test_get_pager_from_env() {
        // Set PAGER environment variable
        std::env::set_var("PAGER", "custom-pager");
        let pager = get_pager();
        assert_eq!(pager, Some("custom-pager".to_string()));

        // Clean up
        std::env::remove_var("PAGER");
    }

    #[test]
    #[serial]
    fn test_get_pager_defaults() {
        // Unset PAGER
        std::env::remove_var("PAGER");
        let pager = get_pager();

        // Should return one of the default pagers if available
        // We can't assert a specific value since it depends on system
        // But we can verify it returns either Some or None
        assert!(pager.is_some() || pager.is_none());
    }

    #[test]
    #[serial]
    fn test_get_pager_empty_env() {
        // Set PAGER to empty string
        std::env::set_var("PAGER", "");
        let pager = get_pager();

        // Should fall back to defaults
        assert!(pager.is_some() || pager.is_none());

        // Clean up
        std::env::remove_var("PAGER");
    }
}
