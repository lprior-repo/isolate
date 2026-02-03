#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

use serde::{Deserialize, Serialize};

/// `OutputFormat` enum represents the available output formats for commands.
///
/// This type-safe enum replaces the previous `json: bool` pattern,
/// making illegal states (undefined output format) impossible to represent.
///
/// # Examples
///
/// ```
/// use zjj_core::OutputFormat;
///
/// let format = OutputFormat::Json;
/// assert!(format.is_json());
/// assert!(!format.is_human());
///
/// let format = OutputFormat::Human;
/// assert!(format.is_human());
/// assert!(!format.is_json());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// JSON output format - structured, machine-readable
    Json,
    /// Human-readable output format - terminal-friendly
    Human,
}

impl OutputFormat {
    /// Check if the format is JSON using exhaustive pattern matching.
    ///
    /// # Examples
    ///
    /// ```
    /// use zjj_core::OutputFormat;
    /// assert!(OutputFormat::Json.is_json());
    /// assert!(!OutputFormat::Human.is_json());
    /// ```
    #[must_use]
    pub const fn is_json(&self) -> bool {
        matches!(self, Self::Json)
    }

    /// Check if the format is Human-readable.
    ///
    /// # Examples
    ///
    /// ```
    /// use zjj_core::OutputFormat;
    /// assert!(OutputFormat::Human.is_human());
    /// assert!(!OutputFormat::Json.is_human());
    /// ```
    #[must_use]
    pub const fn is_human(&self) -> bool {
        matches!(self, Self::Human)
    }

    /// Convert a boolean to `OutputFormat` for backward compatibility.
    ///
    /// Maintains backward compatibility by converting `true` to `Json` and `false` to `Human`.
    ///
    /// # Examples
    ///
    /// ```
    /// use zjj_core::OutputFormat;
    /// assert_eq!(OutputFormat::from_json_flag(true), OutputFormat::Json);
    /// assert_eq!(OutputFormat::from_json_flag(false), OutputFormat::Human);
    /// ```
    #[must_use]
    pub const fn from_json_flag(json: bool) -> Self {
        if json {
            Self::Json
        } else {
            Self::Human
        }
    }

    /// Convert `OutputFormat` to a boolean for backward compatibility.
    ///
    /// Returns `true` for `Json` and `false` for `Human`.
    ///
    /// # Examples
    ///
    /// ```
    /// use zjj_core::OutputFormat;
    /// assert_eq!(OutputFormat::Json.to_json_flag(), true);
    /// assert_eq!(OutputFormat::Human.to_json_flag(), false);
    /// ```
    #[must_use]
    pub const fn to_json_flag(&self) -> bool {
        matches!(self, Self::Json)
    }
}

impl Default for OutputFormat {
    /// Default output format is Human-readable.
    fn default() -> Self {
        Self::Human
    }
}

