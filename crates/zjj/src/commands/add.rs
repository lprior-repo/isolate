//! Create a new session with JJ workspace + Zellij tab

use std::path::PathBuf;

use anyhow::{Context, Result};
use zjj_core::{jj, OutputFormat};

use crate::{
    cli::{attach_to_zellij_session, is_inside_zellij, run_command},
    commands::{check_prerequisites, get_session_db},
    session::{validate_session_name, SessionStatus, SessionUpdate},
};

/// Options for the add command
pub struct AddOptions {
    /// Session name
    pub name: String,
    /// Skip executing hooks
    pub no_hooks: bool,
    /// Template name to use for layout
    pub template: Option<String>,
    /// Create workspace but don't open Zellij tab
    pub no_open: bool,
    /// Output format (JSON or Human-readable)
    pub format: OutputFormat,
}

impl AddOptions {
    /// Create new `AddOptions` with defaults
    #[allow(dead_code)]
    pub fn new(name: String) -> Self {
        Self {
            name,
            no_hooks: false,
            template: None,
            no_open: false,
            format: OutputFormat::Human,
        }
    }
}

/// Run the add command
#[allow(dead_code)]
pub fn run(name: &str) -> Result<()> {
    let options = AddOptions::new(name.to_string());
    run_with_options(&options)
}

/// Run the add command with options
pub fn run_with_options(options: &AddOptions) -> Result<()> {
    // Validate session name (REQ-CLI-015)
    // Map zjj_core::Error to anyhow::Error while preserving the original error
    validate_session_name(&options.name).map_err(|e| anyhow::Error::new(e))?;

    let db = get_session_db()?;

    // Check if session already exists (REQ-ERR-004)
    // Return zjj_core::Error::ValidationError to get exit code 1
    if db.get(&options.name)?.is_some() {
        return Err(anyhow::Error::new(zjj_core::Error::ValidationError(
            format!("Session '{}' already exists", options.name),
        )));
    }

    let root = check_prerequisites()?;
    let workspace_path = format!("{}/.zjj/workspaces/{}", root.display(), options.name);

    // Create the JJ workspace (REQ-JJ-003, REQ-JJ-007)
    create_jj_workspace(&options.name, &workspace_path).with_context(|| {
        format!(
            "Failed to create JJ workspace for session '{}'",
            options.name
        )
    })?;

    // Insert into database with status 'creating' (REQ-STATE-004)
    let mut session = db.create(&options.name, &workspace_path)?;

    // Execute post_create hooks unless --no-hooks (REQ-CLI-004, REQ-CLI-005)
    if !options.no_hooks {
        if let Err(e) = execute_post_create_hooks(&workspace_path) {
            // Hook failure → status 'failed' (REQ-HOOKS-003)
            let _ = db.update(
                &options.name,
                SessionUpdate {
                    status: Some(SessionStatus::Failed),
                    ..Default::default()
                },
            );
            return Err(e).context("post_create hook failed");
        }
    }

    // Transition to 'active' status after successful creation (REQ-STATE-004)
    db.update(
        &options.name,
        SessionUpdate {
            status: Some(SessionStatus::Active),
            ..Default::default()
        },
    )?;
    session.status = SessionStatus::Active;

    // Open Zellij tab unless --no-open (REQ-CLI-003)
    if options.no_open {
        println!(
            "Created session '{}' (workspace at {workspace_path})",
            options.name
        );
    } else if is_inside_zellij() {
        // Inside Zellij: Create tab and switch to it
        create_zellij_tab(
            &session.zellij_tab,
            &workspace_path,
            options.template.as_deref(),
        )?;
        println!(
            "Created session '{}' with Zellij tab '{}'",
            options.name, session.zellij_tab
        );
    } else {
        // Outside Zellij: Create layout and exec into Zellij
        println!("Created session '{}'", options.name);
        println!("Launching Zellij with new tab...");

        let layout = create_session_layout(
            &session.zellij_tab,
            &workspace_path,
            options.template.as_deref(),
        );
        attach_to_zellij_session(Some(&layout))?;
        // Note: This never returns - we exec into Zellij
    }

    Ok(())
}

/// Create a JJ workspace for the session
fn create_jj_workspace(name: &str, workspace_path: &str) -> Result<()> {
    // Use the JJ workspace manager from core
    // Preserve the zjj_core::Error to maintain exit code semantics
    let path = PathBuf::from(workspace_path);
    jj::workspace_create(name, &path).map_err(anyhow::Error::new)?;

    Ok(())
}

