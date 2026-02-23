//! Test Helper Module
//!
//! Provides ergonomic macros and utilities for better test failure messages
//! while maintaining test ergonomics.
//!
//! # Purpose
//!
//! Tests allow unwrap()/expect(), but we can improve error messages:
//! - Replace unwrap() with expect("descriptive message")
//! - Use match for better error context
//! - Helper macros for result testing
//!
//! # Usage
//!
//! ```rust
//! use test_helpers::{unwrap_ok, unwrap_err};
//!
//! // Better than .unwrap()
//! let value = unwrap_ok!(result, "failed to parse config");
//!
//! // Better than .unwrap_err()
//! let error = unwrap_err!(result, "expected error but got success");
//! ```

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

// ═══════════════════════════════════════════════════════════════════════════
// Macros for Better Test Error Messages
// ═══════════════════════════════════════════════════════════════════════════

/// Unwrap a Result<T>, panicking with a descriptive message on error.
///
/// This is better than `.unwrap()` because it provides context about what failed.
///
/// # Example
///
/// ```rust
/// use test_helpers::unwrap_ok;
///
/// let result: Result<i32, &str> = Ok(42);
/// let value = unwrap_ok!(result, "failed to parse config");
/// assert_eq!(value, 42);
/// ```
#[macro_export]
macro_rules! unwrap_ok {
    ($result:expr, $msg:expr) => {
        match $result {
            Ok(value) => value,
            Err(e) => {
                panic!(
                    "{}: {:?}\n  at {}:{}",
                    $msg,
                    e,
                    file!(),
                    line!()
                );
            }
        }
    };
    ($result:expr, $msg:expr, $($arg:tt)*) => {
        match $result {
            Ok(value) => value,
            Err(e) => {
                panic!(
                    "{} ({}): {:?}\n  at {}:{}",
                    $msg,
                    format!($($arg)*),
                    e,
                    file!(),
                    line!()
                );
            }
        }
    };
}

/// Unwrap a Result<E>, panicking with a descriptive message on success.
///
/// This is better than `.unwrap_err()` because it provides context.
///
/// # Example
///
/// ```rust
/// use test_helpers::unwrap_err;
///
/// let result: Result<i32, &str> = Err("error");
/// let error = unwrap_err!(result, "expected error but got success");
/// assert_eq!(error, "error");
/// ```
#[macro_export]
macro_rules! unwrap_err {
    ($result:expr, $msg:expr) => {
        match $result {
            Err(e) => e,
            Ok(v) => {
                panic!(
                    "{}: got {:?}\n  at {}:{}",
                    $msg,
                    v,
                    file!(),
                    line!()
                );
            }
        }
    };
    ($result:expr, $msg:expr, $($arg:tt)*) => {
        match $result {
            Err(e) => e,
            Ok(v) => {
                panic!(
                    "{} ({}): got {:?}\n  at {}:{}",
                    $msg,
                    format!($($arg)*),
                    v,
                    file!(),
                    line!()
                );
            }
        }
    };
}

/// Unwrap an Option<T>, panicking with a descriptive message on None.
///
/// This is better than `.unwrap()` or `.expect()` because it includes
/// file and line information automatically.
///
/// # Example
///
/// ```rust
/// use test_helpers::unwrap_some;
///
/// let value = Some(42);
/// let unwrapped = unwrap_some!(value, "value should be present");
/// assert_eq!(unwrapped, 42);
/// ```
#[macro_export]
macro_rules! unwrap_some {
    ($option:expr, $msg:expr) => {
        match $option {
            Some(value) => value,
            None => {
                panic!(
                    "{}: Option was None\n  at {}:{}",
                    $msg,
                    file!(),
                    line!()
                );
            }
        }
    };
    ($option:expr, $msg:expr, $($arg:tt)*) => {
        match $option {
            Some(value) => value,
            None => {
                panic!(
                    "{} ({}): Option was None\n  at {}:{}",
                    $msg,
                    format!($($arg)*),
                    file!(),
                    line!()
                );
            }
        }
    };
}

/// Assert that an Option is None, with a descriptive message.
///
/// # Example
///
/// ```rust
/// use test_helpers::assert_none;
///
/// let value: Option<i32> = None;
/// assert_none!(value, "value should be None");
/// ```
#[macro_export]
macro_rules! assert_none {
    ($option:expr, $msg:expr) => {
        match $option {
            None => {},
            Some(v) => {
                panic!(
                    "{}: got {:?}\n  at {}:{}",
                    $msg,
                    v,
                    file!(),
                    line!()
                );
            }
        }
    };
    ($option:expr, $msg:expr, $($arg:tt)*) => {
        match $option {
            None => {},
            Some(v) => {
                panic!(
                    "{} ({}): got {:?}\n  at {}:{}",
                    $msg,
                    format!($($arg)*),
                    v,
                    file!(),
                    line!()
                );
            }
        }
    };
}

