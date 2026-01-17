//! Core data structures for JSON Schema generation.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ═══════════════════════════════════════════════════════════════════════════
// Constants
// ═══════════════════════════════════════════════════════════════════════════

/// JSON Schema draft version
pub const SCHEMA_VERSION: &str = "http://json-schema.org/draft-07/schema#";

/// Schema type name for objects
pub const TYPE_OBJECT: &str = "object";

/// Schema type name for strings
pub const TYPE_STRING: &str = "string";

/// Schema type name for booleans
pub const TYPE_BOOLEAN: &str = "boolean";

/// Schema type name for integers
pub const TYPE_INTEGER: &str = "integer";

/// Schema type name for arrays
pub const TYPE_ARRAY: &str = "array";

/// A JSON Schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    #[serde(rename = "$schema")]
    pub schema: String,

    #[serde(rename = "type")]
    pub type_name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<serde_json::Map<String, Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<bool>,
}

/// A property schema for an object field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySchema {
    #[serde(rename = "type")]
    pub type_name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<Self>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_schema_creation() {
        let schema = JsonSchema {
            schema: SCHEMA_VERSION.to_string(),
            type_name: TYPE_OBJECT.to_string(),
            title: Some("TestSchema".to_string()),
            description: None,
            properties: Some(serde_json::Map::new()),
            required: Some(Vec::new()),
            additional_properties: Some(false),
        };

        assert_eq!(schema.type_name, TYPE_OBJECT);
        assert_eq!(schema.title, Some("TestSchema".to_string()));
        assert!(schema.properties.is_some());
    }

    #[test]
    fn test_property_schema_string() {
        let prop = PropertySchema {
            type_name: TYPE_STRING.to_string(),
            description: Some("Test property".to_string()),
            required: Some(true),
            format: None,
            default: None,
            example: None,
            enum_values: None,
            items: None,
        };

        assert_eq!(prop.type_name, TYPE_STRING);
        assert_eq!(prop.description, Some("Test property".to_string()));
        assert_eq!(prop.required, Some(true));
    }

    #[test]
    fn test_schema_serialization() {
        let schema = JsonSchema {
            schema: SCHEMA_VERSION.to_string(),
            type_name: TYPE_OBJECT.to_string(),
            title: Some("TestSchema".to_string()),
            description: None,
            properties: Some(serde_json::Map::new()),
            required: Some(Vec::new()),
            additional_properties: Some(false),
        };

        let json = serde_json::to_string(&schema);
        assert!(json.is_ok());

        if let Ok(json_str) = json {
            assert!(json_str.contains("TestSchema"));
        }
    }
}
