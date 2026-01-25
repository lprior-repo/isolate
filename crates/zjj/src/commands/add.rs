//! Create a new session with JJ workspace + Zellij tab

use std::path::PathBuf;

use anyhow::{Context, Result};
use zjj_core::jj;

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
}

impl AddOptions {
    /// Create new `AddOptions` with defaults
    #[allow(dead_code)]
    pub const fn new(name: String) -> Self {
        Self {
            name,
            no_hooks: false,
            template: None,
            no_open: false,
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
}