/// Expect with context - like expect() but with file/line info.
///
/// # Example
///
/// ```rust
/// use test_helpers::expect_ctx;
///
/// fn get_value() -> Option<i32> {
///     Some(42)
/// }
///
/// let value = expect_ctx!(get_value(), "get_value should return Some");
/// assert_eq!(value, 42);
/// ```
#[macro_export]
macro_rules! expect_ctx {
    ($option:expr, $msg:expr) => {
        match $option {
            Some(v) => v,
            None => {
                panic!(
                    "Expected {} to be Some: {}\n  at {}:{}",
                    stringify!($option),
                    $msg,
                    file!(),
                    line!()
                );
            }
        }
    };
    ($option:expr, $msg:expr, $($arg:tt)*) => {
        match $option {
            Some(v) => v,
            None => {
                panic!(
                    "Expected {} to be Some ({}): {}\n  at {}:{}",
                    stringify!($option),
                    format!($($arg)*),
                    $msg,
                    file!(),
                    line!()
                );
            }
        }
    };
}

/// Helper for BDD test steps - unwrap a Result with step context.
///
/// This is specifically for Given/When/Then steps to provide
/// better error messages indicating which step failed.
///
/// # Example
///
/// ```rust
/// use test_helpers::step_ok;
///
/// // In a BDD test:
/// // GIVEN the database is initialized
/// step_ok!(initialize_database(), "GIVEN", "database initialization");
/// ```
#[macro_export]
macro_rules! step_ok {
    ($result:expr, $step_type:expr, $description:expr) => {
        match $result {
            Ok(value) => value,
            Err(e) => {
                panic!(
                    "STEP FAILED [{} {}]: {:?}\n  at {}:{}",
                    $step_type,
                    $description,
                    e,
                    file!(),
                    line!()
                );
            }
        }
    };
    ($result:expr, $step_type:expr, $description:expr, $($arg:tt)*) => {
        match $result {
            Ok(value) => value,
            Err(e) => {
                panic!(
                    "STEP FAILED [{} {}]: {:?} ({})\n  at {}:{}",
                    $step_type,
                    $description,
                    e,
                    format!($($arg)*),
                    file!(),
                    line!()
                );
            }
        }
    };
}

// ═══════════════════════════════════════════════════════════════════════════
// Helper Functions for Common Test Patterns
// ═══════════════════════════════════════════════════════════════════════════

/// Parse JSON from a string with better error messages.
///
/// # Example
///
/// ```rust
/// use test_helpers::parse_json;
/// use serde_json::Value;
///
/// let json_str = r#"{"key": "value"}"#;
/// let value: Value = parse_json(json_str, "failed to parse response");
/// ```
pub fn parse_json<T>(json: &str, context: &str) -> T
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(json).unwrap_or_else(|e| {
        panic!(
            "{}: Failed to parse JSON: {}\n  Input: {}\n  at {}",
            context,
            e,
            json,
            line!()
        )
    })
}

/// Assert that JSON output is valid and parseable.
///
/// # Example
///
/// ```rust
/// use test_helpers::assert_valid_json;
///
/// let output = r#"{"status": "ok"}"#;
/// assert_valid_json(output, "command output should be valid JSON");
/// ```
pub fn assert_valid_json(json: &str, context: &str) {
    serde_json::from_str::<serde_json::Value>(json).unwrap_or_else(|e| {
        panic!(
            "{}: Invalid JSON: {}\n  Output was: {}\n  at {}",
            context,
            e,
            json,
            line!()
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unwrap_ok_macro() {
        let result: Result<i32, &str> = Ok(42);
        let value = unwrap_ok!(result, "test should pass");
        assert_eq!(value, 42);
    }

    #[test]
    #[should_panic(expected = "test should fail")]
    fn test_unwrap_ok_macro_panic() {
        let result: Result<i32, &str> = Err("error");
        unwrap_ok!(result, "test should fail");
    }

    #[test]
    fn test_unwrap_err_macro() {
        let result: Result<i32, &str> = Err("error");
        let error = unwrap_err!(result, "expected error");
        assert_eq!(error, "error");
    }

    #[test]
    #[should_panic(expected = "expected success but got error")]
    fn test_unwrap_err_macro_panic() {
        let result: Result<i32, &str> = Ok(42);
        unwrap_err!(result, "expected success but got error");
    }

    #[test]
    fn test_unwrap_some_macro() {
        let value = Some(42);
        let unwrapped = unwrap_some!(value, "should be Some");
        assert_eq!(unwrapped, 42);
    }

    #[test]
    #[should_panic(expected = "should be None")]
    fn test_unwrap_some_macro_panic() {
        let value: Option<i32> = None;
        unwrap_some!(value, "should be None");
    }

    #[test]
    fn test_assert_none_macro() {
        let value: Option<i32> = None;
        assert_none!(value, "should be None");
    }

    #[test]
    #[should_panic(expected = "should be Some")]
    fn test_assert_none_macro_panic() {
        let value = Some(42);
        assert_none!(value, "should be Some");
    }

    #[test]
    fn test_parse_json_helper() {
        let json = r#"{"value": 42}"#;
        let value: serde_json::Value = parse_json(json, "test parse");
        assert_eq!(value["value"], 42);
    }

    #[test]
    #[should_panic(expected = "Failed to parse JSON")]
    fn test_parse_json_helper_panic() {
        let json = r#"{"invalid"#;
        parse_json::<serde_json::Value>(json, "test parse");
    }

    #[test]
    fn test_assert_valid_json_helper() {
        let json = r#"{"valid": "json"}"#;
        assert_valid_json(json, "test valid json");
    }

    #[test]
    #[should_panic(expected = "Invalid JSON")]
    fn test_assert_valid_json_helper_panic() {
        let json = r#"{"invalid"#;
        assert_valid_json(json, "test invalid json");
    }
}
