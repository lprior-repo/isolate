//! Builder methods for JSON Schema types.

use super::types::{
    JsonSchema, PropertySchema, SCHEMA_VERSION, TYPE_ARRAY, TYPE_BOOLEAN, TYPE_INTEGER,
    TYPE_OBJECT, TYPE_STRING,
};
use serde_json::Value;

impl JsonSchema {
    /// Create a new JSON schema for an object type
    pub fn object(title: impl Into<String>) -> Self {
        Self {
            schema: SCHEMA_VERSION.to_string(),
            type_name: TYPE_OBJECT.to_string(),
            title: Some(title.into()),
            description: None,
            properties: Some(serde_json::Map::new()),
            required: Some(Vec::new()),
            additional_properties: Some(false),
        }
    }

    /// Add a description to the schema
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a property to the schema
    #[must_use]
    pub fn with_property(mut self, name: impl Into<String>, property: &PropertySchema) -> Self {
        let name = name.into();
        let is_required = property.required.unwrap_or(false);

        if let Some(ref mut props) = self.properties {
            props.insert(
                name.clone(),
                serde_json::to_value(property).unwrap_or(Value::Null),
            );
        }

        // Add to required if property is required
        if is_required {
            if let Some(ref mut required) = self.required {
                required.push(name);
            }
        }

        self
    }
}

impl PropertySchema {
    /// Create a new property schema with the specified type
    fn new(type_name: &str) -> Self {
        Self {
            type_name: type_name.to_string(),
            description: None,
            required: None,
            format: None,
            default: None,
            example: None,
            enum_values: None,
            items: None,
        }
    }

    /// Create a string property
    pub fn string() -> Self {
        Self::new(TYPE_STRING)
    }

    /// Create a boolean property
    pub fn boolean() -> Self {
        Self::new(TYPE_BOOLEAN)
    }

    /// Create an integer property
    pub fn integer() -> Self {
        Self::new(TYPE_INTEGER)
    }

    /// Create an array property
    pub fn array(items: Self) -> Self {
        Self {
            type_name: TYPE_ARRAY.to_string(),
            description: None,
            required: None,
            format: None,
            default: None,
            example: None,
            enum_values: None,
            items: Some(Box::new(items)),
        }
    }

    /// Add a description
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Mark as required
    #[must_use]
    pub const fn required(mut self) -> Self {
        self.required = Some(true);
        self
    }

    /// Add a format hint (e.g., "date-time", "iso8601")
    #[must_use]
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Add an example value
    #[must_use]
    pub fn with_example(mut self, example: Value) -> Self {
        self.example = Some(example);
        self
    }

    /// Add enum values
    #[must_use]
    pub fn with_enum(mut self, values: Vec<String>) -> Self {
        self.enum_values = Some(values);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_schema_object_creation() {
        let schema = JsonSchema::object("TestSchema")
            .with_description("A test schema")
            .with_property(
                "name",
                &PropertySchema::string()
                    .required()
                    .with_description("Name field"),
            );

        assert_eq!(schema.type_name, "object");
        assert_eq!(schema.title, Some("TestSchema".to_string()));
        assert!(schema.properties.is_some());
        assert!(schema.required.is_some());
    }

    #[test]
    fn test_property_schema_string_builder() {
        let prop = PropertySchema::string()
            .with_description("Test property")
            .required();

        assert_eq!(prop.type_name, "string");
        assert_eq!(prop.description, Some("Test property".to_string()));
        assert_eq!(prop.required, Some(true));
    }

    #[test]
    fn test_property_schema_array() {
        let prop = PropertySchema::array(PropertySchema::string());
        assert_eq!(prop.type_name, "array");
        assert!(prop.items.is_some());
    }

    #[test]
    fn test_property_schema_enum() {
        let prop =
            PropertySchema::string().with_enum(vec!["active".to_string(), "paused".to_string()]);

        assert!(prop.enum_values.is_some());
        assert_eq!(prop.enum_values.as_ref().map(Vec::len), Some(2));
    }

    #[test]
    fn test_property_format() {
        let prop = PropertySchema::string().with_format("date-time");

        assert_eq!(prop.format, Some("date-time".to_string()));
    }

    #[test]
    fn test_property_example() {
        let prop = PropertySchema::string().with_example(serde_json::json!("example value"));

        assert!(prop.example.is_some());
    }
}