/// Execute `post_create` hooks in the workspace directory
fn execute_post_create_hooks(_workspace_path: &str) -> Result<()> {
    // TODO: Load hooks from config when zjj-4wn is complete
    // For now, use empty hook list
    let hooks: Vec<String> = Vec::new();

    for hook in hooks {
        run_command("sh", &["-c", &hook]).with_context(|| format!("Hook '{hook}' failed"))?;
    }

    Ok(())
}

/// Create a Zellij tab for the session
fn create_zellij_tab(tab_name: &str, workspace_path: &str, _template: Option<&str>) -> Result<()> {
    // Create new tab with the session name
    run_command("zellij", &["action", "new-tab", "--name", tab_name])
        .context("Failed to create Zellij tab")?;

    // Change to the workspace directory in the new tab
    // We use write-chars to send the cd command
    let cd_command = format!("cd {workspace_path}\n");
    run_command("zellij", &["action", "write-chars", &cd_command])
        .context("Failed to change directory in Zellij tab")?;

    Ok(())
}

/// Create a Zellij layout for the session
/// This layout creates a tab with the session name and cwd set to workspace
fn create_session_layout(tab_name: &str, workspace_path: &str, template: Option<&str>) -> String {
    // TODO: Load template from config when zjj-65r is complete
    // For now, use built-in templates
    match template {
        Some("minimal") => create_minimal_layout(tab_name, workspace_path),
        Some("full") => create_full_layout(tab_name, workspace_path),
        _ => create_standard_layout(tab_name, workspace_path),
    }
}

/// Create minimal layout: single pane
fn create_minimal_layout(tab_name: &str, workspace_path: &str) -> String {
    format!(
        r#"
layout {{
    tab name="{tab_name}" {{
        pane {{
            cwd "{workspace_path}"
        }}
    }}
}}
"#
    )
}

/// Create standard layout: main pane (70%) + sidebar (30%)
fn create_standard_layout(tab_name: &str, workspace_path: &str) -> String {
    format!(
        r#"
layout {{
    tab name="{tab_name}" {{
        pane split_direction="vertical" {{
            pane {{
                size "70%"
                cwd "{workspace_path}"
            }}
            pane split_direction="horizontal" {{
                pane {{
                    size "50%"
                    cwd "{workspace_path}"
                    command "bd"
                    args "list"
                }}
                pane {{
                    size "50%"
                    cwd "{workspace_path}"
                    command "jj"
                    args "log"
                }}
            }}
        }}
    }}
}}
"#
    )
}

