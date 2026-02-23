//! Simple test to verify bounded validation works

// Test that ActionVerb validation works
fn test_action_verb_validation() {
    // Known verbs should work
    assert!(ActionVerb::new("run").is_ok());
    assert!(ActionVerb::new("create").is_ok());
    assert!(ActionVerb::new("delete").is_ok());
    assert!(ActionVerb::new("rebase").is_ok());
    assert!(ActionVerb::new("switch-tab").is_ok());

    // Custom verbs should work if valid format
    assert!(ActionVerb::new("custom-verb").is_ok());
    assert!(ActionVerb::new("test").is_ok());

    // Invalid verbs should fail
    assert!(ActionVerb::new("").is_err());
    assert!(ActionVerb::new("Run").is_err()); // uppercase
    assert!(ActionVerb::new("run@verb").is_err()); // special chars
    assert!(ActionVerb::new("123").is_err()); // starts with number
}

// Test that ActionTarget validation works
fn test_action_target_validation() {
    // Valid targets should work
    assert!(ActionTarget::new("session-1").is_ok());
    assert!(ActionTarget::new("/path/to/workspace").is_ok());
    assert!(ActionTarget::new("target").is_ok());

    // Invalid targets should fail
    assert!(ActionTarget::new("").is_err());
    assert!(ActionTarget::new("   ").is_err()); // whitespace only

    // Too long should fail
    let too_long = "a".repeat(1001);
    assert!(ActionTarget::new(&too_long).is_err());
}

// Test that WarningCode validation works
fn test_warning_code_validation() {
    // Known codes should work
    assert!(WarningCode::new("CONFIG_NOT_FOUND").is_ok());
    assert!(WarningCode::new("MERGE_CONFLICT").is_ok());

    // Custom codes should work if valid format
    assert!(WarningCode::new("W001").is_ok());
    assert!(WarningCode::new("E123").is_ok());
    assert!(WarningCode::new("CUSTOM_CODE").is_ok());

    // Invalid codes should fail
    assert!(WarningCode::new("").is_err());
    assert!(WarningCode::new("123").is_err()); // must start with letter
    assert!(WarningCode::new("INVALID-CODE!").is_err()); // special chars
}

fn main() {
    println!("Testing ActionVerb validation...");
    test_action_verb_validation();
    println!("  ✓ ActionVerb validation works!");

    println!("Testing ActionTarget validation...");
    test_action_target_validation();
    println!("  ✓ ActionTarget validation works!");

    println!("Testing WarningCode validation...");
    test_warning_code_validation();
    println!("  ✓ WarningCode validation works!");

    println!("\n✅ All bounded validation tests passed!");
}

// Mock types for testing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionVerb {
    Run,
    Create,
    Delete,
    Rebase,
    SwitchTab,
    Custom(String),
}

impl ActionVerb {
    pub fn new(verb: &str) -> Result<Self, String> {
        if verb.trim().is_empty() {
            return Err("action verb cannot be empty".to_string());
        }

        let lower = verb.to_lowercase();
        if lower != verb {
            return Err(format!("action verb must be lowercase, got: {verb}"));
        }

        if !lower
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err("invalid characters".to_string());
        }

        if !lower.chars().next().map_or(false, |c| c.is_ascii_lowercase()) {
            return Err("must start with letter".to_string());
        }

        match lower.as_str() {
            "run" => Ok(Self::Run),
            "create" => Ok(Self::Create),
            "delete" => Ok(Self::Delete),
            "rebase" => Ok(Self::Rebase),
            "switch-tab" => Ok(Self::SwitchTab),
            custom => Ok(Self::Custom(custom.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionTarget(String);

impl ActionTarget {
    pub const MAX_LENGTH: usize = 1000;

    pub fn new(target: &str) -> Result<Self, String> {
        let trimmed = target.trim();
        if trimmed.is_empty() {
            return Err("action target cannot be empty".to_string());
        }

        if trimmed.len() > Self::MAX_LENGTH {
            return Err("action target too long".to_string());
        }

        Ok(Self(trimmed.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarningCode {
    ConfigNotFound,
    MergeConflict,
    Custom(String),
}

impl WarningCode {
    pub fn new(code: &str) -> Result<Self, String> {
        if code.is_empty() {
            return Err("warning code cannot be empty".to_string());
        }

        if !code.chars().next().map_or(false, |c| c.is_ascii_alphabetic()) {
            return Err("must start with letter".to_string());
        }

        if !code.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err("invalid characters".to_string());
        }

        match code {
            "CONFIG_NOT_FOUND" => Ok(Self::ConfigNotFound),
            "MERGE_CONFLICT" => Ok(Self::MergeConflict),
            custom => Ok(Self::Custom(custom.to_string())),
        }
    }
}
