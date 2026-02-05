use anyhow::{Context, Result};

use crate::cli::run_command;

/// Create a Zellij tab for the session
pub(super) fn create_zellij_tab(
    tab_name: &str,
    workspace_path: &str,
    _template: Option<&str>,
) -> Result<()> {
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
pub(super) fn create_session_layout(
    tab_name: &str,
    workspace_path: &str,
    template: Option<&str>,
) -> String {
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
                    command "br"
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
                    command "br"
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
    floating_panes {{
        pane {{
            x "10%"
            y "10%"
            width "80%"
            height "80%"
            cwd "{workspace_path}"
            command "nu"
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
        assert!(layout.contains("br"));
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
