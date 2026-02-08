#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]

use std::path::Path;

use anyhow::{anyhow, Result};
use zjj_core::zellij::{self, LayoutConfig, LayoutTemplate};

/// Validate template name and convert to `LayoutTemplate`
///
/// # Errors
///
/// Returns error if template name is invalid (not one of: minimal, standard, full, split, review)
fn parse_template(template: Option<&str>) -> Result<LayoutTemplate> {
    match template {
        Some("minimal") => Ok(LayoutTemplate::Minimal),
        Some("standard") => Ok(LayoutTemplate::Standard),
        Some("full") => Ok(LayoutTemplate::Full),
        Some("split") => Ok(LayoutTemplate::Split),
        Some("review") => Ok(LayoutTemplate::Review),
        None => Ok(LayoutTemplate::Standard),
        Some(invalid) => Err(anyhow!(
            "Invalid template name: '{invalid}'. Valid options: minimal, standard, full, split, review"
        )),
    }
}

/// Create a Zellij tab for the session
pub(super) async fn create_zellij_tab(
    tab_name: &str,
    workspace_path: &str,
    template: Option<&str>,
) -> Result<()> {
    let template_type = parse_template(template)?;

    let stripped_name = tab_name
        .strip_prefix("zjj:")
        .map_or_else(|| tab_name.to_string(), |s| s.to_string());

    let config = LayoutConfig::new(stripped_name, Path::new(workspace_path).to_path_buf());

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

/// Create a Zellij layout for the session (as a string)
///
/// # Errors
///
/// Returns error if template name is invalid
pub(super) fn create_session_layout(
    tab_name: &str,
    workspace_path: &str,
    template: Option<&str>,
) -> Result<String> {
    let template_type = parse_template(template)?;

    let stripped_name = tab_name
        .strip_prefix("zjj:")
        .map_or_else(|| tab_name.to_string(), |s| s.to_string());

    let config = LayoutConfig::new(stripped_name, Path::new(workspace_path).to_path_buf());

    zellij::generate_template_kdl(&config, template_type)
        .map_err(|e| anyhow!("Failed to generate layout KDL: {e}"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn test_create_session_layout_default() {
        let layout = create_session_layout("zjj:test", "/path", None).unwrap();
        assert!(layout.contains("layout"));
        assert!(layout.contains("pane"));
    }

    #[test]
    fn test_create_session_layout_minimal() {
        let layout = create_session_layout("zjj:test", "/path", Some("minimal")).unwrap();
        assert!(layout.contains("layout"));
        assert!(layout.contains("pane"));
    }

    #[test]
    fn test_create_session_layout_full() {
        let layout = create_session_layout("zjj:test", "/path", Some("full")).unwrap();
        assert!(layout.contains("layout"));
        assert!(layout.contains("floating_panes"));
    }

    #[test]
    fn test_create_session_layout_rejects_invalid_template() {
        // Invalid template names should be rejected with an error
        let result = create_session_layout("zjj:test", "/path", Some("nonexistent"));
        assert!(result.is_err(), "Invalid template should cause error");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Invalid template name"),
            "Error should mention invalid template: {err_msg}"
        );
        assert!(
            err_msg.contains("nonexistent"),
            "Error should mention the invalid template name: {err_msg}"
        );
    }

    #[test]
    fn test_parse_template_valid_names() {
        // All valid template names should parse successfully
        assert!(parse_template(Some("minimal")).is_ok());
        assert!(parse_template(Some("standard")).is_ok());
        assert!(parse_template(Some("full")).is_ok());
        assert!(parse_template(Some("split")).is_ok());
        assert!(parse_template(Some("review")).is_ok());
        assert!(parse_template(None).is_ok()); // Defaults to Standard
    }

    #[test]
    fn test_parse_template_invalid_names() {
        // Invalid template names should return errors
        assert!(parse_template(Some("nonexistent")).is_err());
        assert!(parse_template(Some("")).is_err());
        assert!(parse_template(Some("invalid-name")).is_err());
        assert!(parse_template(Some("Minimal")).is_err()); // Case-sensitive
    }

    #[test]
    fn test_create_session_layout_standard_explicit() {
        // Explicitly requesting "standard" should work
        let layout = create_session_layout("zjj:test", "/path", Some("standard")).unwrap();
        assert!(layout.contains("layout"));
    }

    #[tokio::test]
    async fn test_create_zellij_tab_rejects_invalid_template() {
        // Runtime: create_zellij_tab should return error for invalid templates
        let result = create_zellij_tab("zjj:test", "/path", Some("nonexistent")).await;
        assert!(result.is_err(), "Invalid template should cause error");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Invalid template name"),
            "Error should mention invalid template: {err_msg}"
        );
    }
}
