use std::path::Path;
use anyhow::Result;
use zjj_core::zellij::{self, LayoutConfig, LayoutTemplate};

/// Create a Zellij tab for the session
pub(super) async fn create_zellij_tab(
    tab_name: &str,
    workspace_path: &str,
    template: Option<&str>,
) -> Result<()> {
    let template_type = match template {
        Some("minimal") => LayoutTemplate::Minimal,
        Some("full") => LayoutTemplate::Full,
        Some("split") => LayoutTemplate::Split,
        Some("review") => LayoutTemplate::Review,
        _ => LayoutTemplate::Standard,
    };

    let config = LayoutConfig::new(
        tab_name.strip_prefix("zjj:").unwrap_or(tab_name).to_string(),
        Path::new(workspace_path).to_path_buf(),
    );

    // Use a temporary directory for the layout file
    let temp_dir = std::env::temp_dir();
    let layout = zellij::layout_generate(&config, template_type, &temp_dir)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to generate layout: {e}"))?;

    // Open the tab using the generated layout
    zellij::tab_open(&layout.file_path, tab_name)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to open Zellij tab: {e}"))?;

    // Cleanup the temporary layout file
    let _ = tokio::fs::remove_file(&layout.file_path).await;

    Ok(())
}

    Ok(())
}

/// Create a Zellij layout for the session (as a string)
pub(super) fn create_session_layout(
    tab_name: &str,
    workspace_path: &str,
    template: Option<&str>,
) -> String {
    let template_type = match template {
        Some("minimal") => LayoutTemplate::Minimal,
        Some("full") => LayoutTemplate::Full,
        Some("split") => LayoutTemplate::Split,
        Some("review") => LayoutTemplate::Review,
        _ => LayoutTemplate::Standard,
    };

    let config = LayoutConfig::new(
        tab_name.strip_prefix("zjj:").unwrap_or(tab_name).to_string(),
        Path::new(workspace_path).to_path_buf(),
    );

    zellij::generate_template_kdl(&config, template_type)
        .unwrap_or_else(|_| "layout { pane { } }".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session_layout_default() {
        let layout = create_session_layout("zjj:test", "/path", None);
        assert!(layout.contains("layout"));
        assert!(layout.contains("pane"));
    }

    #[test]
    fn test_create_session_layout_minimal() {
        let layout = create_session_layout("zjj:test", "/path", Some("minimal"));
        assert!(layout.contains("layout"));
        assert!(layout.contains("pane"));
    }

    #[test]
    fn test_create_session_layout_full() {
        let layout = create_session_layout("zjj:test", "/path", Some("full"));
        assert!(layout.contains("layout"));
        assert!(layout.contains("floating_panes"));
    }
}
