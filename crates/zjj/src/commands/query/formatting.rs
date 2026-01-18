//! Result formatting and serialization for queries
//!
//! This module handles the formatting and output of query results to JSON,
//! using serde for serialization.

use anyhow::Result;
use serde::Serialize;

/// Format and output a serializable result as JSON
///
/// Converts any serializable value to pretty-printed JSON and outputs to stdout.
/// This is a pure function that handles the formatting side effect.
pub fn output_json<T: Serialize>(result: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(result)?);
    Ok(())
}

/// Create a JSON filter object from a filter string
pub fn create_filter_json(filter: Option<&str>) -> Option<serde_json::Value> {
    filter.map(|f| serde_json::json!({"raw": f}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct TestResult {
        success: bool,
        message: String,
    }

    #[test]
    fn test_create_filter_json_some() {
        let json = create_filter_json(Some("--status=active"));
        let Some(value) = json else {
            panic!("Expected create_filter_json to return Some");
        };
        assert_eq!(
            value.get("raw").and_then(|v| v.as_str()),
            Some("--status=active")
        );
    }

    #[test]
    fn test_create_filter_json_none() {
        let json = create_filter_json(None);
        assert!(json.is_none());
    }

    #[test]
    fn test_output_json() {
        let result = TestResult {
            success: true,
            message: "Test".to_string(),
        };
        // Just verify it doesn't error
        let output_result = output_json(&result);
        assert!(output_result.is_ok());
    }
}
