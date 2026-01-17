//! Configuration command types and structures

/// Options for the config command
#[derive(Debug, Clone)]
pub struct ConfigOptions {
    pub key: Option<String>,
    pub value: Option<String>,
    pub global: bool,
    pub json: bool,
    pub validate: bool,
}

/// Represents a single validation issue (error or warning)
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationIssue {
    pub field: String,
    pub issue: String,
    pub suggestion: Option<String>,
}

/// Result of configuration validation
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub issues: im::Vector<ValidationIssue>,
    pub warnings: im::Vector<ValidationIssue>,
}