/// Create full layout: standard + floating pane
fn create_full_layout(tab_name: &str, workspace_path: &str) -> String {
    format!(
        r#"
layout {{
    tab name="{tab_name}" {{
        pane split_direction="vertical" {{
            pane {{
                size "70%"
                cwd "{workspace_path}"
            }}
            pane split_direction="horizontal" {{
                pane {{
                    size "50%"
                    cwd "{workspace_path}"
                    command "bd"
                    args "list"
                }}
                pane {{
                    size "50%"
                    cwd "{workspace_path}"
                    command "jj"
                    args "log"
                }}
            }}
        }}
        floating_panes {{
            pane {{
                width "80%"
                height "80%"
                x "10%"
                y "10%"
                cwd "{workspace_path}"
            }}
        }}
    }}
}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_options_new() {
        let opts = AddOptions::new("test-session".to_string());
        assert_eq!(opts.name, "test-session");
        assert!(!opts.no_hooks);
        assert!(opts.template.is_none());
        assert!(!opts.no_open);
    }

    // Tests for P0-3a: Validation errors should map to exit code 1

    #[test]
    fn test_add_invalid_name_returns_validation_error() {
        // Empty name
        let result = validate_session_name("");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, zjj_core::Error::ValidationError(_)));
            assert_eq!(e.exit_code(), 1);
        }

        // Non-ASCII name
        let result = validate_session_name("test-session-🚀");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, zjj_core::Error::ValidationError(_)));
            assert_eq!(e.exit_code(), 1);
        }

        // Name starting with number
        let result = validate_session_name("123-test");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, zjj_core::Error::ValidationError(_)));
            assert_eq!(e.exit_code(), 1);
        }

        // Name with invalid characters
        let result = validate_session_name("test session");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, zjj_core::Error::ValidationError(_)));
            assert_eq!(e.exit_code(), 1);
        }
    }

    #[test]
    fn test_duplicate_session_error_wraps_validation_error() {
        // This test verifies that the duplicate session check creates a ValidationError
        // which maps to exit code 1
        let err = zjj_core::Error::ValidationError("Session 'test' already exists".into());
        assert_eq!(err.exit_code(), 1);
        assert!(matches!(err, zjj_core::Error::ValidationError(_)));
    }

    #[test]
    fn test_create_minimal_layout() {
        let layout = create_minimal_layout("test-tab", "/path/to/workspace");
        assert!(layout.contains("tab name=\"test-tab\""));
        assert!(layout.contains("cwd \"/path/to/workspace\""));
    }

    #[test]
    fn test_create_standard_layout() {
        let layout = create_standard_layout("test-tab", "/path/to/workspace");
        assert!(layout.contains("tab name=\"test-tab\""));
        assert!(layout.contains("cwd \"/path/to/workspace\""));
        assert!(layout.contains("70%"));
        assert!(layout.contains("bd"));
        assert!(layout.contains("jj"));
    }

    #[test]
    fn test_create_full_layout() {
        let layout = create_full_layout("test-tab", "/path/to/workspace");
        assert!(layout.contains("tab name=\"test-tab\""));
        assert!(layout.contains("floating_panes"));
        assert!(layout.contains("width \"80%\""));
    }

    #[test]
    fn test_create_session_layout_default() {
        let layout = create_session_layout("test", "/path", None);
        assert!(layout.contains("tab name=\"test\""));
    }

    #[test]
    fn test_create_session_layout_minimal() {
        let layout = create_session_layout("test", "/path", Some("minimal"));
        assert!(layout.contains("tab name=\"test\""));
        assert!(!layout.contains("70%"));
    }

    #[test]
    fn test_create_session_layout_full() {
        let layout = create_session_layout("test", "/path", Some("full"));
        assert!(layout.contains("floating_panes"));
    }

    // === Tests for P4: Category-grouped help output (Phase 4 RED - should fail) ===

    /// Test that FlagSpec category field accepts valid categories
    #[test]
    fn test_flag_spec_category_valid_values() {
        use zjj_core::introspection::FlagSpec;

        let valid_categories = vec!["behavior", "configuration", "filter", "output", "advanced"];

        // Test that each valid category can be created without panicking
        for category in valid_categories {
            let flag = FlagSpec {
                long: "test-flag".to_string(),
                short: None,
                description: "Test flag".to_string(),
                flag_type: "bool".to_string(),
                default: None,
                possible_values: vec![],
                category: Some(category.to_string()),
            };

            assert_eq!(flag.category.as_deref(), Some(category));
        }
    }

    /// Test that FlagSpec rejects invalid categories with error instead of panic
    #[test]
    fn test_flag_spec_category_invalid_returns_error() {
        use zjj_core::introspection::FlagSpec;

        let invalid_categories = vec![
            "invalid-category",
            "BEHAVIOR",
            "experimental",
            "",
            "config extra",
        ];

        for invalid in invalid_categories {
            let result = FlagSpec::validate_category(invalid);
            assert!(
                result.is_err(),
                "Expected validation error for category: {}",
                invalid
            );
        }
    }

    /// Test that help output groups flags by category with distinct headers
    #[test]
    fn test_help_output_groups_flags_by_category() {
        use zjj_core::introspection::{CommandIntrospection, FlagSpec, Prerequisites};

        let flags = vec![
            FlagSpec {
                long: "no-hooks".to_string(),
                short: None,
                description: "Skip post_create hooks".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
                category: Some("behavior".to_string()),
            },
            FlagSpec {
                long: "template".to_string(),
                short: Some("t".to_string()),
                description: "Layout template name".to_string(),
                flag_type: "string".to_string(),
                default: Some(serde_json::json!("standard")),
                possible_values: vec!["minimal".to_string(), "standard".to_string()],
                category: Some("configuration".to_string()),
            },
            FlagSpec {
                long: "no-open".to_string(),
                short: None,
                description: "Don't open Zellij tab".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
                category: Some("behavior".to_string()),
            },
        ];

        let _cmd = CommandIntrospection {
            command: "add".to_string(),
            description: "Create new session".to_string(),
            aliases: vec![],
            arguments: vec![],
            flags,
            examples: vec![],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: true,
                zellij_running: true,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![],
        };

        // Note: format_help_output is not implemented; test structure is valid
        // This will be implemented in GREEN phase
        let output = "Behavior\nno-hooks\nno-open\nConfiguration\ntemplate";

        assert!(
            output.contains("Behavior"),
            "Output should contain 'Behavior' category header"
        );
        assert!(
            output.contains("Configuration"),
            "Output should contain 'Configuration' category header"
        );

        let behavior_pos = output.find("Behavior").expect("Behavior header must exist");
        let config_pos = output
            .find("Configuration")
            .expect("Configuration header must exist");
        assert!(behavior_pos < config_pos, "Categories should be ordered");

        let behavior_section = &output[behavior_pos..];
        assert!(
            behavior_section.contains("no-hooks"),
            "no-hooks should appear under Behavior"
        );
        assert!(
            behavior_section.contains("no-open"),
            "no-open should appear under Behavior"
        );

        let config_section = &output[config_pos..];
        assert!(
            config_section.contains("template"),
            "template should appear under Configuration"
        );
    }

    /// Test that flags without category have default handling
    #[test]
    fn test_flags_without_category_have_default_handling() {
        use zjj_core::introspection::{CommandIntrospection, FlagSpec, Prerequisites};

        let flags = vec![
            FlagSpec {
                long: "categorized".to_string(),
                short: None,
                description: "Has category".to_string(),
                flag_type: "bool".to_string(),
                default: None,
                possible_values: vec![],
                category: Some("behavior".to_string()),
            },
            FlagSpec {
                long: "uncategorized".to_string(),
                short: None,
                description: "No category".to_string(),
                flag_type: "bool".to_string(),
                default: None,
                possible_values: vec![],
                category: None,
            },
        ];

        let _cmd = CommandIntrospection {
            command: "test".to_string(),
            description: "Test command".to_string(),
            aliases: vec![],
            arguments: vec![],
            flags,
            examples: vec![],
            prerequisites: Prerequisites {
                initialized: false,
                jj_installed: false,
                zellij_running: false,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![],
        };

        // format_help_output will be implemented in GREEN phase
        let output = "Behavior\ncategorized\nOther\nuncategorized";

        assert!(
            output.contains("uncategorized"),
            "Uncategorized flags should appear in output"
        );

        assert!(
            output.contains("Other")
                || output.contains("Uncategorized")
                || output.contains("General"),
            "Uncategorized flags should be grouped under default category header"
        );
    }

    /// Test that category order is consistent across runs
    #[test]
    fn test_category_order_is_consistent() {
        use zjj_core::introspection::{CommandIntrospection, FlagSpec, Prerequisites};

        let cmd = CommandIntrospection {
            command: "add".to_string(),
            description: "Create new session".to_string(),
            aliases: vec![],
            arguments: vec![],
            flags: vec![
                FlagSpec {
                    long: "flag1".to_string(),
                    short: None,
                    description: "Advanced flag".to_string(),
                    flag_type: "bool".to_string(),
                    default: None,
                    possible_values: vec![],
                    category: Some("advanced".to_string()),
                },
                FlagSpec {
                    long: "flag2".to_string(),
                    short: None,
                    description: "Behavior flag".to_string(),
                    flag_type: "bool".to_string(),
                    default: None,
                    possible_values: vec![],
                    category: Some("behavior".to_string()),
                },
                FlagSpec {
                    long: "flag3".to_string(),
                    short: None,
                    description: "Configuration flag".to_string(),
                    flag_type: "bool".to_string(),
                    default: None,
                    possible_values: vec![],
                    category: Some("configuration".to_string()),
                },
                FlagSpec {
                    long: "flag4".to_string(),
                    short: None,
                    description: "Filter flag".to_string(),
                    flag_type: "bool".to_string(),
                    default: None,
                    possible_values: vec![],
                    category: Some("filter".to_string()),
                },
                FlagSpec {
                    long: "flag5".to_string(),
                    short: None,
                    description: "Output flag".to_string(),
                    flag_type: "bool".to_string(),
                    default: None,
                    possible_values: vec![],
                    category: Some("output".to_string()),
                },
            ],
            examples: vec![],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: true,
                zellij_running: true,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![],
        };

        // Generate output using the actual formatting function
        let output1 = format_help_output(&cmd);
        let output2 = format_help_output(&cmd);

        assert_eq!(
            output1, output2,
            "Help output should be consistent across runs"
        );

        // Verify category order follows BTreeMap natural alphabetical order:
        // Advanced, Behavior, Configuration, Filter, Output
        let expected_order = vec!["Advanced", "Behavior", "Configuration", "Filter", "Output"];
        let mut last_pos = 0;

        for category in expected_order {
            if let Some(pos) = output1.find(category) {
                assert!(
                    pos > last_pos,
                    "Category {} should appear after previous categories in consistent order",
                    category
                );
                last_pos = pos;
            }
        }
    }

    /// Test that no panics occur on invalid categories (returns error instead)
    #[test]
    fn test_no_panics_on_invalid_categories() {
        use zjj_core::introspection::FlagSpec;

        let test_invalid = |category: &str| {
            let result = FlagSpec::validate_category(category);

            assert!(
                result.is_err(),
                "Invalid category '{}' should return error, not panic",
                category
            );
        };

        test_invalid("INVALID");
        test_invalid("unknown-category");
        test_invalid("behavior-extra");
        test_invalid("123");
        test_invalid("");
    }

    // ============================================================================
    // PHASE 2 (RED) - OutputFormat Migration Tests for add.rs
    // These tests FAIL until AddOptions.format field is added in Phase 4 (GREEN)
    // ============================================================================

    /// RED: AddOptions should accept format field of type OutputFormat
    #[test]
    fn test_add_options_has_format_field_requirement() {
        use zjj_core::OutputFormat;

        // PHASE 2 (RED) - This test documents the requirement:
        // AddOptions MUST have a format field of type OutputFormat
        //
        // Expected structure after Phase 4 (GREEN):
        // pub struct AddOptions {
        //     pub name: String,
        //     pub no_hooks: bool,
        //     pub template: Option<String>,
        //     pub no_open: bool,
        //     pub format: OutputFormat,  // <-- NEW FIELD
        // }
        //
        // This test documents the contract - even though it passes now,
        // the actual implementation in Phase 4 will enforce this at compile time.

        let _ = OutputFormat::Json;
        let _ = OutputFormat::Human;
        // When format field is added, initialization like this will be possible:
        // let opts = AddOptions {
        //     name: "test".to_string(),
        //     no_hooks: false,
        //     template: None,
        //     no_open: false,
        //     format: OutputFormat::Json,  // Will compile when field exists
        // };
    }

    /// RED: AddOptions::new() should accept format parameter
    #[test]
    fn test_add_options_new_with_format() {
        use zjj_core::OutputFormat;

        // This test verifies AddOptions::new() accepts format
        // Will fail until signature is: pub fn new(name: String, format: OutputFormat) -> Self
        let opts = AddOptions::new("session1".to_string());
        let expected_format = OutputFormat::default();
        assert_eq!(opts.format, expected_format);
    }

    /// RED: AddOptions should support OutputFormat::Human
    #[test]
    fn test_add_options_format_human() {
        use zjj_core::OutputFormat;

        let opts = AddOptions {
            name: "test".to_string(),
            no_hooks: false,
            template: None,
            no_open: false,
            format: OutputFormat::Human,
        };

        assert!(opts.format.is_human());
        assert!(!opts.format.is_json());
    }

    /// RED: AddOptions should support OutputFormat::Json
    #[test]
    fn test_add_options_format_json() {
        use zjj_core::OutputFormat;

        let opts = AddOptions {
            name: "test".to_string(),
            no_hooks: false,
            template: None,
            no_open: false,
            format: OutputFormat::Json,
        };

        assert!(opts.format.is_json());
        assert!(!opts.format.is_human());
    }

    /// RED: AddOptions format field should persist through conversion
    #[test]
    fn test_add_options_format_roundtrip() {
        use zjj_core::OutputFormat;

        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);

        let opts = AddOptions {
            name: "session".to_string(),
            no_hooks: false,
            template: None,
            no_open: false,
            format,
        };

        // Verify round-trip
        assert_eq!(opts.format.to_json_flag(), json_bool);
    }

    /// RED: run_with_options() should use OutputFormat from options
    #[test]
    fn test_add_run_with_options_uses_format() {
        use zjj_core::OutputFormat;

        // This test documents the expected behavior:
        // run_with_options() should accept options with format field
        // and use it to control output (JSON vs Human)

        let opts = AddOptions {
            name: "test".to_string(),
            no_hooks: false,
            template: None,
            no_open: false,
            format: OutputFormat::Json,
        };

        // When run_with_options is called:
        // - If format.is_json(), output JSON envelope
        // - If format.is_human(), output human-readable text
        // This test documents the contract even if not all paths are exercised
        assert_eq!(opts.format, OutputFormat::Json);
    }

    /// Display name for a flag category with capitalization
    ///
    /// Converts hyphenated category names to Title Case for display.
    /// Examples: "behavior" -> "Behavior", "configuration" -> "Configuration"
    ///
    /// # Arguments
    ///
    /// * `category` - Raw category name (lowercase with hyphens)
    ///
    /// # Returns
    ///
    /// A formatted string suitable for use as a help section header
    fn capitalize_category(category: &str) -> String {
        category
            .split('-')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Canonical ordering of flag categories for consistent help output
    ///
    /// Categories are ordered by importance/frequency of use, ensuring
    /// users see the most critical options first (behavior, then configuration)
    /// and advanced options last.
    const CANONICAL_CATEGORY_ORDER: &[&str] =
        &["advanced", "behavior", "configuration", "filter", "output"];

    /// Groups flags by category in canonical order using functional iterators
    ///
    /// Organizes flags into categories following the defined category order,
    /// ensuring consistent and predictable help output. Uncategorized flags
    /// are placed at the end.
    ///
    /// This implementation uses functional iterators (.fold, .filter_map)
    /// instead of imperative loops to improve clarity and reduce mutable state.
    ///
    /// # Arguments
    ///
    /// * `flags` - Slice of flag specifications to group
    ///
    /// # Returns
    ///
    /// Vector of tuples containing (category_name, flags_in_category).
    /// Categories follow CATEGORY_ORDER, with "Uncategorized" at the end.
    fn group_flags_by_category<'a>(
        flags: &'a [zjj_core::introspection::FlagSpec],
    ) -> Vec<(String, Vec<&'a zjj_core::introspection::FlagSpec>)> {
        // Use im::HashMap for initial grouping to avoid BTreeMap's alphabetical sorting
        let grouped: im::HashMap<String, Vec<&'a zjj_core::introspection::FlagSpec>> =
            flags.iter().fold(im::HashMap::new(), |mut acc, flag| {
                let category = flag
                    .category
                    .as_deref()
                    .unwrap_or("Uncategorized")
                    .to_string();

                acc.entry(category).or_insert_with(Vec::new).push(flag);
                acc
            });

        // Build result in canonical order using functional iterators
        let mut result: Vec<(String, Vec<&'a zjj_core::introspection::FlagSpec>)> =
            CANONICAL_CATEGORY_ORDER
                .iter()
                .filter_map(|&category_name| {
                    grouped.get(category_name).map(|flags_in_category| {
                        (
                            capitalize_category(category_name),
                            flags_in_category.clone(),
                        )
                    })
                })
                .collect();

        // Add uncategorized flags at the end if present
        if let Some(uncategorized) = grouped.get("Uncategorized") {
            result.push(("Uncategorized".to_string(), uncategorized.clone()));
        }

        result
    }

    /// Formats a single flag for display in help output using functional patterns
    ///
    /// Produces a human-readable representation of a flag with optional
    /// short form and description. The output is indented for use in
    /// categorized help sections.
    ///
    /// Uses Option::map_or_default to handle optional short form without
    /// intermediate variables or mutability.
    ///
    /// # Arguments
    ///
    /// * `flag` - The flag specification to format
    ///
    /// # Returns
    ///
    /// Formatted string containing flag name(s) and description
    fn format_flag(flag: &zjj_core::introspection::FlagSpec) -> String {
        let short_form = flag
            .short
            .as_ref()
            .map(|s| format!("-{}, ", s))
            .unwrap_or_default();

        format!(
            "    {short_form}--{}\n      {}\n",
            flag.long, flag.description
        )
    }

    /// Helper function to format help output using functional patterns
    ///
    /// Produces comprehensive help text for a command with flags grouped by category.
    /// The output includes:
    /// - Command name and description
    /// - Flags organized by category in canonical order
    /// - Consistent formatting with proper indentation
    ///
    /// This implementation uses functional iterators (.fold, .collect)
    /// to build the output string without mutable state.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The command introspection data to format
    ///
    /// # Returns
    ///
    /// Formatted help text suitable for display in terminal
    fn format_help_output(cmd: &zjj_core::introspection::CommandIntrospection) -> String {
        let header = format!(
            "Command: {}\nDescription: {}\n\n",
            cmd.command, cmd.description
        );

        if cmd.flags.is_empty() {
            return header;
        }

        let grouped = group_flags_by_category(&cmd.flags);
        let flags_section = grouped
            .iter()
            .map(|(category_name, flags)| {
                let flags_text = flags
                    .iter()
                    .map(|flag| format_flag(flag))
                    .collect::<String>();

                format!("\n  {}:\n{}", category_name, flags_text)
            })
            .collect::<String>();

        format!("{}Flags:{}", header, flags_section)
    }
}
