//! JSON schema versioning for API outputs.
//!
//! Provides consistent schema references across all JSON outputs to enable
//! validation and versioning of API responses.
//!
//! # Design
//!
//! Schema versioning follows a simple pattern:
//! - `$schema`: URL reference to the JSON schema (e.g., `https://zjj.dev/schemas/v1/list.json`)
//! - `_schema_version`: Semantic version string for the schema (e.g., "1.0")
//!
//! # Usage
//!
//! Wrap any serializable output with `SchemaEnvelope` to add schema metadata:
//!
//! ```ignore
//! let output = SchemaEnvelope::new(SchemaType::List, my_data);
//! println!("{}", serde_json::to_string_pretty(&output)?);
//! ```

use serde::{Deserialize, Serialize};

/// Current schema version for all outputs
pub const SCHEMA_VERSION: &str = "1.0";

/// Base URL for schema references
pub const SCHEMA_BASE_URL: &str = "https://zjj.dev/schemas/v1";

/// Schema types for different command outputs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaType {
    /// List command output
    List,
    /// Query command output (generic)
    Query,
    /// Introspect command output
    Introspect,
    /// Command introspection output
    CommandSpec,
    /// Session exists query
    SessionExists,
    /// Session count query
    SessionCount,
    /// Can-run query
    CanRun,
    /// Suggest-name query
    SuggestName,
    /// Beads query responses
    Beads,
    /// Beads summary query
    BeadsSummary,
    /// Agent list output
    AgentList,
    /// Status output
    Status,
    /// Doctor output
    Doctor,
    /// Error output
    Error,
}

impl SchemaType {
    /// Get the schema filename for this type
    #[must_use]
    pub const fn filename(self) -> &'static str {
        match self {
            Self::List => "list.json",
            Self::Query => "query.json",
            Self::Introspect => "introspect.json",
            Self::CommandSpec => "command-spec.json",
            Self::SessionExists => "session-exists.json",
            Self::SessionCount => "session-count.json",
            Self::CanRun => "can-run.json",
            Self::SuggestName => "suggest-name.json",
            Self::Beads => "beads.json",
            Self::BeadsSummary => "beads-summary.json",
            Self::AgentList => "agent-list.json",
            Self::Status => "status.json",
            Self::Doctor => "doctor.json",
            Self::Error => "error.json",
        }
    }

    /// Get the full schema URL for this type
    #[must_use]
    pub fn schema_url(self) -> String {
        format!("{}/{}", SCHEMA_BASE_URL, self.filename())
    }
}

/// Envelope wrapper that adds schema metadata to any serializable output.
///
/// This wrapper adds `$schema` and `_schema_version` fields to the JSON output
/// while flattening the inner data structure.
///
/// # Example Output
///
/// ```json
/// {
///   "$schema": "https://zjj.dev/schemas/v1/list.json",
///   "_schema_version": "1.0",
///   "count": 5,
///   "sessions": [...]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEnvelope<T> {
    /// JSON Schema URL reference
    #[serde(rename = "$schema")]
    pub schema: String,

    /// Schema version for this output format
    #[serde(rename = "_schema_version")]
    pub schema_version: String,

    /// The actual output data, flattened into the envelope
    #[serde(flatten)]
    pub data: T,
}

impl<T> SchemaEnvelope<T> {
    /// Create a new schema envelope with the specified schema type
    pub fn new(schema_type: SchemaType, data: T) -> Self {
        Self {
            schema: schema_type.schema_url(),
            schema_version: SCHEMA_VERSION.to_string(),
            data,
        }
    }

    /// Create a new schema envelope for list output
    pub fn list(data: T) -> Self {
        Self::new(SchemaType::List, data)
    }

    /// Create a new schema envelope for query output
    pub fn query(data: T) -> Self {
        Self::new(SchemaType::Query, data)
    }

    /// Create a new schema envelope for introspect output
    pub fn introspect(data: T) -> Self {
        Self::new(SchemaType::Introspect, data)
    }

    /// Create a new schema envelope for command spec output
    pub fn command_spec(data: T) -> Self {
        Self::new(SchemaType::CommandSpec, data)
    }

    /// Create a new schema envelope for error output
    pub fn error(data: T) -> Self {
        Self::new(SchemaType::Error, data)
    }
}

/// Helper trait for wrapping outputs with schema metadata
pub trait WithSchema: Sized + Serialize {
    /// Wrap this output with schema metadata
    fn with_schema(self, schema_type: SchemaType) -> SchemaEnvelope<Self> {
        SchemaEnvelope::new(schema_type, self)
    }
}

// Blanket implementation for all serializable types
impl<T: Serialize> WithSchema for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestData {
        count: usize,
        items: Vec<String>,
    }

    #[test]
    fn test_schema_type_filename() {
        assert_eq!(SchemaType::List.filename(), "list.json");
        assert_eq!(SchemaType::Query.filename(), "query.json");
        assert_eq!(SchemaType::Introspect.filename(), "introspect.json");
    }

    #[test]
    fn test_schema_type_url() {
        assert_eq!(
            SchemaType::List.schema_url(),
            "https://zjj.dev/schemas/v1/list.json"
        );
        assert_eq!(
            SchemaType::Introspect.schema_url(),
            "https://zjj.dev/schemas/v1/introspect.json"
        );
    }

    #[test]
    fn test_schema_envelope_creation() {
        let data = TestData {
            count: 3,
            items: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        };

        let envelope = SchemaEnvelope::new(SchemaType::List, data);

        assert_eq!(envelope.schema, "https://zjj.dev/schemas/v1/list.json");
        assert_eq!(envelope.schema_version, "1.0");
        assert_eq!(envelope.data.count, 3);
    }

    #[test]
    fn test_schema_envelope_serialization() -> crate::Result<()> {
        let data = TestData {
            count: 2,
            items: vec!["x".to_string(), "y".to_string()],
        };

        let envelope = SchemaEnvelope::list(data);
        let json = serde_json::to_string_pretty(&envelope)
            .map_err(|e| crate::Error::parse_error(e.to_string()))?;

        assert!(json.contains("\"$schema\":"));
        assert!(json.contains("https://zjj.dev/schemas/v1/list.json"));
        assert!(json.contains("\"_schema_version\":"));
        assert!(json.contains("\"1.0\""));
        assert!(json.contains("\"count\":"));
        assert!(json.contains("\"items\":"));

        Ok(())
    }

    #[test]
    fn test_with_schema_trait() {
        let data = TestData {
            count: 1,
            items: vec!["test".to_string()],
        };

        let envelope = data.with_schema(SchemaType::Query);

        assert_eq!(envelope.schema, "https://zjj.dev/schemas/v1/query.json");
    }

    #[test]
    fn test_schema_envelope_convenience_methods() {
        let data = TestData {
            count: 0,
            items: vec![],
        };

        let list_envelope = SchemaEnvelope::list(data.clone());
        assert!(list_envelope.schema.contains("list.json"));

        let query_envelope = SchemaEnvelope::query(data.clone());
        assert!(query_envelope.schema.contains("query.json"));

        let introspect_envelope = SchemaEnvelope::introspect(data);
        assert!(introspect_envelope.schema.contains("introspect.json"));
    }
}
