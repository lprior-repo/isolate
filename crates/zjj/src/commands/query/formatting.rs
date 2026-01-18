//! Result formatting and serialization for queries
//!
//! This module handles the formatting and output of query results to JSON,
//! using serde for serialization. All query outputs include schema metadata.

use anyhow::Result;
use serde::Serialize;
use zjj_core::json::{SchemaEnvelope, SchemaType};

/// Format and output a serializable result as JSON with schema metadata
///
/// Converts any serializable value to pretty-printed JSON and outputs to stdout.
/// Wraps the result with schema metadata for validation support.
pub fn output_json<T: Serialize>(result: &T) -> Result<()> {
    let envelope = SchemaEnvelope::new(SchemaType::Query, result);
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    Ok(())
}

/// Output with a specific schema type
///
/// Used for query responses that have dedicated schema definitions.
pub fn output_json_with_schema<T: Serialize>(result: &T, schema_type: SchemaType) -> Result<()> {
    let envelope = SchemaEnvelope::new(schema_type, result);
    println!("{}", serde_json::to_string_pretty(&envelope)?);
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
        assert!(json.is_some());
        if let Some(value) = json {
            assert_eq!(
                value.get("raw").and_then(|v| v.as_str()),
                Some("--status=active")
            );
        }
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
