//! JSON Schema serialization for contracts
//!
//! This module handles conversion of contract types to JSON Schema format,
//! providing machine-readable type information for validation and AI tooling.

use serde_json::{json, Value};

use crate::Result;

use super::types::{Constraint, FieldContract, TypeContract};

/// Serialization utilities for TypeContract
impl TypeContract {
    /// Convert contract to JSON Schema format
    pub fn to_json_schema(&self) -> Value {
        let mut schema = json!({
            "type": "object",
            "title": self.name,
            "description": self.description,
        });

        if !self.examples.is_empty() {
            schema["examples"] = json!(self.examples);
        }

        // Add field schemas using functional fold
        let (properties, required): (serde_json::Map<String, Value>, Vec<String>) =
            self.fields.iter().fold(
                (serde_json::Map::new(), Vec::new()),
                |(mut props, mut req), (field_name, field_contract)| {
                    props.insert(field_name.clone(), field_contract.to_json_schema());
                    if field_contract.required {
                        req.push(field_name.clone());
                    }
                    (props, req)
                },
            );

        if !properties.is_empty() {
            schema["properties"] = Value::Object(properties);
        }

        if !required.is_empty() {
            schema["required"] = json!(required);
        }

        schema
    }
}

/// Serialization utilities for FieldContract
impl FieldContract {
    /// Convert field contract to JSON Schema property
    pub fn to_json_schema(&self) -> Value {
        let mut schema = json!({
            "description": self.description,
        });

        // Add type information
        schema["type"] = match self.field_type.as_str() {
            "u32" | "u64" | "i32" | "i64" | "usize" => json!("integer"),
            "bool" => json!("boolean"),
            "Vec<String>" => json!("array"),
            _ => json!("string"), // "String" and unknown types default to string
        };

        // Functional fold: accumulate constraints into schema
        let mut schema = self
            .constraints
            .iter()
            .fold(schema, |mut schema, constraint| {
                match constraint {
                    Constraint::Regex { pattern, .. } => {
                        schema["pattern"] = json!(pattern);
                    }
                    Constraint::Range { min, max, .. } => {
                        if let Some(min_val) = min {
                            schema["minimum"] = json!(min_val);
                        }
                        if let Some(max_val) = max {
                            schema["maximum"] = json!(max_val);
                        }
                    }
                    Constraint::Length { min, max } => {
                        if let Some(min_len) = min {
                            schema["minLength"] = json!(min_len);
                        }
                        if let Some(max_len) = max {
                            schema["maxLength"] = json!(max_len);
                        }
                    }
                    Constraint::Enum { values } => {
                        schema["enum"] = json!(values);
                    }
                    _ => {}
                }
                schema
            });

        if let Some(default) = &self.default {
            schema["default"] = json!(default);
        }

        if !self.examples.is_empty() {
            schema["examples"] = json!(self.examples);
        }

        schema
    }
}

/// Validation logic for constraints
impl Constraint {
    /// Validate a string value against this constraint
    pub fn validate_string(&self, value: &str) -> Result<()> {
        match self {
            Self::Regex {
                pattern,
                description,
            } => {
                let re = regex::Regex::new(pattern).map_err(|e| {
                    crate::Error::validation_error(format!("Invalid regex pattern: {e}"))
                })?;

                if !re.is_match(value) {
                    return Err(crate::Error::validation_error(format!(
                        "Value '{value}' does not match pattern: {description}"
                    )));
                }
            }
            Self::Length { min, max } => {
                let len = value.len();
                if let Some(min_len) = min {
                    if len < *min_len {
                        return Err(crate::Error::validation_error(format!(
                            "Value length {len} is less than minimum {min_len}"
                        )));
                    }
                }
                if let Some(max_len) = max {
                    if len > *max_len {
                        return Err(crate::Error::validation_error(format!(
                            "Value length {len} exceeds maximum {max_len}"
                        )));
                    }
                }
            }
            Self::Enum { values } => {
                if !values.contains(&value.to_string()) {
                    return Err(crate::Error::validation_error(format!(
                        "Value '{value}' not in allowed values: {values:?}"
                    )));
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Validate a numeric value against this constraint
    pub fn validate_number(&self, value: i64) -> Result<()> {
        if let Self::Range {
            min,
            max,
            inclusive,
        } = self
        {
            if let Some(min_val) = min {
                if *inclusive {
                    if value < *min_val {
                        return Err(crate::Error::validation_error(format!(
                            "Value {value} is less than minimum {min_val}"
                        )));
                    }
                } else if value <= *min_val {
                    return Err(crate::Error::validation_error(format!(
                        "Value {value} must be greater than {min_val}"
                    )));
                }
            }
            if let Some(max_val) = max {
                if *inclusive {
                    if value > *max_val {
                        return Err(crate::Error::validation_error(format!(
                            "Value {value} exceeds maximum {max_val}"
                        )));
                    }
                } else if value >= *max_val {
                    return Err(crate::Error::validation_error(format!(
                        "Value {value} must be less than {max_val}"
                    )));
                }
            }
        }
        Ok(())
    }

    /// Validate a path against this constraint
    pub fn validate_path(&self, path: &std::path::Path) -> Result<()> {
        match self {
            Self::PathAbsolute => {
                if !path.is_absolute() {
                    return Err(crate::Error::validation_error(format!(
                        "Path '{}' must be absolute",
                        path.display()
                    )));
                }
            }
            Self::PathExists { must_be_absolute } => {
                if *must_be_absolute && !path.is_absolute() {
                    return Err(crate::Error::validation_error(format!(
                        "Path '{}' must be absolute",
                        path.display()
                    )));
                }
                if !path.exists() {
                    return Err(crate::Error::validation_error(format!(
                        "Path '{}' does not exist",
                        path.display()
                    )));
                }
            }
            _ => {}
        }
        Ok(())
    }
}
