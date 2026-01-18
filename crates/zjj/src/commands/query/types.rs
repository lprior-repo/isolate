//! Query type metadata and definitions
//!
//! This module defines the query types available in the system and provides
//! metadata for help generation and query routing.

/// Metadata about a query type for help generation and documentation
#[derive(Debug, Clone, Copy)]
pub struct QueryTypeInfo {
    /// Name of the query type
    pub name: &'static str,
    /// Human-readable description
    pub description: &'static str,
    /// Whether this query requires an argument
    pub requires_arg: bool,
    /// Name of the argument (for help display)
    pub arg_name: &'static str,
    /// Example usage command
    pub usage_example: &'static str,
    /// Description of the return value
    pub returns_description: &'static str,
}

impl QueryTypeInfo {
    /// Get all available query types
    pub const fn all() -> &'static [Self] {
        &[
            Self {
                name: "session-exists",
                description: "Check if a session exists by name",
                requires_arg: true,
                arg_name: "session_name",
                usage_example: "jjz query session-exists my-session",
                returns_description: r#"{"exists": true, "session": {"name": "my-session", "status": "active"}}"#,
            },
            Self {
                name: "session-count",
                description: "Count total sessions or filter by status",
                requires_arg: false,
                arg_name: "--status=active",
                usage_example: "jjz query session-count --status=active",
                returns_description: r#"{"count": 5, "filter": {"raw": "--status=active"}}"#,
            },
            Self {
                name: "can-run",
                description: "Check if a command can run and show blockers",
                requires_arg: true,
                arg_name: "command_name",
                usage_example: "jjz query can-run add",
                returns_description: r#"{"can_run": true, "command": "add", "blockers": [], "prerequisites_met": 4, "prerequisites_total": 4}"#,
            },
            Self {
                name: "suggest-name",
                description: "Suggest next available name based on pattern",
                requires_arg: true,
                arg_name: "pattern",
                usage_example: r#"jjz query suggest-name "feature-{n}""#,
                returns_description: r#"{"pattern": "feature-{n}", "suggested": "feature-3", "next_available_n": 3, "existing_matches": ["feature-1", "feature-2"]}"#,
            },
        ]
    }

    /// Find query type info by name
    pub fn find(name: &str) -> Option<&'static Self> {
        Self::all().iter().find(|q| q.name == name)
    }

    /// Format error message for missing required argument
    pub fn format_error_message(&self) -> String {
        format!(
            "Error: '{}' query requires {} argument\n\n\
             Description:\n  {}\n\n\
             Usage:\n  {} <{}>\n\n\
             Example:\n  {}\n\n\
             Returns:\n  {}",
            self.name,
            if self.requires_arg {
                "a"
            } else {
                "an optional"
            },
            self.description,
            self.name,
            self.arg_name,
            self.usage_example,
            self.returns_description
        )
    }

    /// Generate help text listing all available queries
    pub fn list_all_queries() -> String {
        let query_list = Self::all().iter().fold(String::new(), |mut acc, query| {
            use std::fmt::Write;
            let _ = write!(
                &mut acc,
                "  {} - {}\n    Example: {}\n\n",
                query.name, query.description, query.usage_example
            );
            acc
        });

        format!(
            "Available query types:\n\n{query_list}For detailed help on a specific query type, try running it without arguments.\n"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_type_info_all_has_content() {
        let all = QueryTypeInfo::all();
        assert!(!all.is_empty());
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn test_query_type_info_find() {
        let Some(info) = QueryTypeInfo::find("session-exists") else {
            panic!("Expected QueryTypeInfo to be found for 'session-exists'");
        };
        assert_eq!(info.name, "session-exists");
    }

    #[test]
    fn test_query_type_info_find_not_found() {
        let info = QueryTypeInfo::find("nonexistent");
        assert!(info.is_none());
    }

    #[test]
    fn test_format_error_message() {
        let Some(info) = QueryTypeInfo::find("session-exists") else {
            panic!("Expected QueryTypeInfo to be found for 'session-exists'");
        };
        let msg = info.format_error_message();
        assert!(msg.contains("session-exists"));
        assert!(msg.contains("Description:"));
        assert!(msg.contains("Usage:"));
        assert!(msg.contains("Example:"));
    }

    #[test]
    fn test_list_all_queries() {
        let list = QueryTypeInfo::list_all_queries();
        assert!(list.contains("session-exists"));
        assert!(list.contains("session-count"));
        assert!(list.contains("can-run"));
        assert!(list.contains("suggest-name"));
    }
}