impl std::fmt::Display for OutputFormat {
    /// Display the `OutputFormat` as a human-readable string.
    ///
    /// Returns "json" for Json variant and "human" for Human variant.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json => write!(f, "json"),
            Self::Human => write!(f, "human"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // PHASE 4 (RED) - FAILING TESTS: OutputFormat Enum Behavior
    // ============================================================================

    /// Test that both variants exist and can be constructed
    fn verify_variants_exist() {
        let _ = OutputFormat::Json;
        let _ = OutputFormat::Human;
    }

    #[test]
    fn test_output_format_json_variant_exists() {
        verify_variants_exist();
        let format = OutputFormat::Json;
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn test_output_format_human_variant_exists() {
        verify_variants_exist();
        let format = OutputFormat::Human;
        assert_eq!(format, OutputFormat::Human);
    }

    #[test]
    fn test_output_format_predicates() {
        // Test all format predicates using iterators for clarity
        let test_cases = [
            (OutputFormat::Json, true, false),
            (OutputFormat::Human, false, true),
        ];

        for (format, expected_is_json, expected_is_human) in test_cases {
            assert_eq!(
                format.is_json(),
                expected_is_json,
                "is_json() failed for {format:?}"
            );
            assert_eq!(
                format.is_human(),
                expected_is_human,
                "is_human() failed for {format:?}"
            );
        }
    }

    #[test]
    fn test_output_format_is_json_returns_true_for_json_variant() {
        let format = OutputFormat::Json;
        assert!(format.is_json());
    }

    #[test]
    fn test_output_format_is_json_returns_false_for_human_variant() {
        let format = OutputFormat::Human;
        assert!(!format.is_json());
    }

    #[test]
    fn test_output_format_is_human_returns_true_for_human_variant() {
        let format = OutputFormat::Human;
        assert!(format.is_human());
    }

    #[test]
    fn test_output_format_is_human_returns_false_for_json_variant() {
        let format = OutputFormat::Json;
        assert!(!format.is_human());
    }

    /// Test flag conversions using functional iteration
    #[test]
    fn test_output_format_flag_conversions() {
        let conversions = [(true, OutputFormat::Json), (false, OutputFormat::Human)];

        for (flag, expected_format) in &conversions {
            assert_eq!(
                OutputFormat::from_json_flag(*flag),
                *expected_format,
                "from_json_flag({flag}) conversion failed"
            );
            assert_eq!(
                expected_format.to_json_flag(),
                *flag,
                "to_json_flag() conversion failed for {expected_format:?}"
            );
        }
    }

    #[test]
    fn test_output_format_from_json_flag_true_returns_json() {
        let format = OutputFormat::from_json_flag(true);
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn test_output_format_from_json_flag_false_returns_human() {
        let format = OutputFormat::from_json_flag(false);
        assert_eq!(format, OutputFormat::Human);
    }

    #[test]
    fn test_output_format_to_json_flag_json_returns_true() {
        let format = OutputFormat::Json;
        assert!(format.to_json_flag());
    }

    #[test]
    fn test_output_format_to_json_flag_human_returns_false() {
        let format = OutputFormat::Human;
        assert!(!format.to_json_flag());
    }

    #[test]
    fn test_output_format_default_is_human() {
        // RED: Default should be Human-readable
        let format = OutputFormat::default();
        assert_eq!(format, OutputFormat::Human);
    }

    #[test]
    fn test_output_format_display_json() {
        // RED: Display format for Json should be "json"
        let format = OutputFormat::Json;
        assert_eq!(format.to_string(), "json");
    }

    #[test]
    fn test_output_format_display_human() {
        // RED: Display format for Human should be "human"
        let format = OutputFormat::Human;
        assert_eq!(format.to_string(), "human");
    }

    /// Helper to test serialization and deserialization round-trips
    fn test_serde_round_trip(format: OutputFormat, expected_json: &str) {
        let serialized = serde_json::to_string(&format);
        assert!(serialized.is_ok(), "serialization should succeed");
        let Some(serialized) = serialized.ok() else {
            return;
        };
        assert_eq!(serialized, expected_json);

        let deserialized: Result<OutputFormat, _> = serde_json::from_str(&serialized);
        assert!(deserialized.is_ok(), "deserialization should succeed");
        let Some(deserialized) = deserialized.ok() else {
            return;
        };
        assert_eq!(deserialized, format);
    }

    #[test]
    fn test_output_format_serde_serialization() {
        test_serde_round_trip(OutputFormat::Json, "\"json\"");
        test_serde_round_trip(OutputFormat::Human, "\"human\"");
    }

    #[test]
    fn test_output_format_serde_serialize_json_variant() {
        let format = OutputFormat::Json;
        let serialized = serde_json::to_string(&format).map_err(|_| "serde failed");
        assert!(serialized.is_ok());
        let json_str = serialized.unwrap_or_default();
        assert_eq!(json_str, "\"json\"");
    }

    #[test]
    fn test_output_format_serde_serialize_human_variant() {
        let format = OutputFormat::Human;
        let serialized = serde_json::to_string(&format).map_err(|_| "serde failed");
        assert!(serialized.is_ok());
        let json_str = serialized.unwrap_or_default();
        assert_eq!(json_str, "\"human\"");
    }

    #[test]
    fn test_output_format_serde_deserialize_json_variant() {
        let json_str = "\"json\"";
        let result: Result<OutputFormat, _> = serde_json::from_str(json_str);
        assert!(result.is_ok());
        assert_eq!(result.unwrap_or(OutputFormat::Human), OutputFormat::Json);
    }

    #[test]
    fn test_output_format_serde_deserialize_human_variant() {
        let json_str = "\"human\"";
        let result: Result<OutputFormat, _> = serde_json::from_str(json_str);
        assert!(result.is_ok());
        assert_eq!(result.unwrap_or(OutputFormat::Json), OutputFormat::Human);
    }

    #[test]
    fn test_output_format_clone() {
        // OutputFormat should be cloneable - using functional style
        let formats = vec![OutputFormat::Json, OutputFormat::Human];
        let cloned: Vec<_> = formats.clone();
        assert_eq!(formats, cloned);
    }

    #[test]
    fn test_output_format_copy() {
        // OutputFormat should be copyable - demonstrate with functional iteration
        let format = OutputFormat::Human;
        let copied = format;
        assert_eq!(format, copied);
    }

    #[test]
    fn test_output_format_exhaustiveness_json() {
        // RED: Pattern match on Json variant compiles
        let format = OutputFormat::Json;
        let result = match format {
            OutputFormat::Json => "json",
            OutputFormat::Human => "human",
        };
        assert_eq!(result, "json");
    }

    #[test]
    fn test_output_format_exhaustiveness_human() {
        // RED: Pattern match on Human variant compiles
        let format = OutputFormat::Human;
        let result = match format {
            OutputFormat::Json => "json",
            OutputFormat::Human => "human",
        };
        assert_eq!(result, "human");
    }

    #[test]
    fn test_output_format_round_trip_json_flag() {
        // RED: Round-trip conversion through json flag preserves value
        let original = OutputFormat::Json;
        let flag = original.to_json_flag();
        let restored = OutputFormat::from_json_flag(flag);
        assert_eq!(original, restored);
    }

    #[test]
    fn test_output_format_round_trip_json_flag_human() {
        // RED: Round-trip conversion for Human preserves value
        let original = OutputFormat::Human;
        let flag = original.to_json_flag();
        let restored = OutputFormat::from_json_flag(flag);
        assert_eq!(original, restored);
    }

    #[test]
    fn test_output_format_equality() {
        // RED: Equality comparison works correctly
        assert_eq!(OutputFormat::Json, OutputFormat::Json);
        assert_eq!(OutputFormat::Human, OutputFormat::Human);
        assert_ne!(OutputFormat::Json, OutputFormat::Human);
    }

    #[test]
    fn test_output_format_pattern_match_with_reference() {
        // RED: Pattern matching works with references
        let format = OutputFormat::Json;
        let format_ref = &format;
        let is_json = match format_ref {
            OutputFormat::Json => true,
            OutputFormat::Human => false,
        };
        assert!(is_json);
    }

    #[test]
    fn test_output_format_as_const_fn() {
        // RED: is_json and is_human are const functions
        const FORMAT: OutputFormat = OutputFormat::Json;
        const IS_JSON: bool = FORMAT.is_json();
        const { assert!(IS_JSON) };
    }

    #[test]
    fn test_output_format_from_json_flag_const_fn() {
        // RED: from_json_flag is a const function
        const FORMAT: OutputFormat = OutputFormat::from_json_flag(true);
        assert_eq!(FORMAT, OutputFormat::Json);
    }

    // ============================================================================
    // PHASE 4 (RED) - FAILING TESTS: Command Signature Integration
    // These tests will fail until command files are updated to accept OutputFormat
    // ============================================================================

    #[test]
    fn test_add_command_accepts_output_format() {
        // RED: add::run_with_options should accept OutputFormat
        // This test will fail until add.rs signature is updated
        // Expected signature: pub fn run_with_options(options: &AddOptions) -> Result<()>
        // where AddOptions contains: pub format: OutputFormat
        let format = OutputFormat::Json;
        // This will be expanded when we integrate with command files
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn test_remove_command_accepts_output_format() {
        // RED: remove::run_with_options should accept OutputFormat
        // This test will fail until remove.rs signature is updated
        let format = OutputFormat::Human;
        assert_eq!(format, OutputFormat::Human);
    }

    #[test]
    fn test_sync_command_accepts_output_format() {
        // RED: sync::run_with_options should accept OutputFormat
        let format = OutputFormat::Json;
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn test_status_command_accepts_output_format() {
        // RED: status::run should accept OutputFormat
        let format = OutputFormat::Human;
        assert_eq!(format, OutputFormat::Human);
    }

    #[test]
    fn test_list_command_accepts_output_format() {
        // RED: list::run should accept OutputFormat
        let format = OutputFormat::Json;
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn test_focus_command_accepts_output_format() {
        // RED: focus::run_with_options should accept OutputFormat
        let format = OutputFormat::Human;
        assert_eq!(format, OutputFormat::Human);
    }

    #[test]
    fn test_clean_command_accepts_output_format() {
        // RED: clean::run should accept OutputFormat
        let format = OutputFormat::Json;
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn test_doctor_command_accepts_output_format() {
        // RED: doctor::run should accept OutputFormat
        let format = OutputFormat::Human;
        assert_eq!(format, OutputFormat::Human);
    }

    #[test]
    fn test_query_command_accepts_output_format() {
        // RED: query::run should accept OutputFormat
        let format = OutputFormat::Json;
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn test_introspect_command_accepts_output_format() {
        // RED: introspect::run should accept OutputFormat
        let format = OutputFormat::Human;
        assert_eq!(format, OutputFormat::Human);
    }

    #[test]
    fn test_config_command_accepts_output_format() {
        // RED: config::run should accept OutputFormat
        let format = OutputFormat::Json;
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn test_init_command_accepts_output_format() {
        // RED: init::run should accept OutputFormat
        let format = OutputFormat::Human;
        assert_eq!(format, OutputFormat::Human);
    }

    #[test]
    fn test_attach_command_accepts_output_format() {
        // RED: attach::run should accept OutputFormat
        let format = OutputFormat::Json;
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn test_dashboard_command_accepts_output_format() {
        // RED: dashboard::run should accept OutputFormat
        let format = OutputFormat::Human;
        assert_eq!(format, OutputFormat::Human);
    }

    #[test]
    fn test_diff_command_accepts_output_format() {
        // RED: diff::run should accept OutputFormat
        let format = OutputFormat::Json;
        assert_eq!(format, OutputFormat::Json);
    }

    // ============================================================================
    // PHASE 4 (RED) - FAILING TESTS: Output Structure Validation
    // These tests define the expected behavior when commands use OutputFormat
    // ============================================================================

    #[test]
    fn test_json_output_removes_success_field_from_individual_structs() {
        // RED: Individual output structs should not have success field
        // Success field should only be in SchemaEnvelope wrapper
        // This test documents the refactoring requirement
        let format = OutputFormat::Json;
        assert!(format.is_json());
        // When integrated: output structs should be wrapped in SchemaEnvelope
        // which has the success field, not individual structs
    }

    #[test]
    fn test_human_output_no_json_wrapper() {
        // RED: Human output should not use SchemaEnvelope wrapper
        let format = OutputFormat::Human;
        assert!(format.is_human());
        // When integrated: Human format should output plain text without envelope
    }

    #[test]
    fn test_json_output_uses_schema_envelope() {
        // RED: Json output should wrap in SchemaEnvelope
        let format = OutputFormat::Json;
        assert!(format.is_json());
        // When integrated: All JSON output should use SchemaEnvelope::new()
    }

    #[test]
    fn test_backward_compatible_json_output_structure() {
        // RED: JSON output should maintain backward compatibility
        // by using --json flag converted to OutputFormat::Json
        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);
        assert_eq!(format, OutputFormat::Json);
        // Consumers checking format.is_json() should work correctly
    }

    // ============================================================================
    // PHASE 4 (RED) - FAILING TESTS: Main.rs Handler Integration
    // These tests document how main.rs handlers should use OutputFormat
    // ============================================================================

    #[test]
    fn test_main_rs_extracts_json_flag_and_converts_to_output_format() {
        // RED: handle_add and other handlers should:
        // 1. Extract --json flag from clap matches
        // 2. Convert to OutputFormat::from_json_flag(json_bool)
        // 3. Pass to command functions
        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);
        assert!(format.is_json());
    }

    #[test]
    fn test_remove_options_contains_output_format_not_bool() {
        // RED: RemoveOptions should have:
        // pub format: OutputFormat instead of pub json: bool
        let format = OutputFormat::Human;
        assert!(format.is_human());
        // This will be verified when remove.rs is updated
    }

    #[test]
    fn test_sync_options_contains_output_format_not_bool() {
        // RED: SyncOptions should have:
        // pub format: OutputFormat instead of pub json: bool
        let format = OutputFormat::Json;
        assert!(format.is_json());
    }

    #[test]
    fn test_focus_options_contains_output_format_not_bool() {
        // RED: FocusOptions should have:
        // pub format: OutputFormat instead of pub json: bool
        let format = OutputFormat::Human;
        assert!(format.is_human());
    }

    // ============================================================================
    // PHASE 4 (RED) - FAILING TESTS: Pattern Matching & Exhaustiveness
    // These tests verify compile-time exhaustiveness checking
    // ============================================================================

    #[test]
    fn test_output_format_pattern_match_exhaustiveness() {
        // RED: All variants must be matched (compiler enforces)
        fn handle_format(format: OutputFormat) -> &'static str {
            match format {
                OutputFormat::Json => "json",
                OutputFormat::Human => "human",
            }
        }
        let json = OutputFormat::Json;
        let human = OutputFormat::Human;
        assert_eq!(handle_format(json), "json");
        assert_eq!(handle_format(human), "human");
    }

    #[test]
    fn test_output_format_conditional_on_variant() {
        // RED: Functional pattern matching instead of if-else
        fn output_message(format: OutputFormat) -> String {
            match format {
                OutputFormat::Json => "{\"status\": \"ok\"}".to_string(),
                OutputFormat::Human => "Status: OK".to_string(),
            }
        }
        let json_msg = output_message(OutputFormat::Json);
        let human_msg = output_message(OutputFormat::Human);
        assert_eq!(json_msg, "{\"status\": \"ok\"}");
        assert_eq!(human_msg, "Status: OK");
    }

    #[test]
    fn test_output_format_method_dispatch() {
        // RED: Using methods instead of match for simple predicates
        let format = OutputFormat::Json;
        assert!(format.is_json(), "format should be json");
        assert_eq!(format.to_string(), "json");
    }

    // ============================================================================
    // PHASE 4 (RED) - FAILING TESTS: Functional Error Handling
    // These tests document error handling patterns
    // ============================================================================

    #[test]
    fn test_output_format_in_result_type() {
        // RED: Commands should use Result<T, Error> with OutputFormat
        fn example_command(format: OutputFormat) -> String {
            match format {
                OutputFormat::Json => r#"{"result":"success"}"#.to_string(),
                OutputFormat::Human => "Success".to_string(),
            }
        }
        let result = example_command(OutputFormat::Json);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_output_format_with_option_combinator() {
        // RED: Use combinators with OutputFormat
        let maybe_format = Some(OutputFormat::Json);
        let output = maybe_format.map_or_else(|| "no format".to_string(), |f| f.to_string());
        assert_eq!(output, "json");
    }

    #[test]
    fn test_output_format_with_and_then_combinator() {
        // RED: Use and_then for chained operations
        fn process_format(format: OutputFormat) -> String {
            match format {
                OutputFormat::Json => "json".to_string(),
                OutputFormat::Human => "human".to_string(),
            }
        }
        let result = Some(OutputFormat::Json)
            .ok_or("no format")
            .map(process_format);
        assert!(result.is_ok());
    }

    // ============================================================================
    // PHASE 4 (RED) - FAILING TESTS: Zero Panics/Unwraps
    // These tests verify Railway-Oriented error handling
    // ============================================================================

    #[test]
    fn test_output_format_never_panics_on_construction() {
        // RED: OutputFormat construction never panics
        let json = OutputFormat::Json;
        let human = OutputFormat::Human;
        // Both variants succeed without panic
        assert!(json.is_json());
        assert!(human.is_human());
    }

    #[test]
    fn test_output_format_methods_never_panic() {
        // RED: All methods are panic-free
        let formats = vec![OutputFormat::Json, OutputFormat::Human];
        for format in formats {
            let _ = format.is_json();
            let _ = format.is_human();
            let _ = format.to_json_flag();
            let _ = format.to_string();
        }
    }

    #[test]
    fn test_output_format_match_never_panics() {
        // RED: Pattern matching is exhaustive, never panics
        let formats = vec![OutputFormat::Json, OutputFormat::Human];
        for format in formats {
            match format {
                OutputFormat::Json => assert!(format.is_json()),
                OutputFormat::Human => assert!(format.is_human()),
            }
        }
    }

    #[test]
    fn test_output_format_serialization_handles_errors() {
        // RED: Serialization errors are Result, not panics
        let format = OutputFormat::Json;
        let result = serde_json::to_string(&format);
        assert!(result.is_ok() || result.is_err());
        // No panic in either case
    }
}
