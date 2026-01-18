//! JSON serialization traits and utilities.
//!
//! Provides a generic trait for types that can be serialized to pretty-printed JSON,
//! enabling consistent JSON output across the application.

use serde::Serialize;

/// Trait for types that can be serialized to pretty-printed JSON strings.
///
/// # Functional Pattern
/// Provides a uniform interface for JSON serialization without requiring
/// direct knowledge of the underlying serialization mechanism. The blanket
/// implementation ensures all `Serialize` types automatically gain this capability.
pub trait JsonSerializable: Serialize {
    /// Convert to pretty-printed JSON string.
    ///
    /// Uses indentation for human readability and returns a `Result`
    /// to enable error propagation via the `?` operator.
    fn to_json(&self) -> crate::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| crate::Error::parse_error(format!("Failed to serialize to JSON: {e}")))
    }
}

// Blanket implementation for all Serialize types
impl<T: Serialize> JsonSerializable for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn test_json_serializable_trait() -> crate::Result<()> {
        #[derive(Serialize)]
        struct TestStruct {
            field: String,
        }

        let test = TestStruct {
            field: "value".to_string(),
        };

        let json = test.to_json()?;
        assert!(json.contains("\"field\""));
        assert!(json.contains("\"value\""));

        Ok(())
    }

    #[test]
    fn test_json_success_wrapper() -> crate::Result<()> {
        use super::super::types::JsonSuccess;

        #[derive(Serialize, Deserialize)]
        struct TestData {
            name: String,
            count: usize,
        }

        let data = TestData {
            name: "test".to_string(),
            count: 42,
        };

        let success = JsonSuccess {
            success: true,
            data,
        };
        let json = success.to_json()?;

        assert!(json.contains("\"name\""));
        assert!(json.contains("\"test\""));
        assert!(json.contains("\"count\""));
        assert!(json.contains("42"));

        Ok(())
    }

    #[test]
    fn test_error_detail_skip_none() -> crate::Result<()> {
        use crate::json::JsonError;

        let err = JsonError::new("TEST", "message");
        let json = err.to_json()?;

        // Should not contain "details" or "suggestion" fields when they're None
        assert!(!json.contains("\"details\""));
        assert!(!json.contains("\"suggestion\""));

        Ok(())
    }

    #[test]
    fn test_json_serializable_with_numbers() -> crate::Result<()> {
        #[derive(Serialize)]
        struct Data {
            value: i32,
            float_val: f64,
            flag: bool,
        }

        let data = Data {
            value: 42,
            float_val: 2.72,
            flag: true,
        };

        let json = data.to_json()?;
        assert!(json.contains("42"));
        assert!(json.contains("2.72"));
        assert!(json.contains("true"));

        Ok(())
    }
}
